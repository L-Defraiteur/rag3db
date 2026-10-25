#pragma once

#include <chrono>
#include <condition_variable>
#include <memory>
#include <mutex>

#include "common/constants.h"
#include "common/uniq_lock.h"
#include "storage/checkpointer.h"
#include "storage/wal/wal.h"
#include "transaction/lock_manager.h"
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
    // Attend qu'aucun point de reprise ne retienne les nouvelles transactions, dans le fil de
    // l'appelant : un BEGIN explicite s'exécute sur un fil du planificateur, et y attendre
    // tiendrait un fil dont les transactions ouvertes ont besoin pour valider et partir (A8).
    // Un lecteur n'attend que la porte « tout le monde » (voir waitWhileCheckpointPendingNoLock).
    void waitUntilNewTransactionsAreAllowed(bool readOnly);
    void rollback(main::ClientContext& clientContext, Transaction* transaction);
    // Vrai tant que la transaction est dans le système. Une validation dont le point de reprise
    // qui suit échoue lève après l'avoir retirée (et détruite) : la connexion le demande avant
    // d'annuler ce qu'elle tient encore.
    bool isActive(common::transaction_t transactionID);
    // Marche T0 : une instruction a échoué dans une transaction explicite. Ses effets sont
    // annulés, elle reste active, ses verrous tenus ; rollback() la ferme ensuite.
    void failAfterStatement(main::ClientContext& clientContext, Transaction* transaction);

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
    // Le même refus quand une annulation a levé au milieu du stockage partagé (rollbackFailed).
    static constexpr const char* REOPEN_AFTER_FAILED_ROLLBACK =
        "A rollback of this database could not complete, so it must be closed and reopened "
        "before it is used again";

    // Les verrous des écritures parallèles. Une transaction les prend pendant ses écritures ;
    // ils sont rendus quand elle quitte le système, validée ou annulée.
    LockManager& getLockManager() { return lockManager; }
    // L'instantané du point de reprise en cours (V2-2), posé par checkpointNoLock sous le verrou
    // public au départ de l'écriture : T, tout ce qui est validé ; F, le plus vieux instantané
    // des lecteurs restés ouverts (T s'il n'y en a pas). Ce qui est validé à F se fond, le reste
    // se porte en versions. Hors point de reprise : le dernier validé (tout se fond).
    common::transaction_t getCheckpointFoldTS() const { return checkpointFoldTS; }
    common::transaction_t getCheckpointSnapshotTS() const { return checkpointSnapshotTS; }
    // Marche V2 : l'instantané d'une transaction qui n'a rien écrit est repris au dernier
    // horodatage de validation — après l'attente de ses verrous annoncés, elle lit l'état
    // d'après l'attente, et n'échoue jamais à la sérialisation sur ce qu'elle a annoncé.
    void refreshSnapshot(Transaction* transaction);

private:
    bool hasNoActiveTransactions() const;
    // Écrit un point de reprise ; publicLock est le verrou public, tenu par l'appelant. Sauf si
    // newTransactionsAlreadyStopped, attend d'abord que toutes les transactions partent.
    void checkpointNoLock(main::ClientContext& clientContext,
        std::unique_lock<std::mutex>& publicLock, bool newTransactionsAlreadyStopped = false);

    // Interdit le démarrage de nouvelles transactions et attend que toutes les autres que
    // `staying` partent, en relâchant le verrou public pendant l'attente : une transaction ne
    // peut partir (commit, rollback) qu'en le prenant. Les validations rangées dans la file
    // (A7) ne bloquent pas : quand il ne reste qu'elles, elles valident à leur tour, et le point
    // de reprise qui suit les contient. Lève une TransactionManagerException si le délai
    // expire, et rouvre alors le démarrage.
    // readersToo : une validation forcée ou une action d'avant validation attend tout le
    // monde ; un point de reprise ordinaire n'attend que les écrivains (V2-2 : les lecteurs
    // gardent leur instantané, le point de reprise fond jusqu'au plus vieux d'entre eux).
    void stopNewTransactionsAndWaitForOthersNoLock(const main::ClientContext* clientContext,
        std::unique_lock<std::mutex>& publicLock, const Transaction* staying, bool readersToo);
    // A7 : une validation qui veut un point de reprise alors qu'un autre attend se range dans la
    // file au lieu d'attendre la fin du premier. Rend vrai si elle a validé et qu'un point de
    // reprise commencé après sa validation s'est écrit ; faux si le point de reprise en attente
    // a renoncé avant qu'elle valide (rien n'est validé : elle prend alors sa place en tête).
    bool commitInCheckpointQueueNoLock(main::ClientContext& clientContext,
        std::unique_lock<std::mutex>& publicLock, Transaction* transaction);
    // A8 : le dernier à partir écrit le point de reprise automatique demandé. Appelé après le
    // départ d'une transaction validée ; une annulation n'écrit pas (elle peut venir de la
    // fermeture d'une connexion), elle renonce à la demande (cancelAutoCheckpointRequestNoLock).
    void writeRequestedCheckpointIfLastNoLock(main::ClientContext& clientContext,
        std::unique_lock<std::mutex>& publicLock);
    void cancelAutoCheckpointRequestNoLock();
    // La porte à deux niveaux (V2-1c). checkpointPending retient les écrivains du début de
    // l'attente des départs à la fin de l'écriture ; checkpointWriting dit que l'écriture est en
    // cours, hors du verrou public : un lecteur nouveau entre alors, commence, valide et annule
    // sans attendre. Il ne lit que ce que le point de reprise écrit (le dernier validé, aucun
    // écrivain ne tourne) et tient ses blocs (NodeGroupScanState::groups).
    void waitWhileCheckpointPendingNoLock(const main::ClientContext* clientContext,
        std::unique_lock<std::mutex>& publicLock, bool readOnly);
    void allowNewTransactionsNoLock();
    // Un point de reprise automatique reporté, parce qu'une autre transaction est ouverte.
    void postponeCheckpointNoLock();

    bool hasActiveWriteTransactionNoLock() const;
    // Le plus ancien instantané des transactions ouvertes autres que celle qui valide, ou son
    // horodatage de validation si elle est seule (Transaction::getOldestActiveStartTS).
    common::transaction_t oldestActiveSnapshotNoLock(const Transaction* committing) const;

    // Note: Used by DBTest::createDB only.
    void setCheckPointWaitTimeoutForTransactionsToLeaveInMicros(uint64_t waitTimeInMicros) {
        checkpointWaitTimeoutInMicros = waitTimeInMicros;
    }

    void clearTransactionNoLock(common::transaction_t transactionID);

