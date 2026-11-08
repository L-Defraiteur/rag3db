#include "catalog/catalog.h"
#include "catalog/catalog_entry/table_catalog_entry.h"
#include "common/data_chunk/data_chunk.h"
#include "common/exception/binder.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/standalone_call_function.h"
#include "function/table/table_function.h"
#include "processor/execution_context.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/stats/table_stats.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
#include "transaction/transaction.h"

using namespace rag3db::common;

namespace rag3db {
namespace function {

// CALL analyze('Table') : la cardinalité et les comptes de valeurs distinctes d'une table de
// nœuds, rebâtis depuis ses lignes validées et vivantes en un balayage, remplacent ceux que les
// écritures ont tenus (une suppression, un SET ou une ligne écartée par IGNORE_ERRORS ne les
// reculent pas). Comme ANALYZE de PostgreSQL, explicite ; sans échantillon : tout est lu.
struct AnalyzeBindData final : TableFuncBindData {
    table_id_t tableID;

    explicit AnalyzeBindData(table_id_t tableID) : TableFuncBindData{0}, tableID{tableID} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<AnalyzeBindData>(tableID);
    }
};

static std::unique_ptr<TableFuncBindData> bindFunc(const main::ClientContext* context,
    const TableFuncBindInput* input) {
    const auto tableName = input->getLiteralVal<std::string>(0);
    const auto catalog = catalog::Catalog::Get(*context);
    const auto transaction = transaction::Transaction::Get(*context);
    if (!catalog->containsTable(transaction, tableName)) {
        throw BinderException{"Table " + tableName + " does not exist!"};
    }
    const auto tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    if (tableEntry->getTableType() != TableType::NODE) {
        throw BinderException{
            "Cannot analyze " + tableName + ": only node tables keep statistics."};
    }
    return std::make_unique<AnalyzeBindData>(tableEntry->getTableID());
}

static offset_t tableFunc(const TableFuncInput& input, TableFuncOutput&) {
    const auto& context = *input.context->clientContext;
    const auto tableID = input.bindData->constPtrCast<AnalyzeBindData>()->tableID;
    auto& table =
        storage::StorageManager::Get(context)->getTable(tableID)->cast<storage::NodeTable>();
    const auto transaction = transaction::Transaction::Get(context);
    const auto memoryManager = storage::MemoryManager::Get(context);
    table.replaceStats(table.rebuildStatsFromLiveRows(transaction, memoryManager));
    return 0;
}

function_set AnalyzeFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<TableFunction>(name, std::vector{LogicalTypeID::STRING});
    func->tableFunc = tableFunc;
    func->bindFunc = bindFunc;
    func->initSharedStateFunc = TableFunction::initEmptySharedState;
    func->initLocalStateFunc = TableFunction::initEmptyLocalState;
    func->canParallelFunc = []() { return false; };
    func->isReadOnly = false;
    functionSet.push_back(std::move(func));
    return functionSet;
}

} // namespace function
} // namespace rag3db
