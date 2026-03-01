#include "function/drop_spatial_index.h"

#include "catalog/spatial_index_catalog_entry.h"
#include "common/exception/binder.h"
#include "common/string_format.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/simple_table_function.h"
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

struct DropSpatialBindData final : TableFuncBindData {
    std::string tableName;
    table_id_t tableID;
    std::string indexName;

    DropSpatialBindData(std::string tableName, table_id_t tableID, std::string indexName)
        : TableFuncBindData{binder::expression_vector{}, 0},
          tableName{std::move(tableName)}, tableID{tableID}, indexName{std::move(indexName)} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<DropSpatialBindData>(*this);
    }
};

// ── Bind ─────────────────────────────────────────────────────────────────────

static std::unique_ptr<TableFuncBindData> bindFunc(ClientContext* context,
    const TableFuncBindInput* input) {
    auto tableName = input->getLiteralVal<std::string>(0);
    auto indexName = input->getLiteralVal<std::string>(1);

    auto catalog = catalog::Catalog::Get(*context);
    auto transaction = transaction::Transaction::Get(*context);
    auto tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    if (!tableEntry) {
        throw BinderException{stringFormat("Table '{}' does not exist.", tableName)};
    }

    return std::make_unique<DropSpatialBindData>(tableName, tableEntry->getTableID(), indexName);
}

// ── Table function (internal _DROP_SPATIAL_INDEX) ────────────────────────────

static offset_t tableFunc(const TableFuncInput& input, TableFuncOutput&) {
    auto& bd = *input.bindData->constPtrCast<DropSpatialBindData>();
    auto& context = *input.context;
    auto transaction = transaction::Transaction::Get(*context.clientContext);
    auto catalog = catalog::Catalog::Get(*context.clientContext);
    auto* storageManager = storage::StorageManager::Get(*context.clientContext);

    // Get index file path before dropping.
    auto& nodeTable = storageManager->getTable(bd.tableID)->cast<storage::NodeTable>();
    auto indexOpt = nodeTable.getIndex(bd.indexName);
    std::string indexFilePath;
    if (indexOpt.has_value()) {
        auto& index = indexOpt.value()->cast<RTreeIndex>();
        auto& si = index.getStorageInfo().constCast<SpatialStorageInfo>();
        indexFilePath = si.indexFilePath;
    }

    // Drop from catalog.
    catalog->dropIndex(transaction, bd.tableID, bd.indexName);

    // Drop from storage.
    nodeTable.dropIndex(bd.indexName);

    // Delete index file.
    if (!indexFilePath.empty()) {
        std::filesystem::remove(indexFilePath);
    }

    transaction->setForceCheckpoint();
    return 0;
}

// ── Public DROP_SPATIAL_INDEX rewrite ────────────────────────────────────────

static std::string rewriteFunc(ClientContext& context, const TableFuncBindData& bindData) {
    context.setUseInternalCatalogEntry(true);
    auto& bd = *bindData.constPtrCast<DropSpatialBindData>();
    auto query = stringFormat(
        "CALL _DROP_SPATIAL_INDEX('{}', '{}');", bd.tableName, bd.indexName);
    query += stringFormat("RETURN 'Spatial index {} dropped from table {}.' AS result;",
        bd.indexName, bd.tableName);
    return query;
}

// ── getFunctionSet ───────────────────────────────────────────────────────────

function_set InternalDropSpatialIndexFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<TableFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::STRING, LogicalTypeID::STRING});
    func->tableFunc = tableFunc;
    func->bindFunc = bindFunc;
    result.push_back(std::move(func));
    return result;
}

function_set DropSpatialIndexFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<TableFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::STRING, LogicalTypeID::STRING});
    func->tableFunc = tableFunc;
    func->bindFunc = bindFunc;
    func->rewriteFunc = rewriteFunc;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
