#pragma once

#include <atomic>
#include <mutex>
#include <span>

#include "common/types/types.h"

namespace rag3db {
namespace binder {
struct BoundAlterInfo;
}
namespace catalog {
class CatalogEntry;
class CatalogSet;
class SequenceCatalogEntry;
struct SequenceRollbackData;
} // namespace catalog
namespace main {
class ClientContext;
} // namespace main
namespace storage {
class LocalWAL;
class LocalStorage;
class UndoBuffer;
class WAL;
class VersionInfo;
class UpdateInfo;
struct VectorUpdateInfo;
class ChunkedNodeGroup;
class VersionRecordHandler;
} // namespace storage
namespace transaction {
class TransactionManager;

enum class TransactionType : uint8_t { READ_ONLY, WRITE, CHECKPOINT, DUMMY, RECOVERY };

class LocalCacheManager;
class RAG3DB_API LocalCacheObject {
public:
    explicit LocalCacheObject(std::string key) : key{std::move(key)} {}

    virtual ~LocalCacheObject() = default;

    std::string getKey() const { return key; }

    template<typename T>
    T* cast() {
        return common::ku_dynamic_cast<T*>(this);
    }

private:
    std::string key;
};

class LocalCacheManager {
public:
    bool contains(const std::string& key) {
        std::unique_lock lck{mtx};
        return cachedObjects.contains(key);
    }
    LocalCacheObject& at(const std::string& key) {
        std::unique_lock lck{mtx};
        return *cachedObjects.at(key);
    }
    bool put(std::unique_ptr<LocalCacheObject> object);

    void remove(const std::string& key) {
        std::unique_lock lck{mtx};
        cachedObjects.erase(key);
    }

private:
    std::unordered_map<std::string, std::unique_ptr<LocalCacheObject>> cachedObjects;
    std::mutex mtx;
};

struct LockRequest;
struct LockResource;
enum class LockMode : uint8_t;

class RAG3DB_API Transaction {
    friend class TransactionManager;

public:
    static constexpr common::transaction_t DUMMY_TRANSACTION_ID = 0;
    static constexpr common::transaction_t DUMMY_START_TIMESTAMP = 0;
    static constexpr common::transaction_t START_TRANSACTION_ID =
        static_cast<common::transaction_t>(1) << 63;
    // L'instantané qui voit toute ligne validée, quel que soit l'instant de sa validation : un
    // horodatage de validation est toujours inférieur au premier identifiant de transaction.
    // Le contrôle d'unicité d'une clé primaire regarde le dernier état validé avec lui, et non
    // l'instantané de la transaction (marche A3′, NodeTable::isVisibleToLatestCommit).
    static constexpr common::transaction_t LATEST_COMMITTED_TS = START_TRANSACTION_ID - 1;

    Transaction(main::ClientContext& clientContext, TransactionType transactionType,
        common::transaction_t transactionID, common::transaction_t startTS);

    explicit Transaction(TransactionType transactionType) noexcept;
    Transaction(TransactionType transactionType, common::transaction_t ID,
        common::transaction_t startTS) noexcept;

    ~Transaction();

    TransactionType getType() const { return type; }
    bool isReadOnly() const { return TransactionType::READ_ONLY == type; }
    bool isWriteTransaction() const { return TransactionType::WRITE == type; }
    bool isDummy() const { return TransactionType::DUMMY == type; }
    bool isRecovery() const { return TransactionType::RECOVERY == type; }
    common::transaction_t getID() const { return ID; }
    common::transaction_t getStartTS() const { return startTS; }
    common::transaction_t getCommitTS() const { return commitTS; }
    int64_t getCurrentTS() const { return currentTS; }

    // La transaction devient forcée : durable par le seul point de reprise que sa validation
    // impose, entière ou pas du tout (Transaction::commit). Elle n'écrira rien au fichier du
    // journal : son journal en mémoire est vidé sur-le-champ, et plus rien n'y est écrit. Sur
    // une base en mémoire rien n'est forcé (shouldForceCheckpoint) : le drapeau y est sans effet.
    void setForceCheckpoint();
    // Le repli d'un COPY journalisé dont le journal dépasse le seuil : la transaction devient
    // forcée, et la base le compte.
    uint64_t copyJournalThreshold() const;
    bool journalExceedsCopyThreshold() const;
    void fallBackToForcedCheckpoint();
    bool shouldAppendToUndoBuffer() const {
        // Only write transactions and recovery transactions should append to the undo buffer.
        return isWriteTransaction() || isRecovery();
    }
    bool shouldLogToWAL() const;
    storage::LocalWAL& getLocalWAL() const {
        KU_ASSERT(localWAL);
        return *localWAL;
    }

    bool shouldForceCheckpoint() const;

