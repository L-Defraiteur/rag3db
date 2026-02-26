#include "function/create_sparse_vector_index.h"

#include "binder/expression/literal_expression.h"
#include "catalog/sparse_vector_catalog_entry.h"
#include "common/exception/binder.h"
#include "common/string_format.h"
#include "common/types/value/nested.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/simple_table_function.h"
#include "index/sparse_vector_index.h"
#include "main/client_context.h"
#include "processor/execution_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

#include <filesystem>

namespace rag3db {
namespace sparse_vector_extension {

using namespace common;
using namespace main;
using namespace function;

// ── BindData ────────────────────────────────────────────────────────────────

struct CreateSparseVectorBindData final : TableFuncBindData {
    std::string tableName;
    table_id_t tableID;
    std::string indexName;
    std::string indicesColumnName;
    std::string weightsColumnName;
    property_id_t indicesPropertyID;
    property_id_t weightsPropertyID;

    CreateSparseVectorBindData(std::string tableName, table_id_t tableID,
        std::string indexName, std::string indicesCol, std::string weightsCol,
        property_id_t indicesPropID, property_id_t weightsPropID)
        : TableFuncBindData{binder::expression_vector{}, 0},
          tableName{std::move(tableName)}, tableID{tableID},
          indexName{std::move(indexName)},
          indicesColumnName{std::move(indicesCol)},
          weightsColumnName{std::move(weightsCol)},
          indicesPropertyID{indicesPropID}, weightsPropertyID{weightsPropID} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<CreateSparseVectorBindData>(*this);
    }
};

// ── Bind helpers ────────────────────────────────────────────────────────────

static void validateListColumn(const catalog::TableCatalogEntry* tableEntry,
    const std::string& tableName, const std::string& colName,
    const std::string& expectedChildType) {
    if (!tableEntry->containsProperty(colName)) {
        throw BinderException{
            stringFormat("Column '{}' does not exist in table '{}'.", colName, tableName)};
    }
    auto propID = tableEntry->getPropertyID(colName);
    auto& propType = tableEntry->getProperty(propID).getType();
    if (propType.getLogicalTypeID() != LogicalTypeID::LIST) {
        throw BinderException{stringFormat(
            "Column '{}' must be a LIST type, got {}.", colName, propType.toString())};
    }
    auto& childType = ListType::getChildType(propType);
    if (expectedChildType == "INT64" && childType.getLogicalTypeID() != LogicalTypeID::INT64) {
        throw BinderException{stringFormat(
            "Column '{}' must be LIST[INT64], got LIST[{}].", colName, childType.toString())};
    }
    if (expectedChildType == "DOUBLE" && childType.getLogicalTypeID() != LogicalTypeID::DOUBLE) {
        throw BinderException{stringFormat(
            "Column '{}' must be LIST[DOUBLE], got LIST[{}].", colName, childType.toString())};
    }
}

// ── Bind (public: tableName, indicesCol, weightsCol) ────────────────────────

static std::unique_ptr<TableFuncBindData> bindFunc(ClientContext* context,
    const TableFuncBindInput* input) {
    auto tableName = input->getLiteralVal<std::string>(0);
    auto indicesCol = input->getLiteralVal<std::string>(1);
    auto weightsCol = input->getLiteralVal<std::string>(2);

    auto catalog = catalog::Catalog::Get(*context);
    auto transaction = transaction::Transaction::Get(*context);
    auto tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    if (!tableEntry) {
        throw BinderException{stringFormat("Table '{}' does not exist.", tableName)};
    }

    validateListColumn(tableEntry, tableName, indicesCol, "INT64");
    validateListColumn(tableEntry, tableName, weightsCol, "DOUBLE");

    auto indexName = tableName;
    return std::make_unique<CreateSparseVectorBindData>(tableName, tableEntry->getTableID(),
        indexName, indicesCol, weightsCol,
        tableEntry->getPropertyID(indicesCol), tableEntry->getPropertyID(weightsCol));
}

// ── Bind (internal: tableName, indexName, indicesCol, weightsCol) ───────────

static std::unique_ptr<TableFuncBindData> bindFuncInternal(ClientContext* context,
    const TableFuncBindInput* input) {
    auto tableName = input->getLiteralVal<std::string>(0);
    auto indexName = input->getLiteralVal<std::string>(1);
    auto indicesCol = input->getLiteralVal<std::string>(2);
    auto weightsCol = input->getLiteralVal<std::string>(3);

    auto catalog = catalog::Catalog::Get(*context);
    auto transaction = transaction::Transaction::Get(*context);
    auto tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    if (!tableEntry) {
        throw BinderException{stringFormat("Table '{}' does not exist.", tableName)};
    }

    validateListColumn(tableEntry, tableName, indicesCol, "INT64");
    validateListColumn(tableEntry, tableName, weightsCol, "DOUBLE");

    return std::make_unique<CreateSparseVectorBindData>(tableName, tableEntry->getTableID(),
        indexName, indicesCol, weightsCol,
        tableEntry->getPropertyID(indicesCol), tableEntry->getPropertyID(weightsCol));
}

// ── rewriteFunc ─────────────────────────────────────────────────────────────

static std::string rewriteFunc(ClientContext& context, const TableFuncBindData& bindData) {
    context.setUseInternalCatalogEntry(true);
    auto& bd = *bindData.constPtrCast<CreateSparseVectorBindData>();
    auto query = stringFormat(
        "CALL _CREATE_SPARSE_VECTOR_INDEX('{}', '{}', '{}', '{}');",
        bd.tableName, bd.indexName, bd.indicesColumnName, bd.weightsColumnName);
    query += stringFormat(
        "RETURN 'Sparse vector index created on table {}.' AS result;", bd.tableName);
    return query;
}

// ── Internal _CREATE_SPARSE_VECTOR_INDEX ────────────────────────────────────

static offset_t tableFunc(const TableFuncInput& input, TableFuncOutput&) {
    auto& bd = *input.bindData->constPtrCast<CreateSparseVectorBindData>();
    auto& context = *input.context;
    auto transaction = transaction::Transaction::Get(*context.clientContext);
    auto catalog = catalog::Catalog::Get(*context.clientContext);
    auto* storageManager = storage::StorageManager::Get(*context.clientContext);
    auto& nodeTable =
        storageManager->getTable(bd.tableID)->cast<storage::NodeTable>();

    // 1. Build index path.
    std::string basePath;
    if (context.clientContext->isInMemory()) {
        basePath = std::filesystem::temp_directory_path().string() + "/rag3db_sparse_vector";
    } else {
        basePath = std::filesystem::path(
            context.clientContext->getDatabasePath()).parent_path().string();
    }
    auto indexPath = stringFormat("{}/sparse_vector_indexes/{}/", basePath, bd.tableName);
    std::filesystem::create_directories(indexPath);

    // 2. Create sparse index via cxx bridge.
    auto handle = create_sparse_index(indexPath);

    // 3. Scan all existing rows and index them.
    auto numRows = nodeTable.getNumTotalRows(transaction);

    if (numRows > 0) {
        auto tableEntry = catalog->getTableCatalogEntry(transaction, bd.tableID);

        std::vector<column_id_t> scanColumnIDs;
        scanColumnIDs.push_back(tableEntry->getColumnID(bd.indicesPropertyID));
        scanColumnIDs.push_back(tableEntry->getColumnID(bd.weightsPropertyID));

        auto dataChunk = DataChunkState::getSingleValueDataChunkState();
        ValueVector idVector{LogicalType::INTERNAL_ID(),
            storage::MemoryManager::Get(*context.clientContext), dataChunk};

        std::vector<std::unique_ptr<ValueVector>> outputVecs;
        std::vector<ValueVector*> outputPtrs;
        outputVecs.push_back(std::make_unique<ValueVector>(
            LogicalType::LIST(LogicalType::INT64()),
            storage::MemoryManager::Get(*context.clientContext), dataChunk));
        outputVecs.push_back(std::make_unique<ValueVector>(
            LogicalType::LIST(LogicalType::DOUBLE()),
            storage::MemoryManager::Get(*context.clientContext), dataChunk));
        outputPtrs.push_back(outputVecs[0].get());
        outputPtrs.push_back(outputVecs[1].get());

        storage::NodeTableScanState scanState{&idVector, outputPtrs, dataChunk};
        scanState.setToTable(transaction, &nodeTable, scanColumnIDs);

        internalID_t nodeID = {INVALID_OFFSET, bd.tableID};
        for (offset_t offset = 0; offset < numRows; offset++) {
            nodeID.offset = offset;
            idVector.setValue(0, nodeID);
            nodeTable.initScanState(transaction, scanState, nodeID.tableID, nodeID.offset);
            nodeTable.lookup(transaction, scanState);

            if (outputPtrs[0]->isNull(0) || outputPtrs[1]->isNull(0)) {
                continue;
            }

            auto indicesList = outputPtrs[0]->getValue<list_entry_t>(0);
            auto* indicesData = ListVector::getDataVector(outputPtrs[0]);
            std::vector<uint32_t> indices;
            for (auto j = indicesList.offset; j < indicesList.offset + indicesList.size; j++) {
                indices.push_back(static_cast<uint32_t>(indicesData->getValue<int64_t>(j)));
            }

            auto weightsList = outputPtrs[1]->getValue<list_entry_t>(0);
            auto* weightsData = ListVector::getDataVector(outputPtrs[1]);
            std::vector<float> weights;
            for (auto j = weightsList.offset; j < weightsList.offset + weightsList.size; j++) {
                weights.push_back(static_cast<float>(weightsData->getValue<double>(j)));
            }

            add_sparse_document(*handle, static_cast<uint64_t>(offset),
                rust::Slice<const uint32_t>(indices.data(), indices.size()),
                rust::Slice<const float>(weights.data(), weights.size()));
        }
    }

    // 4. Commit.
    sparse_commit(*handle);

    // 5. Create catalog entry.
    std::vector<property_id_t> allPropertyIDs;
    allPropertyIDs.push_back(bd.indicesPropertyID);
    allPropertyIDs.push_back(bd.weightsPropertyID);
    auto auxInfo = std::make_unique<SparseVectorIndexAuxInfo>(
        indexPath, bd.indicesColumnName, bd.weightsColumnName);
    auto indexEntry = std::make_unique<catalog::IndexCatalogEntry>(
        SparseVectorCatalogEntry::TYPE_NAME, bd.tableID, bd.indexName, allPropertyIDs,
        std::move(auxInfo));
    catalog->createIndex(transaction, std::move(indexEntry));

    // 6. Create SparseVectorIndex and register on NodeTable.
    auto indexType = SparseVectorIndex::getIndexType();
    std::vector<column_id_t> columnIDs;
    std::vector<PhysicalTypeID> columnTypes;
    auto tableEntry = catalog->getTableCatalogEntry(transaction, bd.tableID);
    columnIDs.push_back(tableEntry->getColumnID(bd.indicesPropertyID));
    columnIDs.push_back(tableEntry->getColumnID(bd.weightsPropertyID));
    columnTypes.push_back(PhysicalTypeID::LIST);
    columnTypes.push_back(PhysicalTypeID::LIST);

    storage::IndexInfo indexInfo{bd.indexName, indexType.typeName, nodeTable.getTableID(),
        std::move(columnIDs), std::move(columnTypes),
        indexType.constraintType == storage::IndexConstraintType::PRIMARY,
        indexType.definitionType == storage::IndexDefinitionType::BUILTIN};
    auto storageInfo = std::make_unique<SparseVectorStorageInfo>(numRows);
    auto sparseIndex = std::make_unique<SparseVectorIndex>(
        std::move(indexInfo), std::move(storageInfo), std::move(handle));
    nodeTable.addIndex(std::move(sparseIndex));
    transaction->setForceCheckpoint();
    return 0;
}

// ── inferInputTypes ─────────────────────────────────────────────────────────

// Internal: tableName, indexName, indicesCol, weightsCol
static std::vector<LogicalType> inferInputTypesInternal(const binder::expression_vector&) {
    std::vector<LogicalType> types;
    types.push_back(LogicalType::STRING());
    types.push_back(LogicalType::STRING());
    types.push_back(LogicalType::STRING());
    types.push_back(LogicalType::STRING());
    return types;
}

// Public: tableName, indicesCol, weightsCol
static std::vector<LogicalType> inferInputTypesPublic(const binder::expression_vector&) {
    std::vector<LogicalType> types;
    types.push_back(LogicalType::STRING());
    types.push_back(LogicalType::STRING());
    types.push_back(LogicalType::STRING());
    return types;
}

// ── getFunctionSet ──────────────────────────────────────────────────────────

function_set InternalCreateSparseVectorFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<TableFunction>(name,
        std::vector{LogicalTypeID::STRING, LogicalTypeID::STRING,
            LogicalTypeID::STRING, LogicalTypeID::STRING});
    func->tableFunc = tableFunc;
    func->bindFunc = bindFuncInternal;
    func->initSharedStateFunc = SimpleTableFunc::initSharedState;
    func->initLocalStateFunc = TableFunction::initEmptyLocalState;
    func->canParallelFunc = [] { return false; };
    func->inferInputTypes = inferInputTypesInternal;
    functionSet.push_back(std::move(func));
    return functionSet;
}

function_set CreateSparseVectorFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<TableFunction>(name,
        std::vector{LogicalTypeID::STRING, LogicalTypeID::STRING, LogicalTypeID::STRING});
    func->tableFunc = TableFunction::emptyTableFunc;
    func->bindFunc = bindFunc;
    func->initSharedStateFunc = SimpleTableFunc::initSharedState;
    func->initLocalStateFunc = TableFunction::initEmptyLocalState;
    func->rewriteFunc = rewriteFunc;
    func->canParallelFunc = [] { return false; };
    func->inferInputTypes = inferInputTypesPublic;
    func->isReadOnly = false;
    functionSet.push_back(std::move(func));
    return functionSet;
}

} // namespace sparse_vector_extension
} // namespace rag3db
