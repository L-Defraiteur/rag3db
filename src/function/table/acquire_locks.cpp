#include "catalog/catalog.h"
#include "catalog/catalog_entry/node_table_catalog_entry.h"
#include "common/exception/binder.h"
#include "common/exception/transaction_manager.h"
#include "common/string_format.h"
#include "common/types/value/nested.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/standalone_call_function.h"
#include "function/table/table_function.h"
#include "processor/execution_context.h"
#include "transaction/lock_manager.h"
#include "transaction/transaction.h"
#include "transaction/transaction_context.h"
#include "transaction/transaction_manager.h"

using namespace rag3db::common;
using namespace rag3db::transaction;

namespace rag3db {
namespace function {

// CALL acquire_locks('Table', [clé, …]) : l'annonce des verrous en tête de transaction (marche
// V2, écart 1 de la note des verrous ; page coeur-cpp/08). Sous le mode multi-écrivains, en
// première instruction d'une transaction explicite, la transaction prend d'un coup, dans un
// ordre unique, l'index de la table en partagé et chaque clé en exclusif — ce que ses
// insertions (A3′) et ses écritures (A4′) prendraient une à une ; puis son instantané est repris
// après l'attente. Deux transactions qui annoncent ne s'interbloquent jamais ; celle qui a
// attendu lit l'état d'après l'attente et n'échoue pas à la sérialisation sur ce qu'elle a
// annoncé. Hors du mode, l'annonce vérifie ses arguments et ne fait rien.
struct AcquireLocksBindData final : TableFuncBindData {
    table_id_t tableID;
    std::vector<std::string> keys;

    AcquireLocksBindData(table_id_t tableID, std::vector<std::string> keys)
        : TableFuncBindData{0}, tableID{tableID}, keys{std::move(keys)} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<AcquireLocksBindData>(tableID, keys);
    }
};

static std::unique_ptr<TableFuncBindData> bindFunc(const main::ClientContext* context,
    const TableFuncBindInput* input) {
    const auto tableName = input->getLiteralVal<std::string>(0);
    const auto catalog = catalog::Catalog::Get(*context);
    const auto transaction = Transaction::Get(*context);
    if (!catalog->containsTable(transaction, tableName)) {
        throw BinderException{"Table " + tableName + " does not exist!"};
    }
    const auto tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    if (tableEntry->getTableType() != TableType::NODE) {
        throw BinderException{"Cannot acquire locks on " + tableName +
                              ": only the keys of a node table can be announced."};
    }
    const auto keysValue = input->getValue(1);
    if (keysValue.getDataType().getLogicalTypeID() != LogicalTypeID::LIST) {
        throw BinderException{stringFormat(
            "Argument 2 of acquire_locks has data type {}. A LIST of keys was expected.",
            keysValue.getDataType().toString())};
    }
    // La forme de verrou d'une clé est sa valeur imprimée (NodeTable::lockKeyOf) : un entier ou
    // une chaîne donnés ici désignent la même ressource que l'écriture qui suivra.
    std::vector<std::string> keys;
    for (auto i = 0u; i < NestedVal::getChildrenSize(&keysValue); ++i) {
        const auto child = NestedVal::getChildVal(&keysValue, i);
        if (child->isNull()) {
            throw BinderException{"A NULL key cannot be announced to acquire_locks."};
        }
        keys.push_back(child->toString());
    }
    return std::make_unique<AcquireLocksBindData>(tableEntry->getTableID(), std::move(keys));
}

static offset_t tableFunc(const TableFuncInput& input, TableFuncOutput&) {
    const auto& context = *input.context->clientContext;
    const auto bindData = input.bindData->constPtrCast<AcquireLocksBindData>();
    const auto transaction = Transaction::Get(context);
    if (TransactionContext::Get(context)->isAutoTransaction()) {
        throw TransactionManagerException(
            "acquire_locks must be the first statement of an explicit transaction (BEGIN "
            "TRANSACTION): in auto-commit it has nothing to protect.");
    }
    if (!transaction->usesLocks()) {
        return 0; // un seul écrivain : rien à prendre, rien à reprendre
    }
    if (transaction->hasAnnouncedLocks()) {
        throw TransactionManagerException(
            "acquire_locks was already called in this transaction: announce every key once, "
            "in one call.");
    }
    if (transaction->hasWritten()) {
        throw TransactionManagerException(
            "acquire_locks must come before any write of the transaction: announce first, "
            "then write.");
    }
    std::vector<LockRequest> requests;
    requests.reserve(bindData->keys.size() + 1);
    requests.push_back(LockRequest{LockResource::index(bindData->tableID), LockMode::SHARED});
    for (const auto& key : bindData->keys) {
        requests.push_back(
            LockRequest{LockResource::row(bindData->tableID, key), LockMode::EXCLUSIVE});
    }
    transaction->acquireLocks(requests);
    transaction->markLocksAnnounced();
    // L'instantané est pris après l'attente : la transaction lit ce que les détenteurs
    // précédents ont validé.
    TransactionManager::Get(context)->refreshSnapshot(transaction);
    return 0;
}

function_set AcquireLocksFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<TableFunction>(name,
        std::vector{LogicalTypeID::STRING, LogicalTypeID::LIST});
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
