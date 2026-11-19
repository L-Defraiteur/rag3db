#include <algorithm>

#include "catalog/catalog.h"
#include "catalog/catalog_entry/rel_group_catalog_entry.h"
#include "common/exception/binder.h"
#include "common/exception/transaction_manager.h"
#include "common/string_format.h"
#include "common/types/value/nested.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/standalone_call_function.h"
#include "function/table/table_function.h"
#include "processor/execution_context.h"
#include "transaction/transaction.h"
#include "transaction/transaction_context.h"

using namespace rag3db::common;
using namespace rag3db::transaction;

namespace rag3db {
namespace function {

// CALL lock_tables(['T', 'R', …]) : en tête d'une transaction explicite, le verrou de schéma de
// chaque table nommée, en partagé, comme l'écriture le prendrait à sa première ligne (question
// d'everything declarative, 2.7 (i), 19 novembre 2026). Un ALTER validé depuis le BEGIN refuse la
// prise par « Schema changed, retry: », avant toute écriture : le paquet annule et recommence sur
// place. Une fois la prise faite, un ALTER ou un DROP de ces tables attend la fin du paquet. Deux
// paquets ne s'attendent pas (partagé). Ni index ni clé : c'est le rôle de acquire_locks.
struct LockTablesBindData final : TableFuncBindData {
    // Les tables de stockage, triées et sans doublon : un groupe de relations donne toutes les
    // siennes. L'ordre unique évite qu'un paquet et un ALTER de plusieurs tables s'interbloquent
    // selon l'ordre des noms.
    std::vector<table_id_t> tableIDs;

    explicit LockTablesBindData(std::vector<table_id_t> tableIDs)
        : TableFuncBindData{0}, tableIDs{std::move(tableIDs)} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<LockTablesBindData>(tableIDs);
    }
};

static std::unique_ptr<TableFuncBindData> bindFunc(const main::ClientContext* context,
    const TableFuncBindInput* input) {
    const auto namesValue = input->getValue(0);
    if (namesValue.getDataType().getLogicalTypeID() != LogicalTypeID::LIST) {
        throw BinderException{stringFormat(
            "Argument 1 of lock_tables has data type {}. A LIST of table names was expected.",
            namesValue.getDataType().toString())};
    }
    const auto catalog = catalog::Catalog::Get(*context);
    const auto transaction = Transaction::Get(*context);
    std::vector<table_id_t> tableIDs;
    for (auto i = 0u; i < NestedVal::getChildrenSize(&namesValue); ++i) {
        const auto child = NestedVal::getChildVal(&namesValue, i);
        if (child->isNull() || child->getDataType().getLogicalTypeID() != LogicalTypeID::STRING) {
            throw BinderException{"lock_tables takes a list of table names (STRING)."};
        }
        const auto name = child->getValue<std::string>();
        if (!catalog->containsTable(transaction, name)) {
            throw BinderException{stringFormat(
                "lock_tables: table {} does not exist in this transaction's snapshot.", name)};
        }
        const auto entry = catalog->getTableCatalogEntry(transaction, name);
        if (entry->getType() == catalog::CatalogEntryType::REL_GROUP_ENTRY) {
            for (const auto& info :
                entry->constCast<catalog::RelGroupCatalogEntry>().getRelEntryInfos()) {
                tableIDs.push_back(info.oid);
            }
        } else {
            tableIDs.push_back(entry->getTableID());
        }
    }
    std::ranges::sort(tableIDs);
    const auto [first, last] = std::ranges::unique(tableIDs);
    tableIDs.erase(first, last);
    return std::make_unique<LockTablesBindData>(std::move(tableIDs));
}

static offset_t tableFunc(const TableFuncInput& input, TableFuncOutput&) {
    const auto& context = *input.context->clientContext;
    const auto bindData = input.bindData->constPtrCast<LockTablesBindData>();
    if (TransactionContext::Get(context)->isAutoTransaction()) {
        throw TransactionManagerException(
            "lock_tables must come at the head of an explicit transaction (BEGIN TRANSACTION): in "
            "auto-commit it has nothing to protect.");
    }
    const auto transaction = Transaction::Get(context);
    for (const auto tableID : bindData->tableIDs) {
        // Sans les écritures multiples, ne prend rien (usesLocks) ; sinon prend et contrôle le
        // schéma depuis le départ de la transaction.
        transaction->lockTableForWrite(tableID);
    }
    return 0;
}

function_set LockTablesFunction::getFunctionSet() {
    function_set functionSet;
    // Déclaré ANY : un type LIST sans type d'élément ne se construit pas au liage ; bindFunc
    // vérifie la liste (comme acquire_locks).
    auto func = std::make_unique<TableFunction>(name, std::vector{LogicalTypeID::ANY});
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
