#include "function/drop_tantivy_index.h"

#include "catalog/tantivy_index_catalog_entry.h"
#include "common/exception/binder.h"
#include "common/string_format.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/simple_table_function.h"
#include "main/client_context.h"
#include "processor/execution_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

#include <filesystem>

namespace rag3db {
namespace tantivy_fts_extension {

using namespace common;
using namespace main;
using namespace function;

// ── BindData ────────────────────────────────────────────────────────────────

struct DropTantivyBindData final : TableFuncBindData {
    std::string tableName;
    table_id_t tableID;
    std::string indexName;

    DropTantivyBindData(std::string tableName, table_id_t tableID, std::string indexName)
        : TableFuncBindData{binder::expression_vector{}, 0},
          tableName{std::move(tableName)}, tableID{tableID},
          indexName{std::move(indexName)} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<DropTantivyBindData>(*this);
    }
};

// ── Bind ────────────────────────────────────────────────────────────────────

static std::unique_ptr<TableFuncBindData> bindFunc(ClientContext* context,
    const TableFuncBindInput* input) {
    auto tableName = input->getLiteralVal<std::string>(0);

    auto catalog = catalog::Catalog::Get(*context);
    auto transaction = transaction::Transaction::Get(*context);
    auto tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    if (!tableEntry) {
        throw BinderException{stringFormat("Table '{}' does not exist.", tableName)};
    }

    // Index name = table name (convention).
    auto indexName = tableName;
    return std::make_unique<DropTantivyBindData>(tableName, tableEntry->getTableID(), indexName);
}

// ── rewriteFunc (public DROP_TANTIVY_INDEX) ─────────────────────────────────

static std::string rewriteFunc(ClientContext& context, const TableFuncBindData& bindData) {
    context.setUseInternalCatalogEntry(true);
    auto& bd = *bindData.constPtrCast<DropTantivyBindData>();
    auto query = stringFormat("CALL _DROP_TANTIVY_INDEX('{}');", bd.tableName);
    query += stringFormat("RETURN 'Tantivy index dropped from table {}.' AS result;", bd.tableName);
    return query;
}

// ── Internal _DROP_TANTIVY_INDEX ────────────────────────────────────────────

static offset_t internalTableFunc(const TableFuncInput& input, TableFuncOutput&) {
    auto& bd = *input.bindData->constPtrCast<DropTantivyBindData>();
    auto& context = *input.context;
    auto catalog = catalog::Catalog::Get(*context.clientContext);
    auto transaction = transaction::Transaction::Get(*context.clientContext);
    auto* storageManager = storage::StorageManager::Get(*context.clientContext);
    auto& nodeTable =
        storageManager->getTable(bd.tableID)->cast<storage::NodeTable>();

    // Get index path before dropping.
    auto indexEntry = catalog->getIndex(transaction, bd.tableID, bd.indexName);
    std::string indexPath;
    if (indexEntry && indexEntry->isLoaded()) {
        indexPath = indexEntry->getAuxInfo().cast<TantivyIndexAuxInfo>().indexPath;
    }

    // Drop from catalog.
    catalog->dropIndex(transaction, bd.tableID, bd.indexName);

    // Drop from NodeTable (drops Box<TantivyHandle> automatically).
    nodeTable.dropIndex(bd.indexName);

    // Remove index directory from disk.
    if (!indexPath.empty()) {
        std::error_code ec;
        std::filesystem::remove_all(indexPath, ec);
    }

    return 0;
}

// ── getFunctionSet ──────────────────────────────────────────────────────────

function_set InternalDropTantivyFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<TableFunction>(name,
        std::vector{LogicalTypeID::STRING});
    func->tableFunc = internalTableFunc;
    func->bindFunc = bindFunc;
    func->initSharedStateFunc = SimpleTableFunc::initSharedState;
    func->initLocalStateFunc = TableFunction::initEmptyLocalState;
    func->canParallelFunc = [] { return false; };
    functionSet.push_back(std::move(func));
    return functionSet;
}

function_set DropTantivyFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<TableFunction>(name,
        std::vector{LogicalTypeID::STRING});
    func->tableFunc = TableFunction::emptyTableFunc;
    func->bindFunc = bindFunc;
    func->initSharedStateFunc = SimpleTableFunc::initSharedState;
    func->initLocalStateFunc = TableFunction::initEmptyLocalState;
    func->rewriteFunc = rewriteFunc;
    func->canParallelFunc = [] { return false; };
    func->isReadOnly = false;
    functionSet.push_back(std::move(func));
    return functionSet;
}

} // namespace tantivy_fts_extension
} // namespace rag3db
