#include "transaction/transaction.h"

#include "common/exception/runtime.h"
#include "main/client_context.h"
#include "main/db_config.h"
#include "storage/local_storage/local_node_table.h"
#include "storage/local_storage/local_storage.h"
#include "storage/storage_manager.h"
#include "storage/undo_buffer.h"
#include "storage/wal/local_wal.h"
#include "transaction/transaction_context.h"

using namespace rag3db::catalog;

namespace rag3db {
namespace transaction {

bool LocalCacheManager::put(std::unique_ptr<LocalCacheObject> object) {
    std::unique_lock lck{mtx};
    const auto key = object->getKey();
    if (cachedObjects.contains(key)) {
        return false;
    }
    cachedObjects[object->getKey()] = std::move(object);
    return true;
}

Transaction::Transaction(main::ClientContext& clientContext, TransactionType transactionType,
    common::transaction_t transactionID, common::transaction_t startTS)
    : type{transactionType}, ID{transactionID}, startTS{startTS},
      commitTS{common::INVALID_TRANSACTION}, forceCheckpoint{false}, hasCatalogChanges{false} {
    this->clientContext = &clientContext;
    localStorage = std::make_unique<storage::LocalStorage>(clientContext);
    undoBuffer = std::make_unique<storage::UndoBuffer>(storage::MemoryManager::Get(clientContext));
    currentTS = common::Timestamp::getCurrentTimestamp().value;
    localWAL = std::make_unique<storage::LocalWAL>(*storage::MemoryManager::Get(clientContext),
        clientContext.getDBConfig()->enableChecksums);
}

Transaction::Transaction(TransactionType transactionType) noexcept
    : type{transactionType}, ID{DUMMY_TRANSACTION_ID}, startTS{DUMMY_START_TIMESTAMP},
      commitTS{common::INVALID_TRANSACTION}, clientContext{nullptr}, undoBuffer{nullptr},
      forceCheckpoint{false}, hasCatalogChanges{false} {
    currentTS = common::Timestamp::getCurrentTimestamp().value;
}

Transaction::Transaction(TransactionType transactionType, common::transaction_t ID,
    common::transaction_t startTS) noexcept
    : type{transactionType}, ID{ID}, startTS{startTS}, commitTS{common::INVALID_TRANSACTION},
      clientContext{nullptr}, undoBuffer{nullptr}, forceCheckpoint{false},
      hasCatalogChanges{false} {
    currentTS = common::Timestamp::getCurrentTimestamp().value;
}

bool Transaction::shouldLogToWAL() const {
    return isWriteTransaction() && !clientContext->isInMemory();
}

bool Transaction::shouldForceCheckpoint() const {
    return !clientContext->isInMemory() && forceCheckpoint;
}

uint64_t Transaction::copyJournalThreshold() const {
    const auto configured = clientContext->getClientConfig()->copyJournalThreshold;
    if (configured != 0) {
        return configured;
    }
    // Le défaut : un huitième du tampon, au plus 256 Mio. Le journal d'une transaction est de
    // la mémoire que le tampon ne peut pas évincer.
    static constexpr uint64_t CEILING = 256ull * 1024 * 1024;
    return std::min(clientContext->getDBConfig()->bufferPoolSize / 8, CEILING);
}

bool Transaction::journalExceedsCopyThreshold() const {
    return localWAL && localWAL->getSize() > copyJournalThreshold();
}

void Transaction::fallBackToForcedCheckpoint() {
    if (forceCheckpoint) {
        return;
    }
    forceCheckpoint = true;
    if (localWAL) {
        localWAL->discard();
    }
    storage::StorageManager::Get(*clientContext)->getWAL().noteCopyJournalFallback();
}

// La sonde RAG3DB_PROFILE_JOURNAL : à la validation, ce que pèse le journal de la transaction,
// par type d'enregistrement et par table, sur la sortie d'erreur. Une ligne par transaction qui
// a écrit ; chaque ligne commence par « [journal] ».
static void printJournalWeights(storage::LocalWAL& localWAL, main::ClientContext& context,
    const Transaction& transaction, bool forced) {
    const auto size = localWAL.getSize();
    const auto weights = localWAL.getWeights();
    if (weights.empty()) {
        return;
    }
    std::string detail;
    for (const auto& weight : weights) {
        std::string table;
        if (weight.tableID != common::INVALID_TABLE_ID) {
            table = " table " + std::to_string(weight.tableID);
            try {
                table += " " + Catalog::Get(context)
                                   ->getTableCatalogEntry(&transaction, weight.tableID)
                                   ->getName();
            } catch (const std::exception&) { // NOLINT(bugprone-empty-catch): un nom en moins.
            }
        }
        detail += common::stringFormat(" ; type {}{} : {} octets, {} enregistrements, {} lignes",
            static_cast<uint32_t>(weight.recordType), table, weight.numBytes, weight.numRecords,
            weight.numRows);
    }
    fprintf(stderr, "[journal] transaction validée : %llu octets au journal%s%s\n",
        static_cast<unsigned long long>(size),
        forced ? " (non écrits : point de reprise forcé ; après un repli, les poids sont ceux "
                 "d'avant lui)" :
                 "",
        detail.c_str());
}

void Transaction::commit(storage::WAL* wal) {
    localStorage->commit();
    undoBuffer->commit(commitTS);
    if (shouldLogToWAL()) {
        KU_ASSERT(localWAL && wal);
        if (storage::LocalWAL::profiled()) {
            printJournalWeights(*localWAL, *clientContext, *this, shouldForceCheckpoint());
        }
        if (shouldForceCheckpoint()) {
            // Une transaction dont la durabilité est son point de reprise (un COPY hors journal,
            // la création d'un index) n'écrit RIEN au journal, pas même ses écritures
            // ordinaires. Les y écrire avec leur COMMIT les rendait durables avant ce qu'elle
            // n'y écrit pas : après une mort pendant le point de reprise, la réouverture
            // rejouait la moitié de la transaction. Le point de reprise, atomique, emporte tout
            // son état — elle revient entière, ou pas du tout.
            //
            // L'invariant qui le permet, tenu par TransactionManager::commit : une transaction
            // forcée ne finit jamais sa validation sans que son point de reprise ait abouti.
            // L'attente du départ des autres a lieu avant, et son délai annule ; le point de
            // reprise qui suit n'est ni reporté ni soumis à auto_checkpoint ; s'il échoue, la
            // base refuse tout jusqu'à sa réouverture, où rien de la transaction ne revient.
            // « Forcée » exclut la base en mémoire (shouldForceCheckpoint), et le rejeu ne
            // force jamais.
        } else {
            localWAL->logCommit();
            wal->logCommittedWAL(*localWAL, clientContext);
        }
        localWAL->clear();
    }
    if (hasCatalogChanges) {
        Catalog::Get(*clientContext)->incrementVersion();
        hasCatalogChanges = false;
    }
}

void Transaction::rollback(storage::WAL*) {
    // Rolling back the local storage will free + evict all optimistically-allocated pages
    // Since the undo buffer may do some scanning (e.g. to delete inserted keys from the hash index)
    // this must be rolled back first
    undoBuffer->rollback(clientContext);
    localStorage->rollback();
    hasCatalogChanges = false;
}

bool Transaction::isUnCommitted(common::table_id_t tableID, common::offset_t nodeOffset) const {
    return localStorage && localStorage->getLocalTable(tableID) &&
           nodeOffset >= getMinUncommittedNodeOffset(tableID);
}

void Transaction::pushCreateDropCatalogEntry(CatalogSet& catalogSet, CatalogEntry& catalogEntry,
    bool isInternal, bool skipLoggingToWAL) {
    undoBuffer->createCatalogEntry(catalogSet, catalogEntry);
    hasCatalogChanges = true;
    if (!shouldLogToWAL() || skipLoggingToWAL) {
        return;
    }
    KU_ASSERT(localWAL);
    const auto newCatalogEntry = catalogEntry.getNext();
    switch (newCatalogEntry->getType()) {
    case CatalogEntryType::INDEX_ENTRY:
    case CatalogEntryType::NODE_TABLE_ENTRY:
    case CatalogEntryType::REL_GROUP_ENTRY: {
        if (catalogEntry.getType() == CatalogEntryType::DUMMY_ENTRY) {
            KU_ASSERT(catalogEntry.isDeleted());
            localWAL->logCreateCatalogEntryRecord(newCatalogEntry, isInternal);
        } else {
            throw common::RuntimeException("This shouldn't happen. Alter table is not supported.");
        }
    } break;
    case CatalogEntryType::SEQUENCE_ENTRY: {
        KU_ASSERT(
            catalogEntry.getType() == CatalogEntryType::DUMMY_ENTRY && catalogEntry.isDeleted());
        if (newCatalogEntry->hasParent()) {
            // We don't log SERIAL catalog entry creation as it is implicit
            return;
        }
        localWAL->logCreateCatalogEntryRecord(newCatalogEntry, isInternal);
    } break;
    case CatalogEntryType::SCALAR_MACRO_ENTRY:
    case CatalogEntryType::TYPE_ENTRY: {
        KU_ASSERT(
            catalogEntry.getType() == CatalogEntryType::DUMMY_ENTRY && catalogEntry.isDeleted());
        localWAL->logCreateCatalogEntryRecord(newCatalogEntry, isInternal);
    } break;
    case CatalogEntryType::DUMMY_ENTRY: {
        KU_ASSERT(newCatalogEntry->isDeleted());
        if (catalogEntry.hasParent()) {
            return;
        }
        switch (catalogEntry.getType()) {
        case CatalogEntryType::INDEX_ENTRY:
        case CatalogEntryType::SCALAR_MACRO_ENTRY:
        case CatalogEntryType::NODE_TABLE_ENTRY:
        case CatalogEntryType::REL_GROUP_ENTRY:
        case CatalogEntryType::SEQUENCE_ENTRY: {
            localWAL->logDropCatalogEntryRecord(catalogEntry.getOID(), catalogEntry.getType());
        } break;
        case CatalogEntryType::SCALAR_FUNCTION_ENTRY:
        case CatalogEntryType::TABLE_FUNCTION_ENTRY:
        case CatalogEntryType::STANDALONE_TABLE_FUNCTION_ENTRY: {
            // DO NOTHING. We don't persist function entries.
        } break;
        case CatalogEntryType::TYPE_ENTRY:
        default: {
            throw common::RuntimeException(
                common::stringFormat("Not supported catalog entry type {} yet.",
                    CatalogEntryTypeUtils::toString(catalogEntry.getType())));
        }
        }
    } break;
    case CatalogEntryType::SCALAR_FUNCTION_ENTRY:
    case CatalogEntryType::TABLE_FUNCTION_ENTRY:
    case CatalogEntryType::STANDALONE_TABLE_FUNCTION_ENTRY: {
        // DO NOTHING. We don't persist function entries.
    } break;
    default: {
        throw common::RuntimeException(
            common::stringFormat("Not supported catalog entry type {} yet.",
                CatalogEntryTypeUtils::toString(catalogEntry.getType())));
    }
    }
}

void Transaction::pushAlterCatalogEntry(CatalogSet& catalogSet, CatalogEntry& catalogEntry,
    const binder::BoundAlterInfo& alterInfo) {
    undoBuffer->createCatalogEntry(catalogSet, catalogEntry);
    hasCatalogChanges = true;
    if (shouldLogToWAL()) {
        KU_ASSERT(localWAL);
        localWAL->logAlterCatalogEntryRecord(&alterInfo);
    }
}

void Transaction::pushSequenceChange(SequenceCatalogEntry* sequenceEntry, int64_t kCount,
    const SequenceRollbackData& data) {
    undoBuffer->createSequenceChange(*sequenceEntry, data);
    hasCatalogChanges = true;
    if (shouldLogToWAL()) {
        KU_ASSERT(localWAL);
        localWAL->logUpdateSequenceRecord(sequenceEntry->getOID(), kCount);
    }
}

void Transaction::pushInsertInfo(common::node_group_idx_t nodeGroupIdx, common::row_idx_t startRow,
    common::row_idx_t numRows, const storage::VersionRecordHandler* versionRecordHandler) const {
    undoBuffer->createInsertInfo(nodeGroupIdx, startRow, numRows, versionRecordHandler);
}

void Transaction::pushDeleteInfo(common::node_group_idx_t nodeGroupIdx, common::row_idx_t startRow,
    common::row_idx_t numRows, const storage::VersionRecordHandler* versionRecordHandler) const {
    undoBuffer->createDeleteInfo(nodeGroupIdx, startRow, numRows, versionRecordHandler);
}

void Transaction::pushVectorUpdateInfo(storage::UpdateInfo& updateInfo,
    const common::idx_t vectorIdx, storage::VectorUpdateInfo& vectorUpdateInfo,
    common::transaction_t version) const {
    undoBuffer->createVectorUpdateInfo(&updateInfo, vectorIdx, &vectorUpdateInfo, version);
}

Transaction::~Transaction() = default;

common::offset_t Transaction::getMinUncommittedNodeOffset(common::table_id_t tableID) const {
    if (localStorage && localStorage->getLocalTable(tableID)) {
        auto& localTable = localStorage->getLocalTable(tableID)->cast<storage::LocalNodeTable>();
        // Sans ligne locale, aucun décalage n'en désigne une : le départ noté est celui d'un
        // moment passé (avant un COPY de la transaction, ou avant un versement).
        return localTable.getNumTotalRows() == 0 ? common::INVALID_OFFSET :
                                                   localTable.getStartOffset();
    }
    return 0;
}

Transaction* Transaction::Get(const main::ClientContext& context) {
    return TransactionContext::Get(context)->getActiveTransaction();
}

Transaction DUMMY_TRANSACTION = Transaction(TransactionType::DUMMY);
Transaction DUMMY_CHECKPOINT_TRANSACTION = Transaction(TransactionType::CHECKPOINT,
    Transaction::DUMMY_TRANSACTION_ID, Transaction::START_TRANSACTION_ID - 1);

} // namespace transaction
} // namespace rag3db
