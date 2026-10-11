#include "transaction/transaction_manager.h"

#include <algorithm>
#include <cstdio>

#include "storage/checkpoint_profile.h"

#include <thread>

#include "common/exception/checkpoint.h"
#include "common/exception/runtime.h"
#include "common/exception/transaction_manager.h"
#include "common/string_format.h"
#include "main/attached_database.h"
#include "main/client_context.h"
#include "main/database.h"
#include "main/db_config.h"
#include "storage/checkpointer.h"
#include "storage/wal/local_wal.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace rag3db {
namespace transaction {

Transaction* TransactionManager::beginTransaction(main::ClientContext& clientContext,
    TransactionType type) {
    // We acquire the lock for starting new transactions. In case this cannot be acquired, this
    // ensures calls to other public functions are not restricted.
    std::unique_lock publicFunctionLck{mtxForSerializingPublicFunctionCalls};
    // Pas de nouvelle transaction pendant qu'un point de reprise attend ou s'écrit.
    transactionsChanged.wait(publicFunctionLck, [this] { return !checkpointPending; });
    throwIfCheckpointFailedNoLock();
    switch (type) {
    case TransactionType::READ_ONLY: {
        auto transaction =
            std::make_unique<Transaction>(clientContext, type, ++lastTransactionID, lastTimestamp);
        activeTransactions.push_back(std::move(transaction));
        return activeTransactions.back().get();
    }
    case TransactionType::RECOVERY:
    case TransactionType::WRITE: {
        if (!clientContext.getDBConfig()->enableMultiWrites && hasActiveWriteTransactionNoLock()) {
            throw TransactionManagerException(
                "Cannot start a new write transaction in the system. "
                "Only one write transaction at a time is allowed in the system.");
        }
        auto transaction =
            std::make_unique<Transaction>(clientContext, type, ++lastTransactionID, lastTimestamp);
        if (transaction->shouldLogToWAL()) {
            transaction->getLocalWAL().logBeginTransaction();
        }
        activeTransactions.push_back(std::move(transaction));
        return activeTransactions.back().get();
    }
        // LCOV_EXCL_START
    default: {
        throw TransactionManagerException("Invalid transaction type to begin transaction.");
    }
        // LCOV_EXCL_STOP
    }
}

void TransactionManager::refreshSnapshot(Transaction* transaction) {
    std::unique_lock lck{mtxForSerializingPublicFunctionCalls};
    KU_ASSERT(!transaction->hasWritten());
    transaction->startTS = lastTimestamp;
}

void TransactionManager::commit(main::ClientContext& clientContext, Transaction* transaction) {
    std::unique_lock lck{mtxForSerializingPublicFunctionCalls};
    clientContext.cleanUp();
    switch (transaction->getType()) {
    case TransactionType::READ_ONLY: {
        clearTransactionNoLock(transaction->getID());
    } break;
    case TransactionType::RECOVERY:
    case TransactionType::WRITE: {
        const auto forced = transaction->shouldForceCheckpoint();
        if (forced && checkpointPending &&
            commitInCheckpointQueueNoLock(clientContext, lck, transaction)) {
            // A7 : validée dans la file, et contenue dans un point de reprise écrit après elle.
            return;
        }
        if (forced) {
            // Une écriture dont la durabilité est son propre point de reprise (COPY FROM, hors
            // journal) ne devient visible que si ce point de reprise peut se faire : on attend
            // d'abord le départ des autres transactions. Si le délai expire, l'exception sort
            // avant la validation, l'appelant annule, et l'erreur dit vrai : rien n'est validé.
            stopNewTransactionsAndWaitForOthersNoLock(lck, transaction);
        }
        lastTimestamp++;
        transaction->commitTS = lastTimestamp;
        storage::CheckpointProfile profile;
        try {
            transaction->commit(&wal);
        } catch (...) {
            if (forced) {
                allowNewTransactionsNoLock();
            }
            throw;
        }
        const auto commitMs = profile.lap();
        auto shouldCheckpoint =
            forced || Checkpointer::canAutoCheckpoint(clientContext, *transaction);
        clearTransactionNoLock(transaction->getID());
        if (forced) {
            checkpointNoLock(clientContext, lck, true /* newTransactionsAlreadyStopped */);
        } else if (shouldCheckpoint) {
            if (!activeTransactions.empty() || checkpointPending) {
                // L'écriture est validée et au journal. Comme PostgreSQL, un point de reprise
                // ne retarde ni ne fait échouer une validation : si une autre transaction est
                // ouverte, il est reporté à une validation suivante, sans attendre.
                postponeCheckpointNoLock();
            } else {
                checkpointNoLock(clientContext, lck);
            }
        }
        if (storage::CheckpointProfile::enabled() && (shouldCheckpoint || commitMs >= 50.0)) {
            const auto checkpointMs = profile.lap();
            fprintf(stderr,
                "[commit-profile] total=%.1f ms : validation=%.1f point-de-reprise=%.1f (%s)\n",
                profile.total(), commitMs, checkpointMs,
                !shouldCheckpoint ? "aucun" : forced ? "forcé" : "au seuil");
        }
    } break;
        // LCOV_EXCL_START
    default: {
        throw TransactionManagerException("Invalid transaction type to commit.");
    }
        // LCOV_EXCL_STOP
    }
}

// Note: We take in additional `transaction` here is due to that `transactionContext` might be
// destructed when a transaction throws an exception, while we need to roll back the active
// transaction still.
void TransactionManager::failAfterStatement(main::ClientContext& clientContext,
    Transaction* transaction) {
    std::unique_lock lck{mtxForSerializingPublicFunctionCalls};
    clientContext.cleanUp();
    KU_ASSERT(transaction->isWriteTransaction() || transaction->isReadOnly());
    if (transaction->isWriteTransaction()) {
        transaction->failAfterStatement(&wal);
    }
    // Pas de clearTransactionNoLock : la transaction reste active et garde ses verrous.
}

void TransactionManager::rollback(main::ClientContext& clientContext, Transaction* transaction) {
    std::unique_lock lck{mtxForSerializingPublicFunctionCalls};
    // Celui qui attend le départ des transactions est réveillé par clearTransactionNoLock.
    clientContext.cleanUp();
    switch (transaction->getType()) {
    case TransactionType::READ_ONLY: {
        clearTransactionNoLock(transaction->getID());
    } break;
    case TransactionType::RECOVERY:
    case TransactionType::WRITE: {
        if (!transaction->hasFailed()) {
            transaction->rollback(&wal);
        }
        clearTransactionNoLock(transaction->getID());
    } break;
    default: {
        throw TransactionManagerException("Invalid transaction type to rollback.");
    }
    }
}

void TransactionManager::checkpoint(main::ClientContext& clientContext) {
    std::unique_lock lck{mtxForSerializingPublicFunctionCalls};
    if (clientContext.isInMemory()) {
        return;
    }
    throwIfCheckpointFailedNoLock();
    checkpointNoLock(clientContext, lck);
}

void TransactionManager::throwIfCheckpointFailedNoLock() const {
    if (checkpointFailed) {
        throw TransactionManagerException(common::stringFormat(
            "{}: what it holds in memory no longer matches its files. No committed transaction "
            "is lost: the journal is replayed when the database is reopened.",
            REOPEN_AFTER_FAILED_CHECKPOINT));
    }
}

TransactionManager* TransactionManager::Get(const main::ClientContext& context) {
    if (context.getAttachedDatabase() != nullptr) {
        context.getAttachedDatabase()->getTransactionManager();
    }
    return context.getDatabase()->getTransactionManager();
}

void TransactionManager::stopNewTransactionsAndWaitForOthersNoLock(
    std::unique_lock<std::mutex>& publicLock, const Transaction* staying) {
    // Un seul point de reprise attend à la fois ; une validation qui en veut un pendant ce temps
    // se range dans la file (commitInCheckpointQueueNoLock) au lieu d'attendre ici.
    transactionsChanged.wait(publicLock, [this] { return !checkpointPending; });
    checkpointPending = true;
    const auto othersLeftOrQueued = [this, staying] {
        return std::ranges::all_of(activeTransactions, [this, staying](const auto& transaction) {
            return transaction.get() == staying ||
                   std::ranges::find(queuedCheckpointCommits, transaction.get()) !=
                       queuedCheckpointCommits.end();
        });
    };
    // wait_for relâche le verrou public : les transactions ouvertes peuvent valider ou annuler.
    if (!transactionsChanged.wait_for(publicLock,
            std::chrono::microseconds(checkpointWaitTimeoutInMicros), othersLeftOrQueued)) {
        allowNewTransactionsNoLock();
        throw TransactionManagerException(
            "Timeout waiting for active transactions to leave the system before "
            "checkpointing. If you have an open transaction, please close it and try again.");
    }
    if (!queuedCheckpointCommits.empty()) {
        // Il ne reste que des validations rangées dans la file : elles valident maintenant, et
        // le point de reprise qui suit les contient. On attend qu'elles soient toutes PARTIES,
        // pas seulement sorties de la file : une validation rangée qui échoue reste active,
        // à moitié appliquée, jusqu'à son annulation, et le point de reprise ne doit pas
        // l'écrire. L'attente est bornée comme la première.
        const auto othersLeft = [this, staying] {
            return std::ranges::all_of(activeTransactions,
                [staying](const auto& transaction) { return transaction.get() == staying; });
        };
        commitQueuedNow = true;
        transactionsChanged.notify_all();
        const auto left = transactionsChanged.wait_for(publicLock,
            std::chrono::microseconds(checkpointWaitTimeoutInMicros), othersLeft);
        commitQueuedNow = false;
        if (!left) {
            // Les validations de la file déjà faites écrivent elles-mêmes le point de reprise
            // qui les contient (commitInCheckpointQueueNoLock).
            allowNewTransactionsNoLock();
            throw TransactionManagerException(
                "Timeout waiting for active transactions to leave the system before "
                "checkpointing. If you have an open transaction, please close it and try again.");
        }
    }
}

bool TransactionManager::commitInCheckpointQueueNoLock(main::ClientContext& clientContext,
    std::unique_lock<std::mutex>& publicLock, Transaction* transaction) {
    queuedCheckpointCommits.push_back(transaction);
    const auto leaveQueue = [this, transaction] {
        std::erase(queuedCheckpointCommits, transaction);
        transactionsChanged.notify_all();
    };
    // Le point de reprise en attente réévalue : cette transaction ne le bloque plus.
    transactionsChanged.notify_all();
    transactionsChanged.wait(publicLock,
        [this] { return commitQueuedNow || !checkpointPending || checkpointFailed; });
    if (checkpointFailed || !commitQueuedNow) {
        // Le point de reprise en attente a échoué, ou il a renoncé (une autre transaction est
        // restée au-delà du délai) : rien n'est validé, l'appelant reprend le chemin ordinaire.
        leaveQueue();
        throwIfCheckpointFailedNoLock();
        return false;
    }
    lastTimestamp++;
    transaction->commitTS = lastTimestamp;
    try {
        transaction->commit(&wal);
    } catch (...) {
        leaveQueue();
        throw;
    }
    // Le numéro se prend sous le verrou public, la validation faite : un point de reprise
    // commencé après lui la contient.
    const auto ticket = checkpointsStarted;
    leaveQueue();
    clearTransactionNoLock(transaction->getID());
    transactionsChanged.wait(publicLock, [this, ticket] {
        return lastCompletedCheckpoint > ticket || checkpointFailed || !checkpointPending;
    });
    throwIfCheckpointFailedNoLock();
    if (lastCompletedCheckpoint <= ticket) {
        // Le point de reprise qui devait la contenir n'a pas eu lieu (la validation qui le
        // portait a échoué après l'attente) : cette validation, faite en mémoire, l'écrit elle-même.
        try {
            checkpointNoLock(clientContext, publicLock);
        } catch (const CheckpointException& e) {
            throw TransactionManagerException(common::stringFormat(
                "This transaction is committed in memory, but the checkpoint that makes it durable "
                "could not be written ({}); it becomes durable at the next checkpoint.",
                e.what()));
        }
    }
    return true;
}

void TransactionManager::postponeCheckpointNoLock() {
    // Une ligne au premier report, puis chaque fois que le journal a doublé : sous un lecteur
    // long, chaque validation reporte, et une ligne par validation noierait les journaux.
    const auto walSize = wal.getFileSize();
    if (walSizeAtLastPostponeWarning == 0 || walSize >= 2 * walSizeAtLastPostponeWarning) {
        fprintf(stderr, "[%s] the journal holds %llu bytes\n", CHECKPOINT_POSTPONED,
            static_cast<unsigned long long>(walSize));
        walSizeAtLastPostponeWarning = std::max<uint64_t>(walSize, 1);
    }
}

void TransactionManager::allowNewTransactionsNoLock() {
    checkpointPending = false;
    transactionsChanged.notify_all();
}

bool TransactionManager::hasNoActiveTransactions() const {
    return activeTransactions.empty();
}

bool TransactionManager::hasActiveWriteTransactionNoLock() const {
    return std::ranges::any_of(activeTransactions,
        [](const auto& transaction) { return transaction->isWriteTransaction(); });
}

void TransactionManager::clearTransactionNoLock(transaction_t transactionID) {
    KU_ASSERT(std::ranges::any_of(activeTransactions.begin(), activeTransactions.end(),
        [transactionID](const auto& activeTransaction) {
            return activeTransaction->getID() == transactionID;
        }));
    std::erase_if(activeTransactions, [transactionID](const auto& activeTransaction) {
        return activeTransaction->getID() == transactionID;
    });
    // La transaction a fini, validée ou annulée : ses verrous sont rendus, et ceux qui les
    // attendaient passent.
    lockManager.releaseAll(transactionID);
    // Un point de reprise attend peut-être ce départ.
    transactionsChanged.notify_all();
}

std::unique_ptr<Checkpointer> TransactionManager::initCheckpointer(
    main::ClientContext& clientContext) {
    return std::make_unique<Checkpointer>(clientContext);
}

void TransactionManager::checkpointNoLock(main::ClientContext& clientContext,
    std::unique_lock<std::mutex>& publicLock, bool newTransactionsAlreadyStopped) {
    // Note: It is enough to stop and wait for transactions to leave the system instead of, for
    // example, checking on the query processor's task scheduler. This is because the
    // first and last steps that a connection performs when executing a query are to
    // start and commit/rollback transaction. The query processor also ensures that it
    // will only return results or error after all threads working on the tasks of a
    // query stop working on the tasks of the query and these tasks are removed from the
    // query.
    if (!newTransactionsAlreadyStopped) {
        try {
            stopNewTransactionsAndWaitForOthersNoLock(publicLock, nullptr);
        } catch (std::exception& e) {
            throw CheckpointException{e};
        }
    }
    // Le démarrage reste interdit jusqu'à la fin de l'écriture, quelle qu'en soit l'issue.
    auto checkpointer = initCheckpointerFunc(clientContext);
    const auto checkpointNumber = ++checkpointsStarted;
    try {
        checkpointer->writeCheckpoint();
        walSizeAtLastPostponeWarning = 0;
        lastCompletedCheckpoint = checkpointNumber;
        allowNewTransactionsNoLock();
    } catch (std::exception& e) {
        allowNewTransactionsNoLock();
        // Nothing is served after this, see checkpointFailed. A timeout while waiting for the
        // transactions to leave, above, wrote nothing and does not come here.
        checkpointFailed = true;
        checkpointer->rollback();
        // The caller must be able to tell this error from an ordinary one: it carries the same
        // name as the refusals that follow, after its cause.
        throw CheckpointException{RuntimeException(common::stringFormat(
            "{} {}: the checkpoint did not complete. Transactions already written to the "
            "journal are replayed when the database is reopened; a transaction whose durability "
            "is its own checkpoint (it holds a COPY FROM outside the journal, or creates an "
            "index) is not: nothing of it remains, and it must be run again.",
            e.what(), REOPEN_AFTER_FAILED_CHECKPOINT))};
    }
}

} // namespace transaction
} // namespace rag3db