    void commit(storage::WAL* wal);
    void rollback(storage::WAL* wal);

    storage::LocalStorage* getLocalStorage() const { return localStorage.get(); }
    LocalCacheManager& getLocalCacheManager() { return localCacheManager; }
    bool isUnCommitted(common::table_id_t tableID, common::offset_t nodeOffset) const;
    common::row_idx_t getLocalRowIdx(common::table_id_t tableID,
        common::offset_t nodeOffset) const {
        return nodeOffset - getMinUncommittedNodeOffset(tableID);
    }
    common::offset_t getUncommittedOffset(common::table_id_t tableID,
        common::row_idx_t localRowIdx) const {
        return getMinUncommittedNodeOffset(tableID) + localRowIdx;
    }

    void pushCreateDropCatalogEntry(catalog::CatalogSet& catalogSet,
        catalog::CatalogEntry& catalogEntry, bool isInternal, bool skipLoggingToWAL = false);
    void pushAlterCatalogEntry(catalog::CatalogSet& catalogSet, catalog::CatalogEntry& catalogEntry,
        const binder::BoundAlterInfo& alterInfo);
    void pushSequenceChange(catalog::SequenceCatalogEntry* sequenceEntry, int64_t kCount,
        const catalog::SequenceRollbackData& data);
    void pushInsertInfo(common::node_group_idx_t nodeGroupIdx, common::row_idx_t startRow,
        common::row_idx_t numRows, const storage::VersionRecordHandler* versionRecordHandler) const;
    void pushDeleteInfo(common::node_group_idx_t nodeGroupIdx, common::row_idx_t startRow,
        common::row_idx_t numRows, const storage::VersionRecordHandler* versionRecordHandler) const;
    void pushVectorUpdateInfo(storage::UpdateInfo& updateInfo, common::idx_t vectorIdx,
        storage::VectorUpdateInfo& vectorUpdateInfo, common::transaction_t version) const;

    static Transaction* Get(const main::ClientContext& context);

    main::ClientContext* getClientContext() const { return clientContext; }

    // Les verrous des écritures parallèles (marches A3′, A4′, V2). Une transaction d'écriture
    // ordinaire prend ses verrous sous le mode multi-écrivains ; hors de ce mode, en reprise, ou
    // sans contexte, elle ne prend rien et acquireLocks ne fait rien.
    bool usesLocks() const;
    // Prend les verrous demandés, en attendant s'il le faut : au plus lock_timeout, en regardant
    // l'interruption de la connexion. Sur un interblocage, un délai dépassé ou une interruption,
    // lève l'erreur nommée du gestionnaire (TransactionManagerException) : l'instruction échoue,
    // la transaction est annulée par là, et rend tout ce qu'elle tient.
    void acquireLocks(std::span<const LockRequest> requests) const;
    void acquireLock(const LockResource& resource, LockMode mode) const;

    // Pendant cette portée, la transaction lit le dernier état validé au lieu de son instantané
    // (marche A4′ : après l'attente d'un verrou, ce qu'un autre a validé entre-temps compte —
    // les relations d'un nœud qu'on supprime, par exemple). À n'ouvrir que sur le fil qui
    // exécute une écriture, pour la durée d'un balayage : les lectures du même fil, pendant ce
    // temps, voient aussi le dernier état validé.
    class LatestCommittedView {
    public:
        explicit LatestCommittedView(Transaction& transaction_)
            : transaction{transaction_}, saved{transaction_.startTS} {
            transaction.startTS = LATEST_COMMITTED_TS;
        }
        ~LatestCommittedView() { transaction.startTS = saved; }
        LatestCommittedView(const LatestCommittedView&) = delete;
        LatestCommittedView& operator=(const LatestCommittedView&) = delete;

    private:
        Transaction& transaction;
        common::transaction_t saved;
    };

private:
    common::offset_t getMinUncommittedNodeOffset(common::table_id_t tableID) const;

private:
    TransactionType type;
    common::transaction_t ID;
    common::transaction_t startTS;
    common::transaction_t commitTS;
    int64_t currentTS;
    main::ClientContext* clientContext;
    std::unique_ptr<storage::LocalStorage> localStorage;
    std::unique_ptr<storage::UndoBuffer> undoBuffer;
    std::unique_ptr<storage::LocalWAL> localWAL;
    LocalCacheManager localCacheManager;
    bool forceCheckpoint;
    std::atomic<bool> hasCatalogChanges;
};

// TODO(bmwinger): These shouldn't need to be exported
extern RAG3DB_API Transaction DUMMY_TRANSACTION;
extern RAG3DB_API Transaction DUMMY_CHECKPOINT_TRANSACTION;

} // namespace transaction
} // namespace rag3db
