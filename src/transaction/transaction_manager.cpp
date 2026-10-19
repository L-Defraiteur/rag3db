#include "transaction/transaction_manager.h"

#include <algorithm>
#include <cstdio>
#include <optional>

#include "storage/checkpoint_profile.h"

#include <thread>

#include "common/exception/checkpoint.h"
#include "common/exception/runtime.h"
#include "common/exception/transaction_manager.h"
#include "common/string_format.h"
#include "common/task_system/task_scheduler.h"
#include "main/attached_database.h"
#include "main/client_context.h"
#include "main/database.h"
#include "main/db_config.h"
#include "storage/checkpointer.h"
#include "storage/storage_manager.h"
#include "storage/wal/local_wal.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace rag3db {
namespace transaction {

namespace {

// Le CHECKPOINT et le COMMIT explicites s'exécutent sur un fil du planificateur (l'opérateur
// Transaction). Une attente du gestionnaire sur un tel fil le rend à la file pendant qu'elle dure
// (un remplaçant la sert), comme l'attente des verrous (A3′) : sinon deux attentes prennent les
// deux fils du banc, et la transaction attendue ne trouve plus de fil pour valider ; tout reste
// figé jusqu'au délai du point de reprise (page coeur-cpp-tickets/06, « ce qui reste »). Sans
// effet hors d'un fil ouvrier. On ne cède que si l'attente est réelle (son prédicat encore faux),
// et une seule fois par fonction : la portée survit à l'attente jusqu'à la fin de la fonction.
// Le fil ouvrier finit donc ses dernières instructions avec un remplaçant en plus ; c'est voulu,
// et borné à la fonction.
void yieldWorkerWhileWaiting(std::optional<common::TaskScheduler::BlockingWait>& blocking,
    const main::ClientContext* clientContext) {
    if (blocking.has_value() || clientContext == nullptr ||
        !common::TaskScheduler::onWorkerThread()) {
        return;
    }
    blocking.emplace(common::TaskScheduler::Get(*clientContext));
}

} // namespace

Transaction* TransactionManager::beginTransaction(main::ClientContext& clientContext,
    TransactionType type) {
    // We acquire the lock for starting new transactions. In case this cannot be acquired, this
    // ensures calls to other public functions are not restricted.
    std::unique_lock publicFunctionLck{mtxForSerializingPublicFunctionCalls};
    // Pas de nouvelle transaction pendant qu'un point de reprise attend ou s'écrit.
    waitWhileCheckpointPendingNoLock(&clientContext, publicFunctionLck);
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
    transaction->snapshotFloorTS = lastTimestamp;
}

transaction_t TransactionManager::oldestActiveSnapshotNoLock(
    const Transaction* committing) const {
    auto oldest = committing->commitTS;
    for (const auto& active : activeTransactions) {
        if (active.get() != committing && active->snapshotFloorTS < oldest) {
            oldest = active->snapshotFloorTS;
        }
    }
    return oldest;
}

void TransactionManager::commit(main::ClientContext& clientContext, Transaction* transaction) {
    std::unique_lock lck{mtxForSerializingPublicFunctionCalls};
    clientContext.cleanUp();
    switch (transaction->getType()) {
    case TransactionType::READ_ONLY: {
        clearTransactionNoLock(transaction->getID());
        writeRequestedCheckpointIfLastNoLock(clientContext, lck);
    } break;
    case TransactionType::RECOVERY:
    case TransactionType::WRITE: {
        const auto forced = transaction->shouldForceCheckpoint();
        if (forced && autoCheckpointRequested) {
            // Une demande automatique attend le départ des autres : le point de reprise forcé de
            // cette validation la remplit, par le chemin ordinaire (il attend les autres, borné).
            // Entre ce rouvrir et la reprise du blocage par stopNewTransactions…, quelques BEGIN
            // peuvent entrer : sans danger, le point de reprise forcé les attend comme les autres.
            autoCheckpointRequested = false;
            allowNewTransactionsNoLock();
        }
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
            stopNewTransactionsAndWaitForOthersNoLock(&clientContext, lck, transaction);
        }
        // Une action d'avant validation (Transaction::addPreCommitAction) lit ce que les autres
        // ont validé : elle veut qu'ils soient tous partis. Une validation forcée les a déjà
        // attendus ; sans point de reprise (base en mémoire), on les attend de même, sous le
        // même délai, et les BEGIN reprennent dès la validation faite.
        const auto waitsWithoutCheckpoint = !forced && transaction->hasPreCommitActions();
        if (waitsWithoutCheckpoint) {
            stopNewTransactionsAndWaitForOthersNoLock(&clientContext, lck, transaction);
        }
        lastTimestamp++;
        transaction->commitTS = lastTimestamp;
        transaction->oldestActiveStartTS = oldestActiveSnapshotNoLock(transaction);
        storage::CheckpointProfile profile;
        try {
            transaction->commit(&wal);
        } catch (...) {
            if (forced || waitsWithoutCheckpoint) {
                allowNewTransactionsNoLock();
            }
            throw;
        }
        const auto commitMs = profile.lap();
        auto shouldCheckpoint =
            forced || Checkpointer::canAutoCheckpoint(clientContext, *transaction);
        clearTransactionNoLock(transaction->getID());
        if (waitsWithoutCheckpoint) {
            allowNewTransactionsNoLock();
        }
        if (forced) {
            checkpointCarriesForcedCommit = true;
            checkpointNoLock(clientContext, lck, true /* newTransactionsAlreadyStopped */);
        } else if (shouldCheckpoint) {
            if (checkpointPending) {
                // Un point de reprise est déjà demandé ou en attente : il contiendra celle-ci.
                postponeCheckpointNoLock();
            } else if (activeTransactions.empty()) {
                checkpointNoLock(clientContext, lck);
            } else if (std::ranges::any_of(activeTransactions,
                           [this](const auto& active) {
                               return active->getID() <= lastBlockingTransactionID;
                           })) {
                // Une transaction qui a déjà fait abandonner une demande est encore ouverte (un
                // lecteur long) : reporté, sans retenir de nouveau les écrivains.
                postponeCheckpointNoLock();
            } else {
                // A8 : d'autres transactions sont ouvertes. La validation est faite et au
                // journal ; elle pose la demande et rend la main, sans attendre (attendre ici
                // tiendrait un fil du planificateur dont les autres ont besoin pour partir). Les
                // nouvelles transactions sont retenues ; le dernier à partir écrit.
                checkpointPending = true;
                autoCheckpointRequested = true;
                autoCheckpointRequestedAt = std::chrono::steady_clock::now();
            }
        }
        // Une demande posée par une autre validation : si celle-ci part la dernière, elle écrit.
        writeRequestedCheckpointIfLastNoLock(clientContext, lck);
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
        try {
            transaction->failAfterStatement(&wal);
        } catch (const std::exception& e) {
            // La transaction est marquée échouée : le ROLLBACK qui ferme le bloc la retire sans
            // rejouer l'annulation (TransactionManager::rollback).
            const auto halfDone = transaction->hasHalfDoneRollback();
            if (halfDone) {
                rollbackFailed = true;
            }
            throw TransactionManagerException(common::stringFormat(
                "The rollback of the failed statement could not complete ({}). Send ROLLBACK to "
                "close the transaction{}",
                e.what(),
                halfDone ? std::string{". "} + REOPEN_AFTER_FAILED_ROLLBACK + "." : "."));
        }
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
        // La transaction quitte le système même si son annulation lève : restée active, elle
        // gardait ses verrous, et tout le monde l'attendait, BEGIN et CHECKPOINT compris (ticket
        // du ROLLBACK sous un tampon plein). L'erreur dit ce qu'il en est.
        std::optional<std::string> failure;
        if (!transaction->hasFailed()) {
            try {
                transaction->rollback(&wal);
            } catch (const std::exception& e) {
                failure = e.what();
            }
        }
        const auto halfDone = failure && transaction->hasHalfDoneRollback();
        if (halfDone) {
            rollbackFailed = true;
        }
        clearTransactionNoLock(transaction->getID());
        if (failure) {
            if (autoCheckpointRequested && activeTransactions.empty()) {
                cancelAutoCheckpointRequestNoLock();
            }
            throw TransactionManagerException(common::stringFormat(
                "The rollback could not complete ({}). The transaction is closed and its locks "
                "are released{}",
                *failure,
                halfDone ? std::string{". "} + REOPEN_AFTER_FAILED_ROLLBACK + "." : "."));
        }
    } break;
    default: {
        throw TransactionManagerException("Invalid transaction type to rollback.");
    }
    }
    // A8 : une annulation peut venir de la fermeture d'une connexion ou de la base ; elle n'écrit
    // pas de point de reprise. Partie la dernière, elle renonce à la demande, et le point de
    // reprise est reporté à une validation suivante.
    if (autoCheckpointRequested && activeTransactions.empty()) {
        cancelAutoCheckpointRequestNoLock();
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
    if (rollbackFailed) {
        throw TransactionManagerException(common::stringFormat(
            "{}: what it holds in memory no longer matches anything committed or rolled back. No "
            "committed transaction is lost: the journal is replayed when the database is "
            "reopened.",
            REOPEN_AFTER_FAILED_ROLLBACK));
    }
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
    const main::ClientContext* clientContext, std::unique_lock<std::mutex>& publicLock,
    const Transaction* staying) {
    std::optional<common::TaskScheduler::BlockingWait> blocking;
    // Un seul point de reprise attend à la fois ; une validation qui en veut un pendant ce temps
    // se range dans la file (commitInCheckpointQueueNoLock) au lieu d'attendre ici. Une demande
    // automatique en attente (A8) est reprise par ce point de reprise, qui la remplit.
    if (autoCheckpointRequested) {
        autoCheckpointRequested = false;
    } else {
        if (checkpointPending) {
            yieldWorkerWhileWaiting(blocking, clientContext);
        }
        transactionsChanged.wait(publicLock, [this] { return !checkpointPending; });
    }
    checkpointPending = true;
    const auto othersLeftOrQueued = [this, staying] {
        return std::ranges::all_of(activeTransactions, [this, staying](const auto& transaction) {
            return transaction.get() == staying ||
                   std::ranges::find(queuedCheckpointCommits, transaction.get()) !=
                       queuedCheckpointCommits.end();
        });
    };
    // wait_for relâche le verrou public : les transactions ouvertes peuvent valider ou annuler.
    if (!othersLeftOrQueued()) {
        yieldWorkerWhileWaiting(blocking, clientContext);
    }
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
        yieldWorkerWhileWaiting(blocking, clientContext);
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
    std::optional<common::TaskScheduler::BlockingWait> blocking;
    queuedCheckpointCommits.push_back(transaction);
    const auto leaveQueue = [this, transaction] {
        std::erase(queuedCheckpointCommits, transaction);
        transactionsChanged.notify_all();
    };
    // Le point de reprise en attente réévalue : cette transaction ne le bloque plus.
    transactionsChanged.notify_all();
    if (!(commitQueuedNow || !checkpointPending || checkpointFailed)) {
        yieldWorkerWhileWaiting(blocking, &clientContext);
    }
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
    transaction->oldestActiveStartTS = oldestActiveSnapshotNoLock(transaction);
    try {
        transaction->commit(&wal);
        checkpointCarriesForcedCommit = true;
    } catch (...) {
        leaveQueue();
        throw;
    }
    // Le numéro se prend sous le verrou public, la validation faite : un point de reprise
    // commencé après lui la contient.
    const auto ticket = checkpointsStarted;
    leaveQueue();
    clearTransactionNoLock(transaction->getID());
    const auto checkpointDoneOrGivenUp = [this, ticket] {
        return lastCompletedCheckpoint > ticket || checkpointFailed || !checkpointPending;
    };
    if (!checkpointDoneOrGivenUp()) {
        yieldWorkerWhileWaiting(blocking, &clientContext);
    }
    transactionsChanged.wait(publicLock, checkpointDoneOrGivenUp);
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

void TransactionManager::waitWhileCheckpointPendingNoLock(const main::ClientContext* clientContext,
    std::unique_lock<std::mutex>& publicLock) {
    std::optional<common::TaskScheduler::BlockingWait> blocking;
    if (checkpointPending) {
        yieldWorkerWhileWaiting(blocking, clientContext);
    }
    // Une demande automatique (A8) ne retient pas plus de AUTO_CHECKPOINT_WAIT : au-delà, elle est
    // abandonnée, et le point de reprise reporté (un écrivain long la tient).
    while (checkpointPending) {
        if (autoCheckpointRequested) {
            const auto deadline = autoCheckpointRequestedAt + AUTO_CHECKPOINT_WAIT;
            if (transactionsChanged.wait_until(publicLock, deadline) == std::cv_status::timeout &&
                autoCheckpointRequested && std::chrono::steady_clock::now() >= deadline) {
                cancelAutoCheckpointRequestNoLock();
            }
        } else {
            transactionsChanged.wait(publicLock);
        }
    }
}

void TransactionManager::waitUntilNewTransactionsAreAllowed() {
    std::unique_lock lck{mtxForSerializingPublicFunctionCalls};
    waitWhileCheckpointPendingNoLock(nullptr, lck);
}

void TransactionManager::writeRequestedCheckpointIfLastNoLock(main::ClientContext& clientContext,
    std::unique_lock<std::mutex>& publicLock) {
    if (!autoCheckpointRequested || !activeTransactions.empty()) {
        return;
    }
    autoCheckpointRequested = false;
    try {
        checkpointNoLock(clientContext, publicLock, true /* newTransactionsAlreadyStopped */);
    } catch (const CheckpointException& e) {
        // La validation de cette transaction est faite et au journal : c'est le point de reprise
        // d'après, qu'elle écrivait en partant la dernière, qui a échoué. Le dire, sinon le
        // client croit sa validation perdue.
        throw TransactionManagerException(common::stringFormat(
            "This transaction is committed (it is in the journal); the automatic checkpoint that "
            "followed it, written by the last transaction to leave, failed: {}",
            e.what()));
    }
}

void TransactionManager::cancelAutoCheckpointRequestNoLock() {
    autoCheckpointRequested = false;
    for (const auto& active : activeTransactions) {
        lastBlockingTransactionID = std::max(lastBlockingTransactionID, active->getID());
    }
    allowNewTransactionsNoLock();
    postponeCheckpointNoLock();
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

bool TransactionManager::isActive(transaction_t transactionID) {
    std::unique_lock lck{mtxForSerializingPublicFunctionCalls};
    return std::ranges::any_of(activeTransactions, [transactionID](const auto& transaction) {
        return transaction->getID() == transactionID;
    });
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
            stopNewTransactionsAndWaitForOthersNoLock(&clientContext, publicLock, nullptr);
        } catch (std::exception& e) {
            throw CheckpointException{e};
        }
    }
    // Le démarrage reste interdit jusqu'à la fin de l'écriture, quelle qu'en soit l'issue.
    auto checkpointer = initCheckpointerFunc(clientContext);
    const auto checkpointNumber = ++checkpointsStarted;
    // Une validation forcée (un COPY hors journal, la création d'un index) n'est durable que par
    // ce point de reprise : son échec, quel qu'il soit, condamne la base (voir plus bas).
    const auto carriesForcedCommit = checkpointCarriesForcedCommit;
    checkpointCarriesForcedCommit = false;
    try {
        checkpointer->writeCheckpoint();
        walSizeAtLastPostponeWarning = 0;
        lastCompletedCheckpoint = checkpointNumber;
        allowNewTransactionsNoLock();
    } catch (std::exception& e) {
        allowNewTransactionsNoLock();
        if (!carriesForcedCommit &&
            !storage::StorageManager::Get(clientContext)->hasCheckpointChangedMemory()) {
            // Refusé avant d'avoir rien changé de ce qui est servi (le plus souvent : la copie
            // fusionnée d'un groupe, sous un tampon plein) : Checkpointer::rollback rend les
            // pages qu'il a prises, et la base reste ouverte. Tout ce qui est validé est au
            // journal (ticket du ROLLBACK sous un tampon plein, défaut 3).
            checkpointer->rollback();
            throw CheckpointException{RuntimeException(common::stringFormat(
                "{}: the checkpoint was refused before it changed anything. Every committed "
                "transaction is in the journal, and the database stays open.",
                e.what()))};
        }
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
