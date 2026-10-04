#pragma once

#include <condition_variable>
#include <memory>
#include <mutex>

#include "common/constants.h"
#include "common/uniq_lock.h"
#include "storage/checkpointer.h"
#include "storage/wal/wal.h"
#include "transaction/transaction.h"

namespace rag3db {
namespace main {
class ClientContext;
} // namespace main

namespace testing {
class DBTest;
class FlakyBufferManager;
class FlakyCheckpointer;
} // namespace testing

namespace transaction {

class TransactionManager {
    friend class testing::DBTest;
    friend class testing::FlakyBufferManager;
    friend class testing::FlakyCheckpointer;

    using init_checkpointer_func_t =
        std::function<std::unique_ptr<storage::Checkpointer>(main::ClientContext&)>;
    static std::unique_ptr<storage::Checkpointer> initCheckpointer(
        main::ClientContext& clientContext);

public:
    // Timestamp starts from 1. 0 is reserved for the dummy system transaction.
    explicit TransactionManager(storage::WAL& wal)
        : wal{wal}, lastTransactionID{Transaction::START_TRANSACTION_ID}, lastTimestamp{1} {
        initCheckpointerFunc = initCheckpointer;
    }

    Transaction* beginTransaction(main::ClientContext& clientContext, TransactionType type);

    void commit(main::ClientContext& clientContext, Transaction* transaction);
    void rollback(main::ClientContext& clientContext, Transaction* transaction);

    void checkpoint(main::ClientContext& clientContext);

    static TransactionManager* Get(const main::ClientContext& context);

    // The start of the error that every statement gets once a checkpoint of this database has
    // failed while writing, until the database is closed and reopened. See checkpointNoLock.
    // L'avertissement d'un point de reprise automatique reporté parce qu'une autre transaction
    // était ouverte à la validation (écrit sur stderr).
    static constexpr const char* CHECKPOINT_POSTPONED =
        "Checkpoint postponed: other transactions are open";
    static constexpr const char* REOPEN_AFTER_FAILED_CHECKPOINT =
        "A checkpoint of this database failed, so it must be closed and reopened before it is "
        "used again";

private:
    bool hasNoActiveTransactions() const;
    // Écrit un point de reprise ; publicLock est le verrou public, tenu par l'appelant. Sauf si
    // newTransactionsAlreadyStopped, attend d'abord que toutes les transactions partent.
    void checkpointNoLock(main::ClientContext& clientContext,
        std::unique_lock<std::mutex>& publicLock, bool newTransactionsAlreadyStopped = false);

    // Interdit le démarrage de nouvelles transactions et attend que toutes les autres que
    // `staying` partent, en relâchant le verrou public pendant l'attente : une transaction ne
    // peut partir (commit, rollback) qu'en le prenant. Lève une TransactionManagerException si
    // le délai expire, et rouvre alors le démarrage.
    void stopNewTransactionsAndWaitForOthersNoLock(std::unique_lock<std::mutex>& publicLock,
        const Transaction* staying);
    void allowNewTransactionsNoLock();
    // Un point de reprise automatique reporté, parce qu'une autre transaction est ouverte.
    void postponeCheckpointNoLock();

    bool hasActiveWriteTransactionNoLock() const;

    // Note: Used by DBTest::createDB only.
    void setCheckPointWaitTimeoutForTransactionsToLeaveInMicros(uint64_t waitTimeInMicros) {
        checkpointWaitTimeoutInMicros = waitTimeInMicros;
    }

    void clearTransactionNoLock(common::transaction_t transactionID);

private:
    storage::WAL& wal;
    std::vector<std::unique_ptr<Transaction>> activeTransactions;
    common::transaction_t lastTransactionID;
    common::transaction_t lastTimestamp;
    // Une seule fonction publique à la fois, sauf pendant l'attente d'un point de reprise, qui
    // relâche ce verrou pour que les transactions ouvertes puissent valider ou annuler.
    std::mutex mtxForSerializingPublicFunctionCalls;
    // Vrai tant qu'un point de reprise attend le départ des transactions ou s'écrit : aucune
    // transaction ne démarre. Réveils par transactionsChanged. Gardé par le verrou public.
    bool checkpointPending = false;
    std::condition_variable transactionsChanged;
    // La taille du journal au dernier avertissement de report ; 0 après un point de reprise.
    uint64_t walSizeAtLastPostponeWarning = 0;
    uint64_t checkpointWaitTimeoutInMicros = common::DEFAULT_CHECKPOINT_WAIT_TIMEOUT_IN_MICROS;

    init_checkpointer_func_t initCheckpointerFunc;

    // Set when a checkpoint failed while writing. From then on what this process holds in memory
    // no longer matches the files: the failed checkpoint already advanced the in-memory
    // structures, and what it wrote to the shadow file is read by no ordinary transaction.
    // Serving anything would read wrong data, and a later checkpoint would make it durable. So
    // every transaction and every checkpoint, including the one on close, is refused until the
    // database is reopened: the reopening replays the journal over the data file of the last
    // complete checkpoint, which is sound. Guarded by mtxForSerializingPublicFunctionCalls.
    bool checkpointFailed = false;
    void throwIfCheckpointFailedNoLock() const;
};
} // namespace transaction
} // namespace rag3db