private:
    storage::WAL& wal;
    std::vector<std::unique_ptr<Transaction>> activeTransactions;
    LockManager lockManager;
    common::transaction_t lastTransactionID;
    common::transaction_t lastTimestamp;
    // Une seule fonction publique à la fois, sauf pendant l'attente d'un point de reprise, qui
    // relâche ce verrou pour que les transactions ouvertes puissent valider ou annuler.
    std::mutex mtxForSerializingPublicFunctionCalls;
    // Vrai tant qu'un point de reprise attend le départ des transactions ou s'écrit : aucun
    // écrivain ne démarre. Réveils par transactionsChanged. Gardé par le verrou public.
    bool checkpointPending = false;
    common::transaction_t checkpointFoldTS = Transaction::LATEST_COMMITTED_TS;
    common::transaction_t checkpointSnapshotTS = Transaction::LATEST_COMMITTED_TS;
    // Vrai pendant l'écriture elle-même, hors du verrou public : les lecteurs entrent (V2-1c).
    bool checkpointWriting = false;
    std::condition_variable transactionsChanged;
    // A7, la file des points de reprise. Les validations qui veulent un point de reprise et en
    // trouvent un en attente ; quand il ne reste qu'elles, le point de reprise en attente pose
    // `commitQueuedNow` et elles valident en mémoire, puis attendent un point de reprise commencé
    // après leur validation (`checkpointsStarted` au moment de valider, comparé à
    // `lastCompletedCheckpoint`). Gardé par le verrou public.
    std::vector<const Transaction*> queuedCheckpointCommits;
    bool commitQueuedNow = false;
    // A8, le point de reprise automatique sous des écrivains qui se succèdent : la validation qui
    // atteint le seuil, s'il reste d'autres transactions ouvertes, pose la demande
    // (`checkpointPending` retient les nouvelles transactions) et rend la main ; le dernier à
    // partir écrit. Un BEGIN retenu plus de AUTO_CHECKPOINT_WAIT renonce à la demande, et le
    // point de reprise est reporté comme avant. Gardé par le verrou public.
    bool autoCheckpointRequested = false;
    std::chrono::steady_clock::time_point autoCheckpointRequestedAt;
    static constexpr std::chrono::milliseconds AUTO_CHECKPOINT_WAIT{250};
    // Une demande abandonnée (une transaction est restée au-delà de la borne, un lecteur long) :
    // le plus grand identifiant des transactions qui l'ont tenue. Aucune nouvelle demande tant
    // qu'une transaction d'identifiant inférieur ou égal reste ouverte : sous un lecteur long,
    // les écrivains n'attendent qu'une fois, puis le point de reprise est reporté comme avant,
    // jusqu'à son départ.
    common::transaction_t lastBlockingTransactionID = 0;
    uint64_t checkpointsStarted = 0;
    uint64_t lastCompletedCheckpoint = 0;
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
    // Une annulation a levé après avoir commencé de défaire le stockage partagé ou le catalogue
    // (Transaction::hasHalfDoneRollback) : des versions marquées d'une transaction qui n'existe
    // plus, des clés laissées dans l'index de clé primaire. La base est refusée comme après un
    // point de reprise raté ; à la réouverture, le journal ne porte rien de cette transaction.
    // Une annulation qui n'a levé que dans le stockage local de la transaction ne la condamne
    // pas. Gardé par mtxForSerializingPublicFunctionCalls.
    bool rollbackFailed = false;
    // Le prochain point de reprise porte une validation forcée, faite en mémoire et durable par
    // lui seul (la validation forcée elle-même, ou une validation de la file A7 pendant son
    // attente). Lu et remis à faux par checkpointNoLock. Gardé par
    // mtxForSerializingPublicFunctionCalls.
    bool checkpointCarriesForcedCommit = false;
    void throwIfCheckpointFailedNoLock() const;
};
} // namespace transaction
} // namespace rag3db
