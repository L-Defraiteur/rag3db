#include "transaction/transaction_context.h"

#include "common/exception/transaction_manager.h"
#include "common/string_format.h"
#include "main/client_context.h"
#include "main/database.h"
#include "transaction/transaction_manager.h"

using namespace rag3db::common;

namespace rag3db {
namespace transaction {

TransactionContext::TransactionContext(main::ClientContext& clientContext)
    : clientContext{clientContext}, mode{TransactionMode::AUTO}, activeTransaction{nullptr} {}

TransactionContext::~TransactionContext() = default;

// Le mode ne passe à MANUAL qu'une fois la transaction obtenue : un BEGIN refusé (second
// écrivain, point de reprise en échec, tampon plein) laissait la connexion en MANUAL sans
// transaction, ROLLBACK répondait « No active transaction » sans la remettre en AUTO, et la
// requête suivante se liait sur une transaction nulle (ticket du 18 octobre 2026 du banc).
void TransactionContext::beginReadTransaction() {
    std::unique_lock lck{mtx};
    beginTransactionInternal(TransactionType::READ_ONLY);
    mode = TransactionMode::MANUAL;
}

void TransactionContext::beginWriteTransaction() {
    std::unique_lock lck{mtx};
    beginTransactionInternal(TransactionType::WRITE);
    mode = TransactionMode::MANUAL;
}

void TransactionContext::beginAutoTransaction(bool readOnlyStatement) {
    // LCOV_EXCL_START
    if (hasActiveTransaction()) {
        throw TransactionManagerException(
            "Cannot start a new transaction while there is an active transaction.");
    }
    // LCOV_EXCL_STOP
    beginTransactionInternal(
        readOnlyStatement ? TransactionType::READ_ONLY : TransactionType::WRITE);
}

void TransactionContext::beginRecoveryTransaction() {
    std::unique_lock lck{mtx};
    beginTransactionInternal(TransactionType::RECOVERY);
    mode = TransactionMode::MANUAL;
}

void TransactionContext::validateManualTransaction(bool readOnlyStatement) const {
    KU_ASSERT(hasActiveTransaction());
    if (!hasActiveTransaction()) {
        // Jamais atteint depuis que le mode ne passe à MANUAL qu'avec sa transaction ; un refus
        // nommé vaut mieux qu'une liaison sur une transaction nulle si un chemin l'oubliait.
        throw TransactionManagerException(
            "The connection is in manual transaction mode without a transaction. Send ROLLBACK.");
    }
    if (activeTransaction->isReadOnly() && !readOnlyStatement) {
        throw TransactionManagerException(
            "Can not execute a write query inside a read-only transaction.");
    }
}

void TransactionContext::commit() {
    if (!hasActiveTransaction()) {
        return;
    }
    if (manualTransactionFailed) {
        // Rien à valider : le COMMIT ferme le bloc comme un ROLLBACK, et le dit.
        rollback();
        throw TransactionManagerException(
            stringFormat("{}: nothing was committed. The transaction is now closed.",
                MANUAL_TRANSACTION_FAILED));
    }
    clientContext.getDatabase()->getTransactionManager()->commit(clientContext, activeTransaction);
    clearTransaction();
}

void TransactionContext::rollback() {
    if (!hasActiveTransaction()) {
        return;
    }
    try {
        clientContext.getDatabase()->getTransactionManager()->rollback(clientContext,
            activeTransaction);
    } catch (...) {
        // Le gestionnaire a retiré la transaction même en échec : la connexion l'oublie aussi,
        // sinon elle restait en mode manuel sur une transaction détruite.
        clearTransaction();
        throw;
    }
    clearTransaction();
}

void TransactionContext::rollbackAfterStatementFailure() {
    if (!hasActiveTransaction()) {
        return;
    }
    if (mode != TransactionMode::MANUAL) {
        rollback();
        return;
    }
    // Marche T0 : la transaction reste ouverte et garde ses verrous jusqu'au ROLLBACK (ou au
    // COMMIT, qui annule et le dit) ; ses effets sont annulés tout de suite.
    // Échoué même si l'annulation lève : le bloc ne sert plus rien jusqu'au ROLLBACK.
    manualTransactionFailed = true;
    clientContext.getDatabase()->getTransactionManager()->failAfterStatement(clientContext,
        activeTransaction);
}

void TransactionContext::throwIfManualTransactionFailed() const {
    if (manualTransactionFailed) {
        throw TransactionManagerException(stringFormat(
            "{}: nothing it wrote remains, and nothing can run in it. Send ROLLBACK to close it.",
            MANUAL_TRANSACTION_FAILED));
    }
}

void TransactionContext::clearTransaction() {
    activeTransaction = nullptr;
    mode = TransactionMode::AUTO;
    manualTransactionFailed = false;
}

TransactionContext* TransactionContext::Get(const main::ClientContext& context) {
    return context.transactionContext.get();
}

void TransactionContext::beginTransactionInternal(TransactionType transactionType) {
    KU_ASSERT(!activeTransaction);
    activeTransaction = clientContext.getDatabase()->getTransactionManager()->beginTransaction(
        clientContext, transactionType);
}

} // namespace transaction
} // namespace rag3db
