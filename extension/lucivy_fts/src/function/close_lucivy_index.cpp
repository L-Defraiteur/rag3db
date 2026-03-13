#include "function/close_lucivy_index.h"

#include "common/exception/binder.h"
#include "common/exception/runtime.h"
#include "common/string_format.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/simple_table_function.h"
#include "index/lucivy_index.h"
#include "main/client_context.h"
#include "processor/execution_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

namespace rag3db {
namespace lucivy_fts_extension {

using namespace common;
using namespace main;
using namespace function;

// ── BindData ────────────────────────────────────────────────────────────────

struct CloseLucivyBindData final : TableFuncBindData {
    std::string tableName;
    table_id_t tableID;
    std::string indexName;

    CloseLucivyBindData(std::string tableName, table_id_t tableID, std::string indexName)
        : TableFuncBindData{binder::expression_vector{}, 0},
          tableName{std::move(tableName)}, tableID{tableID},
          indexName{std::move(indexName)} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<CloseLucivyBindData>(*this);
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

    auto indexName = tableName;
    return std::make_unique<CloseLucivyBindData>(tableName, tableEntry->getTableID(), indexName);
}

// ── rewriteFunc (public CLOSE_LUCIVY_INDEX) ─────────────────────────────────

static std::string rewriteFunc(ClientContext& context, const TableFuncBindData& bindData) {
    context.setUseInternalCatalogEntry(true);
    auto& bd = *bindData.constPtrCast<CloseLucivyBindData>();
    auto query = stringFormat("CALL _CLOSE_LUCIVY_INDEX('{}');", bd.tableName);
    query +=
        stringFormat("RETURN 'Lucivy index closed for table {}.' AS result;", bd.tableName);
    return query;
}

// ── Internal _CLOSE_LUCIVY_INDEX ────────────────────────────────────────────

static offset_t internalTableFunc(const TableFuncInput& input, TableFuncOutput&) {
    auto& bd = *input.bindData->constPtrCast<CloseLucivyBindData>();
    auto& context = *input.context;
    auto* storageManager = storage::StorageManager::Get(*context.clientContext);
    auto& nodeTable =
        storageManager->getTable(bd.tableID)->cast<storage::NodeTable>();

    auto indexOpt = nodeTable.getIndex(bd.indexName);
    if (!indexOpt.has_value()) {
        // No index — nothing to close, not an error.
        return 0;
    }
    auto& lucivyIndex = indexOpt.value()->cast<LucivyIndex>();
    lucivyIndex.close();

    return 0;
}

// ── getFunctionSet ──────────────────────────────────────────────────────────

function_set InternalCloseLucivyFunction::getFunctionSet() {
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

function_set CloseLucivyFunction::getFunctionSet() {
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

} // namespace lucivy_fts_extension
} // namespace rag3db
