#include "function/create_lucivy_index.h"

#include "binder/expression/literal_expression.h"
#include "catalog/lucivy_index_catalog_entry.h"
#include "common/exception/binder.h"
#include "common/string_format.h"
#include "common/types/value/nested.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/simple_table_function.h"
#include "index/lucivy_index.h"
#include "main/client_context.h"
#include "processor/execution_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

#include <filesystem>

namespace rag3db {
namespace lucivy_fts_extension {

using namespace common;
using namespace main;
using namespace function;

// ── BindData ────────────────────────────────────────────────────────────────

struct FilterFieldInfo {
    std::string name;
    property_id_t propertyID;
    std::string lucivyType; // "u64", "i64", "f64", "string"
    LogicalTypeID originalTypeID; // for BOOL→i64, TIMESTAMP→i64 conversion
};

struct CreateLucivyBindData final : TableFuncBindData {
    std::string tableName;
    table_id_t tableID;
    std::string indexName;
    std::vector<std::string> propertyNames; // text fields
    std::vector<property_id_t> propertyIDs; // text field IDs
    std::string stemmer;
    std::vector<FilterFieldInfo> filterFields;

    CreateLucivyBindData(std::string tableName, table_id_t tableID, std::string indexName,
        std::vector<std::string> propertyNames, std::vector<property_id_t> propertyIDs,
        std::string stemmer, std::vector<FilterFieldInfo> filterFields)
        : TableFuncBindData{binder::expression_vector{}, 0 /* maxOffset */},
          tableName{std::move(tableName)}, tableID{tableID}, indexName{std::move(indexName)},
          propertyNames{std::move(propertyNames)}, propertyIDs{std::move(propertyIDs)},
          stemmer{std::move(stemmer)}, filterFields{std::move(filterFields)} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<CreateLucivyBindData>(*this);
    }
};

// ── Bind helpers ─────────────────────────────────────────────────────────────

static std::pair<std::vector<std::string>, std::vector<property_id_t>> resolveProperties(
    const catalog::TableCatalogEntry* tableEntry, const std::string& tableName,
    const Value& propertyValue) {
    std::vector<std::string> propertyNames;
    std::vector<property_id_t> propertyIDs;
    for (auto i = 0u; i < propertyValue.getChildrenSize(); i++) {
        auto propName = NestedVal::getChildVal(&propertyValue, i)->toString();
        if (!tableEntry->containsProperty(propName)) {
            throw BinderException{
                stringFormat("Property '{}' does not exist in table '{}'.", propName, tableName)};
        }
        if (tableEntry->getProperty(tableEntry->getPropertyID(propName)).getType() !=
            LogicalType::STRING()) {
            throw BinderException{
                "Lucivy index can only be built on STRING properties."};
        }
        propertyNames.push_back(propName);
        propertyIDs.push_back(tableEntry->getPropertyID(propName));
    }
    return {std::move(propertyNames), std::move(propertyIDs)};
}

static std::string mapLogicalTypeToLucivy(const LogicalType& type) {
    switch (type.getLogicalTypeID()) {
    case LogicalTypeID::INT64:
    case LogicalTypeID::INT32:
    case LogicalTypeID::INT16:
    case LogicalTypeID::INT8:
    case LogicalTypeID::BOOL:
    case LogicalTypeID::TIMESTAMP:
        return "i64";
    case LogicalTypeID::UINT64:
    case LogicalTypeID::UINT32:
    case LogicalTypeID::UINT16:
    case LogicalTypeID::UINT8:
        return "u64";
    case LogicalTypeID::DOUBLE:
    case LogicalTypeID::FLOAT:
        return "f64";
    case LogicalTypeID::STRING:
        return "string";
    default:
        return "";
    }
}

static std::vector<FilterFieldInfo> resolveFilterFields(
    const catalog::TableCatalogEntry* tableEntry, const std::string& tableName,
    const Value& filterFieldsValue) {
    std::vector<FilterFieldInfo> result;
    for (auto i = 0u; i < filterFieldsValue.getChildrenSize(); i++) {
        auto propName = NestedVal::getChildVal(&filterFieldsValue, i)->toString();
        if (!tableEntry->containsProperty(propName)) {
            throw BinderException{
                stringFormat("Filter field '{}' does not exist in table '{}'.", propName, tableName)};
        }
        auto propID = tableEntry->getPropertyID(propName);
        auto& propType = tableEntry->getProperty(propID).getType();
        auto lucivyType = mapLogicalTypeToLucivy(propType);
        if (lucivyType.empty()) {
            throw BinderException{stringFormat(
                "Filter field '{}' has type {} which is not supported by Lucivy. "
                "Supported types: INT64, INT32, INT16, INT8, UINT64, UINT32, UINT16, UINT8, "
                "DOUBLE, FLOAT, STRING, BOOLEAN, TIMESTAMP.", propName, propType.toString())};
        }
        result.push_back(FilterFieldInfo{propName, propID, lucivyType, propType.getLogicalTypeID()});
    }
    return result;
}

// ── Bind (public CREATE_LUCIVY_INDEX: tableName, propertyList) ──────────────

static std::unique_ptr<TableFuncBindData> bindFunc(ClientContext* context,
    const TableFuncBindInput* input) {
    auto tableName = input->getLiteralVal<std::string>(0);
    auto propertyValue =
        input->getParam(1)->constPtrCast<binder::LiteralExpression>()->getValue();
    auto stemmer = std::string("english");
    std::vector<FilterFieldInfo> filterFields;

    for (auto& [name, val] : input->optionalParams) {
        if (name == "stemmer") {
            stemmer = val.toString();
        }
    }

    auto catalog = catalog::Catalog::Get(*context);
    auto transaction = transaction::Transaction::Get(*context);
    auto tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    if (!tableEntry) {
        throw BinderException{stringFormat("Table '{}' does not exist.", tableName)};
    }

    for (auto& [name, val] : input->optionalParams) {
        if (name == "filter_fields") {
            filterFields = resolveFilterFields(tableEntry, tableName, val);
        }
    }

    auto [propertyNames, propertyIDs] = resolveProperties(tableEntry, tableName, propertyValue);
    auto indexName = tableName;
    return std::make_unique<CreateLucivyBindData>(tableName, tableEntry->getTableID(), indexName,
        std::move(propertyNames), std::move(propertyIDs), stemmer, std::move(filterFields));
}

// ── Bind (internal _CREATE_LUCIVY_INDEX: tableName, indexName, propertyList) ─

static std::unique_ptr<TableFuncBindData> bindFuncInternal(ClientContext* context,
    const TableFuncBindInput* input) {
    auto tableName = input->getLiteralVal<std::string>(0);
    auto indexName = input->getLiteralVal<std::string>(1);
    auto propertyValue =
        input->getParam(2)->constPtrCast<binder::LiteralExpression>()->getValue();
    auto stemmer = std::string("english");
    std::vector<FilterFieldInfo> filterFields;

    for (auto& [name, val] : input->optionalParams) {
        if (name == "stemmer") {
            stemmer = val.toString();
        }
    }

    auto catalog = catalog::Catalog::Get(*context);
    auto transaction = transaction::Transaction::Get(*context);
    auto tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    if (!tableEntry) {
        throw BinderException{stringFormat("Table '{}' does not exist.", tableName)};
    }

    for (auto& [name, val] : input->optionalParams) {
        if (name == "filter_fields") {
            filterFields = resolveFilterFields(tableEntry, tableName, val);
        }
    }

    auto [propertyNames, propertyIDs] = resolveProperties(tableEntry, tableName, propertyValue);
    return std::make_unique<CreateLucivyBindData>(tableName, tableEntry->getTableID(), indexName,
        std::move(propertyNames), std::move(propertyIDs), stemmer, std::move(filterFields));
}

// ── rewriteFunc (public CREATE_LUCIVY_INDEX) ───────────────────────────────

static std::string rewriteFunc(ClientContext& context, const TableFuncBindData& bindData) {
    context.setUseInternalCatalogEntry(true);
    auto& bd = *bindData.constPtrCast<CreateLucivyBindData>();
    std::string properties = "[";
    for (size_t i = 0; i < bd.propertyNames.size(); i++) {
        if (i > 0) properties += ", ";
        properties += stringFormat("'{}'", bd.propertyNames[i]);
    }
    properties += "]";
    std::string optionalArgs = stringFormat("stemmer := '{}'", bd.stemmer);
    if (!bd.filterFields.empty()) {
        std::string filterList = "[";
        for (size_t i = 0; i < bd.filterFields.size(); i++) {
            if (i > 0) filterList += ", ";
            filterList += stringFormat("'{}'", bd.filterFields[i].name);
        }
        filterList += "]";
        optionalArgs += stringFormat(", filter_fields := {}", filterList);
    }
    auto query = stringFormat(
        "CALL _CREATE_LUCIVY_INDEX('{}', '{}', {}, {});",
        bd.tableName, bd.indexName, properties, optionalArgs);
    query += stringFormat("RETURN 'Lucivy index created on table {}.' AS result;", bd.tableName);
    return query;
}

// ── Internal _CREATE_LUCIVY_INDEX ──────────────────────────────────────────

static offset_t tableFunc(const TableFuncInput& input, TableFuncOutput&) {
    auto& bd = *input.bindData->constPtrCast<CreateLucivyBindData>();
    auto& context = *input.context;
    auto transaction = transaction::Transaction::Get(*context.clientContext);
    auto catalog = catalog::Catalog::Get(*context.clientContext);
    auto* storageManager = storage::StorageManager::Get(*context.clientContext);
    auto& nodeTable =
        storageManager->getTable(bd.tableID)->cast<storage::NodeTable>();

    // 0. Skip if index already exists (idempotent — e.g. DB reopen with initLucivyEntries).
    auto existingIndex = nodeTable.getIndex(bd.indexName);
    if (existingIndex.has_value()) {
        return 0;
    }

    // 1. Build index path.
    std::string basePath;
    if (context.clientContext->isInMemory()) {
        // Each in-memory DB instance gets a unique directory based on its pointer address,
        // so multiple in-memory databases in the same process don't collide.
        auto dbId = std::to_string(
            reinterpret_cast<uintptr_t>(context.clientContext->getDatabase()));
        basePath = std::filesystem::temp_directory_path().string() + "/rag3db_lucivy/" + dbId;
    } else {
        basePath = std::filesystem::path(
            context.clientContext->getDatabasePath()).parent_path().string();
    }
    auto indexPath = stringFormat("{}/lucivy_indexes/{}/", basePath, bd.tableName);
    std::filesystem::create_directories(indexPath);

    // 2. Build Lucivy schema JSON.
    std::string schemaJson = "{\"fields\":[";
    for (size_t i = 0; i < bd.propertyNames.size(); i++) {
        if (i > 0) schemaJson += ",";
        schemaJson += "{\"name\":\"" + bd.propertyNames[i] +
            "\",\"type\":\"text\",\"stored\":true}";
    }
    for (auto& ff : bd.filterFields) {
        schemaJson += ",{\"name\":\"" + ff.name +
            "\",\"type\":\"" + ff.lucivyType +
            "\",\"stored\":true,\"indexed\":true,\"fast\":true}";
    }
    schemaJson += "]";
    // Le champ "stemmer" a disparu de SchemaConfig en lucivy v3, qui rejette
    // désormais les clés inconnues (lucivy 32ca1dc) : l'émettre fait échouer la
    // création d'index avec « unknown field `stemmer` ».
    //
    // On le RETIRE plutôt que de le renommer en "tokenizer" : ce dernier ne
    // désigne pas la même chose, et le renommer changerait silencieusement la
    // sélection du tokenizer. v3 n'ayant aucun champ stemmer, ne rien envoyer
    // est la traduction fidèle de ce qu'il sait faire.
    schemaJson += "}";

    // 3. Create Lucivy index via cxx bridge.
    auto handle = create_index(indexPath, schemaJson);

    // 4. Get field IDs — build mapping from field name to Lucivy field ID and type.
    auto fieldInfos = get_field_ids(*handle);
    std::unordered_map<std::string, uint32_t> fieldIdMap;
    std::unordered_map<std::string, std::string> fieldTypeMap;
    for (const auto& f : fieldInfos) {
        auto name = std::string(f.name);
        if (name == "_node_id") continue;
        fieldIdMap[name] = f.field_id;
        fieldTypeMap[name] = std::string(f.field_type);
    }

    bool hasMixed = !bd.filterFields.empty();

    // 5. Scan all existing rows and index them.
    auto numRows = nodeTable.getNumTotalRows(transaction);

    if (numRows > 0) {
        auto tableEntry = catalog->getTableCatalogEntry(transaction, bd.tableID);

        // Build column IDs for scan: text fields first, then filter fields.
        std::vector<column_id_t> scanColumnIDs;
        std::vector<LogicalType> scanTypes;
        auto numTextFields = bd.propertyIDs.size();
        for (auto& propID : bd.propertyIDs) {
            scanColumnIDs.push_back(tableEntry->getColumnID(propID));
            scanTypes.push_back(LogicalType::STRING());
        }
        for (auto& ff : bd.filterFields) {
            scanColumnIDs.push_back(tableEntry->getColumnID(ff.propertyID));
            auto& propType = tableEntry->getProperty(ff.propertyID).getType();
            scanTypes.push_back(propType.copy());
        }

        auto dataChunk = DataChunkState::getSingleValueDataChunkState();
        ValueVector idVector{LogicalType::INTERNAL_ID(),
            storage::MemoryManager::Get(*context.clientContext), dataChunk};

        // Build output vectors for the scan.
        std::vector<std::unique_ptr<ValueVector>> outputVecs;
        std::vector<ValueVector*> outputPtrs;
        for (size_t c = 0; c < scanColumnIDs.size(); c++) {
            auto vec = std::make_unique<ValueVector>(scanTypes[c].copy(),
                storage::MemoryManager::Get(*context.clientContext), dataChunk);
            outputPtrs.push_back(vec.get());
            outputVecs.push_back(std::move(vec));
        }

        storage::NodeTableScanState scanState{&idVector, outputPtrs, dataChunk};
        scanState.setToTable(transaction, &nodeTable, scanColumnIDs);

        internalID_t nodeID = {INVALID_OFFSET, bd.tableID};
        for (offset_t offset = 0; offset < numRows; offset++) {
            nodeID.offset = offset;
            idVector.setValue(0, nodeID);
            nodeTable.initScanState(transaction, scanState, nodeID.tableID, nodeID.offset);
            nodeTable.lookup(transaction, scanState);

            if (!hasMixed) {
                std::vector<DocFieldText> fields;
                for (size_t f = 0; f < numTextFields; f++) {
                    if (outputPtrs[f]->isNull(0)) continue;
                    auto text = outputPtrs[f]->getValue<ku_string_t>(0).getAsString();
                    DocFieldText field;
                    field.field_id = fieldIdMap[bd.propertyNames[f]];
                    field.value = rust::String(text);
                    fields.push_back(std::move(field));
                }
                add_document_texts(*handle, static_cast<uint64_t>(offset),
                    rust::Slice<const DocFieldText>(fields.data(), fields.size()));
            } else {
                std::vector<DocFieldText> textFields;
                std::vector<DocFieldU64> u64Fields;
                std::vector<DocFieldI64> i64Fields;
                std::vector<DocFieldF64> f64Fields;
                // Text fields.
                for (size_t f = 0; f < numTextFields; f++) {
                    if (outputPtrs[f]->isNull(0)) continue;
                    auto text = outputPtrs[f]->getValue<ku_string_t>(0).getAsString();
                    DocFieldText field;
                    field.field_id = fieldIdMap[bd.propertyNames[f]];
                    field.value = rust::String(text);
                    textFields.push_back(std::move(field));
                }
                // Filter fields.
                for (size_t ff = 0; ff < bd.filterFields.size(); ff++) {
                    auto vecIdx = numTextFields + ff;
                    if (outputPtrs[vecIdx]->isNull(0)) continue;
                    auto& ffInfo = bd.filterFields[ff];
                    auto fid = fieldIdMap[ffInfo.name];
                    if (ffInfo.lucivyType == "u64") {
                        DocFieldU64 field;
                        field.field_id = fid;
                        field.value = outputPtrs[vecIdx]->getValue<uint64_t>(0);
                        u64Fields.push_back(field);
                    } else if (ffInfo.lucivyType == "i64") {
                        DocFieldI64 field;
                        field.field_id = fid;
                        if (ffInfo.originalTypeID == LogicalTypeID::BOOL) {
                            field.value = outputPtrs[vecIdx]->getValue<bool>(0) ? 1 : 0;
                        } else {
                            field.value = outputPtrs[vecIdx]->getValue<int64_t>(0);
                        }
                        i64Fields.push_back(field);
                    } else if (ffInfo.lucivyType == "f64") {
                        DocFieldF64 field;
                        field.field_id = fid;
                        field.value = outputPtrs[vecIdx]->getValue<double>(0);
                        f64Fields.push_back(field);
                    } else if (ffInfo.lucivyType == "string") {
                        DocFieldText field;
                        field.field_id = fid;
                        field.value = rust::String(
                            outputPtrs[vecIdx]->getValue<ku_string_t>(0).getAsString());
                        textFields.push_back(std::move(field));
                    }
                }
                add_document_mixed(*handle, static_cast<uint64_t>(offset),
                    rust::Slice<const DocFieldText>(textFields.data(), textFields.size()),
                    rust::Slice<const DocFieldU64>(u64Fields.data(), u64Fields.size()),
                    rust::Slice<const DocFieldI64>(i64Fields.data(), i64Fields.size()),
                    rust::Slice<const DocFieldF64>(f64Fields.data(), f64Fields.size()));
            }
        }
    }

    // 6. Commit + reload.
    commit(*handle);
    reload_reader(*handle);

    // 7. Create catalog entry (includes filter field property IDs).
    std::vector<property_id_t> allPropertyIDs = bd.propertyIDs;
    for (auto& ff : bd.filterFields) {
        allPropertyIDs.push_back(ff.propertyID);
    }
    auto auxInfo = std::make_unique<LucivyIndexAuxInfo>(indexPath, schemaJson, bd.stemmer);
    auto indexEntry = std::make_unique<catalog::IndexCatalogEntry>(
        LucivyIndexCatalogEntry::TYPE_NAME, bd.tableID, bd.indexName, allPropertyIDs,
        std::move(auxInfo));
    catalog->createIndex(transaction, std::move(indexEntry));

    // 8. Create LucivyIndex and register on NodeTable.
    auto indexType = LucivyIndex::getIndexType();
    std::vector<column_id_t> columnIDs;
    std::vector<PhysicalTypeID> columnTypes;
    auto tableEntry = catalog->getTableCatalogEntry(transaction, bd.tableID);
    for (auto& propID : bd.propertyIDs) {
        columnIDs.push_back(tableEntry->getColumnID(propID));
        columnTypes.push_back(PhysicalTypeID::STRING);
    }
    for (auto& ff : bd.filterFields) {
        columnIDs.push_back(tableEntry->getColumnID(ff.propertyID));
        auto& propType = tableEntry->getProperty(ff.propertyID).getType();
        columnTypes.push_back(propType.getPhysicalType());
    }
    storage::IndexInfo indexInfo{bd.indexName, indexType.typeName, nodeTable.getTableID(),
        std::move(columnIDs), std::move(columnTypes),
        indexType.constraintType == storage::IndexConstraintType::PRIMARY,
        indexType.definitionType == storage::IndexDefinitionType::BUILTIN};
    auto storageInfo = std::make_unique<LucivyStorageInfo>(numRows);
    auto lucivyIndex = std::make_unique<LucivyIndex>(
        std::move(indexInfo), std::move(storageInfo), std::move(handle));
    nodeTable.addIndex(std::move(lucivyIndex));
    transaction->setForceCheckpoint();
    return 0;
}

// ── inferInputTypes ─────────────────────────────────────────────────────────

// For internal _CREATE_LUCIVY_INDEX (3 params: tableName, indexName, properties)
static std::vector<LogicalType> inferInputTypesInternal(const binder::expression_vector&) {
    std::vector<LogicalType> types;
    types.push_back(LogicalType::STRING());
    types.push_back(LogicalType::STRING());
    types.push_back(LogicalType::LIST(LogicalType::STRING()));
    return types;
}

// For public CREATE_LUCIVY_INDEX (2 params: tableName, properties)
static std::vector<LogicalType> inferInputTypesPublic(const binder::expression_vector&) {
    std::vector<LogicalType> types;
    types.push_back(LogicalType::STRING());
    types.push_back(LogicalType::LIST(LogicalType::STRING()));
    return types;
}

// ── getFunctionSet ──────────────────────────────────────────────────────────

function_set InternalCreateLucivyFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<TableFunction>(name,
        std::vector{LogicalTypeID::STRING, LogicalTypeID::STRING, LogicalTypeID::LIST});
    func->tableFunc = tableFunc;
    func->bindFunc = bindFuncInternal;
    func->initSharedStateFunc = SimpleTableFunc::initSharedState;
    func->initLocalStateFunc = TableFunction::initEmptyLocalState;
    func->canParallelFunc = [] { return false; };
    func->inferInputTypes = inferInputTypesInternal;
    functionSet.push_back(std::move(func));
    return functionSet;
}

function_set CreateLucivyFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<TableFunction>(name,
        std::vector{LogicalTypeID::STRING, LogicalTypeID::LIST});
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

} // namespace lucivy_fts_extension
} // namespace rag3db
