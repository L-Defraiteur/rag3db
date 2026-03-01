#include "function/create_spatial_index.h"

#include "binder/expression/literal_expression.h"
#include "catalog/spatial_index_catalog_entry.h"
#include "common/exception/binder.h"
#include "common/string_format.h"
#include "common/types/value/nested.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/simple_table_function.h"
#include "index/rtree.h"
#include "index/rtree_index.h"
#include "main/client_context.h"
#include "processor/execution_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

#include <filesystem>

namespace rag3db {
namespace geo_extension {

using namespace common;
using namespace main;
using namespace function;

struct CreateSpatialBindData final : TableFuncBindData {
    std::string tableName;
    table_id_t tableID;
    std::string indexName;
    std::vector<std::string> columnNames; // e.g. ["lat", "lon"]
    std::vector<property_id_t> propertyIDs;
    std::string metric; // "euclidean" or "haversine"

    CreateSpatialBindData(std::string tableName, table_id_t tableID, std::string indexName,
        std::vector<std::string> columnNames, std::vector<property_id_t> propertyIDs,
        std::string metric)
        : TableFuncBindData{binder::expression_vector{}, 0},
          tableName{std::move(tableName)}, tableID{tableID}, indexName{std::move(indexName)},
          columnNames{std::move(columnNames)}, propertyIDs{std::move(propertyIDs)},
          metric{std::move(metric)} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<CreateSpatialBindData>(*this);
    }
};

// ── Bind ─────────────────────────────────────────────────────────────────────

static std::unique_ptr<TableFuncBindData> bindFunc(ClientContext* context,
    const TableFuncBindInput* input) {
    auto tableName = input->getLiteralVal<std::string>(0);
    auto indexName = input->getLiteralVal<std::string>(1);
    auto columnsValue =
        input->getParam(2)->constPtrCast<binder::LiteralExpression>()->getValue();

    auto metric = std::string("euclidean");
    for (auto& [name, val] : input->optionalParams) {
        if (name == "metric") {
            metric = val.toString();
        }
    }

    auto catalog = catalog::Catalog::Get(*context);
    auto transaction = transaction::Transaction::Get(*context);
    auto tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    if (!tableEntry) {
        throw BinderException{stringFormat("Table '{}' does not exist.", tableName)};
    }

    std::vector<std::string> columnNames;
    std::vector<property_id_t> propertyIDs;
    for (auto i = 0u; i < columnsValue.getChildrenSize(); i++) {
        auto colName = NestedVal::getChildVal(&columnsValue, i)->toString();
        if (!tableEntry->containsProperty(colName)) {
            throw BinderException{
                stringFormat("Column '{}' does not exist in table '{}'.", colName, tableName)};
        }
        auto propID = tableEntry->getPropertyID(colName);
        auto& propType = tableEntry->getProperty(propID).getType();
        if (propType.getLogicalTypeID() != LogicalTypeID::DOUBLE &&
            propType.getLogicalTypeID() != LogicalTypeID::FLOAT) {
            throw BinderException{stringFormat(
                "Spatial index column '{}' must be DOUBLE or FLOAT, got {}.",
                colName, propType.toString())};
        }
        columnNames.push_back(colName);
        propertyIDs.push_back(propID);
    }

    if (columnNames.size() < 2) {
        throw BinderException{"Spatial index requires at least 2 coordinate columns."};
    }

    return std::make_unique<CreateSpatialBindData>(tableName, tableEntry->getTableID(),
        indexName, std::move(columnNames), std::move(propertyIDs), metric);
}

// ── Table function (internal _CREATE_SPATIAL_INDEX) ──────────────────────────

static offset_t tableFunc(const TableFuncInput& input, TableFuncOutput&) {
    auto& bd = *input.bindData->constPtrCast<CreateSpatialBindData>();
    auto& context = *input.context;
    auto transaction = transaction::Transaction::Get(*context.clientContext);
    auto catalog = catalog::Catalog::Get(*context.clientContext);
    auto* storageManager = storage::StorageManager::Get(*context.clientContext);
    auto& nodeTable =
        storageManager->getTable(bd.tableID)->cast<storage::NodeTable>();

    uint32_t dims = static_cast<uint32_t>(bd.columnNames.size());
    auto metric = bd.metric == "haversine" ? DistanceMetric::HAVERSINE : DistanceMetric::EUCLIDEAN;
    auto rtree = std::make_unique<RTree>(dims, 16, 4, metric);

    // Scan all existing rows and insert into R-tree.
    auto numRows = nodeTable.getNumTotalRows(transaction);
    if (numRows > 0) {
        auto tableEntry = catalog->getTableCatalogEntry(transaction, bd.tableID);
        std::vector<column_id_t> scanColumnIDs;
        for (auto& propID : bd.propertyIDs) {
            scanColumnIDs.push_back(tableEntry->getColumnID(propID));
        }

        auto dataChunkState = DataChunkState::getSingleValueDataChunkState();
        ValueVector idVector{LogicalType::INTERNAL_ID(),
            storage::MemoryManager::Get(*context.clientContext), dataChunkState};

        std::vector<std::unique_ptr<ValueVector>> outputVecs;
        std::vector<ValueVector*> outputPtrs;
        for (size_t c = 0; c < scanColumnIDs.size(); c++) {
            auto vec = std::make_unique<ValueVector>(LogicalType::DOUBLE(),
                storage::MemoryManager::Get(*context.clientContext), dataChunkState);
            outputPtrs.push_back(vec.get());
            outputVecs.push_back(std::move(vec));
        }

        storage::NodeTableScanState scanState{&idVector, outputPtrs, dataChunkState};
        scanState.setToTable(transaction, &nodeTable, scanColumnIDs);

        internalID_t nodeID = {INVALID_OFFSET, bd.tableID};
        for (offset_t offset = 0; offset < numRows; offset++) {
            nodeID.offset = offset;
            idVector.setValue(0, nodeID);
            nodeTable.initScanState(transaction, scanState, nodeID.tableID, nodeID.offset);
            nodeTable.lookup(transaction, scanState);

            bool isNull = false;
            std::vector<double> coords(dims);
            for (uint32_t d = 0; d < dims; d++) {
                if (outputPtrs[d]->isNull(0)) {
                    isNull = true;
                    break;
                }
                coords[d] = outputPtrs[d]->getValue<double>(0);
            }
            if (!isNull) {
                rtree->insert(offset, coords);
            }
        }
    }

    // Build index file path.
    std::string basePath;
    if (context.clientContext->isInMemory()) {
        auto dbId = std::to_string(
            reinterpret_cast<uintptr_t>(context.clientContext->getDatabase()));
        basePath = std::filesystem::temp_directory_path().string() + "/rag3db_spatial/" + dbId;
    } else {
        basePath = std::filesystem::path(
            context.clientContext->getDatabasePath()).parent_path().string();
    }
    auto indexFilePath = stringFormat("{}/spatial_indexes/{}/{}.rtree",
        basePath, bd.tableName, bd.indexName);

    // Register in catalog.
    std::vector<column_id_t> columnIDs;
    std::vector<PhysicalTypeID> columnTypes;
    auto tableEntry = catalog->getTableCatalogEntry(transaction, bd.tableID);
    for (auto& propID : bd.propertyIDs) {
        columnIDs.push_back(tableEntry->getColumnID(propID));
        columnTypes.push_back(PhysicalTypeID::DOUBLE);
    }
    auto auxInfo = std::make_unique<SpatialIndexAuxInfo>(dims, bd.columnNames, bd.metric);
    auto indexEntry = std::make_unique<catalog::IndexCatalogEntry>(
        SpatialIndexCatalogEntry::TYPE_NAME, bd.tableID, bd.indexName,
        bd.propertyIDs, std::move(auxInfo));
    catalog->createIndex(transaction, std::move(indexEntry));

    // Create storage info and RTreeIndex.
    auto si = std::make_unique<SpatialStorageInfo>(indexFilePath, rtree->size());
    auto indexType = RTreeIndex::getIndexType();
    storage::IndexInfo indexInfo{bd.indexName, indexType.typeName, nodeTable.getTableID(),
        std::move(columnIDs), std::move(columnTypes),
        indexType.constraintType == storage::IndexConstraintType::PRIMARY,
        indexType.definitionType == storage::IndexDefinitionType::BUILTIN};
    auto index = std::make_unique<RTreeIndex>(
        std::move(indexInfo), std::move(si), std::move(rtree));

    // Save to disk and add to node table.
    index->saveToFile();
    nodeTable.addIndex(std::move(index));

    transaction->setForceCheckpoint();
    return 0;
}

// ── Public CREATE_SPATIAL_INDEX rewrite ──────────────────────────────────────

static std::unique_ptr<TableFuncBindData> publicBindFunc(ClientContext* context,
    const TableFuncBindInput* input) {
    auto tableName = input->getLiteralVal<std::string>(0);
    auto indexName = input->getLiteralVal<std::string>(1);
    auto columnsValue =
        input->getParam(2)->constPtrCast<binder::LiteralExpression>()->getValue();

    auto metric = std::string("euclidean");
    for (auto& [name, val] : input->optionalParams) {
        if (name == "metric") metric = val.toString();
    }

    auto catalog = catalog::Catalog::Get(*context);
    auto transaction = transaction::Transaction::Get(*context);
    auto tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    if (!tableEntry) {
        throw BinderException{stringFormat("Table '{}' does not exist.", tableName)};
    }

    std::vector<std::string> columnNames;
    std::vector<property_id_t> propertyIDs;
    for (auto i = 0u; i < columnsValue.getChildrenSize(); i++) {
        auto colName = NestedVal::getChildVal(&columnsValue, i)->toString();
        if (!tableEntry->containsProperty(colName)) {
            throw BinderException{
                stringFormat("Column '{}' does not exist in table '{}'.", colName, tableName)};
        }
        columnNames.push_back(colName);
        propertyIDs.push_back(tableEntry->getPropertyID(colName));
    }

    return std::make_unique<CreateSpatialBindData>(tableName, tableEntry->getTableID(),
        indexName, std::move(columnNames), std::move(propertyIDs), metric);
}

static std::string rewriteFunc(ClientContext& context, const TableFuncBindData& bindData) {
    context.setUseInternalCatalogEntry(true);
    auto& bd = *bindData.constPtrCast<CreateSpatialBindData>();
    std::string cols = "[";
    for (size_t i = 0; i < bd.columnNames.size(); i++) {
        if (i > 0) cols += ", ";
        cols += stringFormat("'{}'", bd.columnNames[i]);
    }
    cols += "]";
    auto query = stringFormat(
        "CALL _CREATE_SPATIAL_INDEX('{}', '{}', {}, metric := '{}');",
        bd.tableName, bd.indexName, cols, bd.metric);
    query += stringFormat("RETURN 'Spatial index {} created on table {}.' AS result;",
        bd.indexName, bd.tableName);
    return query;
}

// ── getFunctionSet ───────────────────────────────────────────────────────────

function_set InternalCreateSpatialIndexFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<TableFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::STRING, LogicalTypeID::STRING,
            LogicalTypeID::LIST});
    func->tableFunc = tableFunc;
    func->bindFunc = bindFunc;
    result.push_back(std::move(func));
    return result;
}

function_set CreateSpatialIndexFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<TableFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::STRING, LogicalTypeID::STRING,
            LogicalTypeID::LIST});
    func->tableFunc = tableFunc;
    func->bindFunc = publicBindFunc;
    func->rewriteFunc = rewriteFunc;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
