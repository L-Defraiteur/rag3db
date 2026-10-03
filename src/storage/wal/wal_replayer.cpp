#include "storage/wal/wal_replayer.h"

#include <algorithm>
#include <chrono>
#include <iostream>
#include <optional>

#include "binder/binder.h"
#include "catalog/catalog_entry/index_catalog_entry.h"
#include "catalog/catalog_entry/scalar_macro_catalog_entry.h"
#include "catalog/catalog_entry/sequence_catalog_entry.h"
#include "catalog/catalog_entry/table_catalog_entry.h"
#include "catalog/catalog_entry/type_catalog_entry.h"
#include "common/checksum.h"
#include "common/exception/end_of_file.h"
#include "common/exception/io.h"
#include "common/exception/runtime.h"
#include "common/file_system/file_info.h"
#include "common/file_system/file_system.h"
#include "common/file_system/virtual_file_system.h"
#include "common/serializer/buffered_file.h"
#include "common/string_format.h"
#include "extension/extension_manager.h"
#include "main/client_context.h"
#include "processor/expression_mapper.h"
#include "storage/file_db_id_utils.h"
#include "storage/local_storage/local_rel_table.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
#include "storage/table/rel_table.h"
#include "storage/wal/checksum_reader.h"
#include "storage/wal/wal_record.h"
#include "transaction/transaction_context.h"

using namespace rag3db::binder;
using namespace rag3db::catalog;
using namespace rag3db::common;
using namespace rag3db::processor;
using namespace rag3db::storage;
using namespace rag3db::transaction;

namespace rag3db {
namespace storage {

static constexpr std::string_view checksumMismatchMessage =
    "Checksum verification failed, the WAL file is corrupted.";

WALReplayer::WALReplayer(main::ClientContext& clientContext) : clientContext{clientContext} {
    walPath = StorageUtils::getWALFilePath(clientContext.getDatabasePath());
    shadowFilePath = StorageUtils::getShadowFilePath(clientContext.getDatabasePath());
}

static WALHeader readWALHeader(Deserializer& deserializer) {
    WALHeader header{};
    deserializer.deserializeValue(header.databaseID);

    // It is possible to read a value other than 0/1 when deserializing the flag
    // This causes some weird behaviours with some toolchains so we manually do the conversion here
    uint8_t enableChecksumsBytes = 0;
    deserializer.deserializeValue(enableChecksumsBytes);
    header.enableChecksums = enableChecksumsBytes != 0;

    return header;
}

static Deserializer initDeserializer(FileInfo& fileInfo, main::ClientContext& clientContext,
    bool enableChecksums) {
    if (enableChecksums) {
        return Deserializer{std::make_unique<ChecksumReader>(fileInfo,
            *MemoryManager::Get(clientContext), checksumMismatchMessage)};
    } else {
        return Deserializer{std::make_unique<BufferedFileReader>(fileInfo)};
    }
}

static void checkWALHeader(const WALHeader& header, bool enableChecksums) {
    if (enableChecksums != header.enableChecksums) {
        throw RuntimeException(stringFormat(
            "The database you are trying to open was serialized with enableChecksums={} but you "
            "are trying to open it with enableChecksums={}. Please open your database using the "
            "correct enableChecksums config. If you wish to change this for your database, please "
            "use the export/import functionality.",
            TypeUtils::toString(header.enableChecksums), TypeUtils::toString(enableChecksums)));
    }
}

static uint64_t getReadOffset(Deserializer& deSer, bool enableChecksums) {
    if (enableChecksums) {
        return deSer.getReader()->cast<ChecksumReader>()->getReadOffset();
    } else {
        return deSer.getReader()->cast<BufferedFileReader>()->getReadOffset();
    }
}

// Test-only, see wal_replayer.h. Not synchronised: tests set it before opening.
static WALReplayer::read_only_open_hook_t readOnlyOpenHookForTesting;

void WALReplayer::setReadOnlyOpenHookForTesting(read_only_open_hook_t hook) {
    readOnlyOpenHookForTesting = std::move(hook);
}

void WALReplayer::runReadOnlyOpenHook(ReadOnlyOpenPhase phase) const {
    if (readOnlyOpenHookForTesting && StorageManager::Get(clientContext)->isReadOnly()) {
        readOnlyOpenHookForTesting(phase);
    }
}

// Checksum of the first `size` bytes of a file, read in blocks. Used only to compare two reads
// of the same bytes; a file that shrinks under it gives a different value, which is what we want.
static uint64_t checksumOfPrefix(FileInfo& fileInfo, uint64_t size) {
    static constexpr uint64_t BLOCK_SIZE = 1ull << 20;
    const auto buffer = std::make_unique<uint8_t[]>(std::min<uint64_t>(BLOCK_SIZE, size) + 1);
    uint64_t result = 0;
    for (uint64_t position = 0; position < size; position += BLOCK_SIZE) {
        const auto numBytes = std::min<uint64_t>(BLOCK_SIZE, size - position);
        fileInfo.readFromFile(buffer.get(), numBytes, position);
        result = result * 1099511628211ull + checksum(buffer.get(), numBytes);
    }
    return result;
}

// Page 0 of the data file, raw. Empty if the file is missing.
static std::string readHeaderPage(main::ClientContext& clientContext) {
    auto vfs = VirtualFileSystem::GetUnsafe(clientContext);
    const auto databasePath = clientContext.getDatabasePath();
    std::string page;
    if (!vfs->fileOrPathExists(databasePath, &clientContext)) {
        return page;
    }
    const auto dataFile = vfs->openFile(databasePath, FileOpenFlags(FileFlags::READ_ONLY));
    page.resize(std::min<uint64_t>(dataFile->getFileSize(), RAG3DB_PAGE_SIZE));
    if (!page.empty()) {
        dataFile->readFromFile(page.data(), page.size(), 0 /* position */);
    }
    return page;
}

WALReplayer::ReadOnlyOpenIdentity WALReplayer::captureReadOnlyOpenIdentity() const {
    ReadOnlyOpenIdentity identity;
    identity.headerPage = readHeaderPage(clientContext);
    auto vfs = VirtualFileSystem::GetUnsafe(clientContext);
    try {
        if (vfs->fileOrPathExists(walPath, &clientContext)) {
            const auto journal = vfs->openFile(walPath, FileOpenFlags(FileFlags::READ_ONLY));
            identity.journalSize = journal->getFileSize();
            identity.journalChecksum = checksumOfPrefix(*journal, identity.journalSize);
            identity.journalExists = true;
        }
    } catch (const IOException&) {
        // The journal vanished between the test and the open: a checkpoint just removed it. Seen
        // as absent; the header page tells the rest.
        identity = ReadOnlyOpenIdentity{identity.headerPage};
    }
    return identity;
}

void WALReplayer::throwIfCheckpointCrossedReadOnlyOpen(const ReadOnlyOpenIdentity& before,
    bool enableChecksums) const {
    auto crossed = readHeaderPage(clientContext) != before.headerPage;
    if (!crossed) {
        auto vfs = VirtualFileSystem::GetUnsafe(clientContext);
        std::unique_ptr<FileInfo> journal;
        uint64_t journalSize = 0;
        try {
            if (vfs->fileOrPathExists(walPath, &clientContext)) {
                journal = vfs->openFile(walPath, FileOpenFlags(FileFlags::READ_ONLY));
                journalSize = journal->getFileSize();
            }
            if (before.journalExists && before.journalSize > 0) {
                // The bytes this open decided on must still be there, unchanged.
                crossed = !journal || journalSize < before.journalSize ||
                          checksumOfPrefix(*journal, before.journalSize) != before.journalChecksum;
            }
            if (!crossed && journal && journalSize > 0 && journalSize != before.journalSize) {
                // The journal grew. New commits are harmless: they only append here and write
                // pages the last checkpoint does not reference. A CHECKPOINT record at the end is
                // a checkpoint in progress, applying its pages to the data file right now.
                crossed = dryReplay(*journal, false /* throwOnWalReplayFailure */, enableChecksums)
                              .isLastRecordCheckpoint;
            }
        } catch (const IOException&) {
            // The journal was removed or cut while we were comparing: that is a checkpoint.
            crossed = true;
        }
    }
    if (crossed) {
        throw RuntimeException(stringFormat(
            "{}: what it read may mix the state before and after that checkpoint, so the open is "
            "refused rather than served. Retry the open.",
            CHECKPOINT_CROSSED_READ_ONLY_OPEN));
    }
}

void WALReplayer::replay(bool throwOnWalReplayFailure, bool enableChecksums) const {
    auto vfs = VirtualFileSystem::GetUnsafe(clientContext);
    Checkpointer checkpointer(clientContext);
    // A read-only open takes no lock and reads the journal and the data file at different times.
    // It remembers what both looked like before reading them, and checks afterwards that no
    // checkpoint of a writer crossed it (see ReadOnlyOpenIdentity). This protects the open only:
    // a reader that stays open while a checkpoint passes is NOT protected by it.
    std::optional<ReadOnlyOpenIdentity> identity;
    if (StorageManager::Get(clientContext)->isReadOnly()) {
        identity = captureReadOnlyOpenIdentity();
    }
    auto refusedAsCrossed = false;
    const auto verifyNoCheckpointCrossed = [&]() {
        if (!identity) {
            return;
        }
        try {
            throwIfCheckpointCrossedReadOnlyOpen(*identity, enableChecksums);
        } catch (...) {
            refusedAsCrossed = true;
            throw;
        }
    };
    // Whatever goes wrong while a read-only open reads the two files, the first question is
    // whether a checkpoint crossed it: if so that is the cause, and it is what gets reported. If
    // not, the original error goes out untouched. The error is neither read nor retried here.
    const auto rethrowAsCrossedIfSo = [&]() {
        if (!refusedAsCrossed) {
            verifyNoCheckpointCrossed();
        }
    };
    // First, check if the WAL file exists. If it does not, we can safely remove the shadow file.
    if (!vfs->fileOrPathExists(walPath, &clientContext)) {
        removeFileIfExists(shadowFilePath);
        runReadOnlyOpenHook(ReadOnlyOpenPhase::JOURNAL_SCANNED);
        // Read the checkpointed data from the disk.
        try {
            checkpointer.readCheckpoint();
        } catch (const std::exception&) {
            rethrowAsCrossedIfSo();
            throw;
        }
        verifyNoCheckpointCrossed();
        return;
    }
    // If the WAL file exists, we need to replay it.
    std::unique_ptr<FileInfo> fileInfo;
    try {
        fileInfo = openWALFile();
    } catch (const std::exception&) {
        // A reader can find the journal removed between the test above and this open: the end of
        // a writer's checkpoint.
        rethrowAsCrossedIfSo();
        throw;
    }
    // Check if the wal file is empty. If so, we do not need to replay anything.
    if (fileInfo->getFileSize() == 0) {
        removeWALAndShadowFiles();
        runReadOnlyOpenHook(ReadOnlyOpenPhase::JOURNAL_SCANNED);
        // Read the checkpointed data from the disk.
        try {
            checkpointer.readCheckpoint();
        } catch (const std::exception&) {
            rethrowAsCrossedIfSo();
            throw;
        }
        verifyNoCheckpointCrossed();
        return;
    }
    // A previous unclean exit may have left non-durable contents in the WAL, so before we start
    // replaying the WAL records, make a best-effort attempt at ensuring the WAL is fully durable.
    syncWALFile(*fileInfo);

    // Start replaying the WAL records.
    try {
        // First, we dry run the replay to find out the offset of the last record that was
        // CHECKPOINT or COMMIT.
        auto [offsetDeserialized, isLastRecordCheckpoint, tornEnd] =
            dryReplay(*fileInfo, throwOnWalReplayFailure, enableChecksums);
        // Whatever is about to be cut from the journal is copied aside first,
        // torn end or not (throwOnWalReplayFailure=false on a corruption).
        if (!isLastRecordCheckpoint) {
            setAsideCutBytes(*fileInfo, offsetDeserialized, tornEnd);
        }
        if (isLastRecordCheckpoint) {
            // If the last record is a checkpoint, we resume by replaying the shadow file.
            ShadowFile::replayShadowPageRecords(clientContext);
            removeWALAndShadowFiles();
            // Re-read checkpointed data from disk again as now the shadow file is applied.
            checkpointer.readCheckpoint();
        } else {
            // There is no checkpoint record, so we should remove the shadow file if it exists.
            removeFileIfExists(shadowFilePath);
            runReadOnlyOpenHook(ReadOnlyOpenPhase::JOURNAL_SCANNED);
            // Read the checkpointed data from the disk.
            checkpointer.readCheckpoint();
            // The data file is read: before replaying the journal on top of it, make sure it is
            // still the one the journal belongs to.
            verifyNoCheckpointCrossed();
            // Il y a un journal à rejouer : il lui faut les extensions dont viennent les
            // index des tables où il écrit, et il ne les porte plus si un point de reprise
            // est passé depuis leur chargement.
            extension::ExtensionManager::Get(clientContext)
                ->loadExtensionsNotedBesideTheDatabase(&clientContext);
            runReadOnlyOpenHook(ReadOnlyOpenPhase::DATA_FILE_READ);
            // Resume by replaying the WAL file from the beginning until the last COMMIT record.
            Deserializer deserializer = initDeserializer(*fileInfo, clientContext, enableChecksums);

            if (offsetDeserialized > 0) {
                // Make sure the WAL file is for the current database
                deserializer.getReader()->onObjectBegin();
                const auto walHeader = readWALHeader(deserializer);
                FileDBIDUtils::verifyDatabaseID(*fileInfo,
                    StorageManager::Get(clientContext)->getOrInitDatabaseID(clientContext),
                    walHeader.databaseID);
                deserializer.getReader()->onObjectEnd();
            }

            while (getReadOffset(deserializer, enableChecksums) < offsetDeserialized) {
                KU_ASSERT(!deserializer.finished());
                auto walRecord = WALRecord::deserialize(deserializer, clientContext);
                replayWALRecord(*walRecord);
            }
            // After replaying all the records, we should truncate the WAL file to the last
            // COMMIT/CHECKPOINT record.
            truncateWALFile(*fileInfo, offsetDeserialized);
            // The replay read the journal again and, through the tables, pages of the data file:
            // a checkpoint that started meanwhile invalidates it too.
            verifyNoCheckpointCrossed();
        }
    } catch (const std::exception&) {
        auto transactionContext = TransactionContext::Get(clientContext);
        if (transactionContext->hasActiveTransaction()) {
            // Handle the case that some transaction went during replaying. We should roll back
            // under this case. Usually this shouldn't happen, but it is possible if we have a bug
            // with the replay logic. This is to handle cases like that so we don't corrupt
            // transactions that have been replayed.
            transactionContext->rollback();
        }
        rethrowAsCrossedIfSo();
        throw;
    }
}

WALReplayer::WALReplayInfo WALReplayer::dryReplay(FileInfo& fileInfo, bool throwOnWalReplayFailure,
    bool enableChecksums) const {
    uint64_t offsetDeserialized = 0;
    bool isLastRecordCheckpoint = false;
    bool tornEnd = false;
    try {
        Deserializer deserializer = initDeserializer(fileInfo, clientContext, enableChecksums);

        // Skip the databaseID here, we'll verify it when we actually replay
        deserializer.getReader()->onObjectBegin();
        const auto walHeader = readWALHeader(deserializer);
        checkWALHeader(walHeader, enableChecksums);
        deserializer.getReader()->onObjectEnd();

        bool finishedDeserializing = deserializer.finished();
        while (!finishedDeserializing) {
            auto walRecord = WALRecord::deserialize(deserializer, clientContext);
            finishedDeserializing = deserializer.finished();
            switch (walRecord->type) {
            case WALRecordType::CHECKPOINT_RECORD: {
                KU_ASSERT(finishedDeserializing);
                // If we reach a checkpoint record, we can stop replaying.
                isLastRecordCheckpoint = true;
                finishedDeserializing = true;
                offsetDeserialized = getReadOffset(deserializer, enableChecksums);
            } break;
            case WALRecordType::COMMIT_RECORD: {
                // Update the offset to the end of the last commit record.
                offsetDeserialized = getReadOffset(deserializer, enableChecksums);
            } break;
            default: {
                // DO NOTHING.
            }
            }
        }
        // Records after the last COMMIT that end on a record boundary: a
        // transaction whose COMMIT was never written. Also a torn end.
        tornEnd = !isLastRecordCheckpoint &&
                  getReadOffset(deserializer, enableChecksums) > offsetDeserialized;
    } catch (const EndOfFileException&) {
        // The file ended inside a record: a torn end, left by a stop while a
        // commit was being appended. Every byte before it was read and
        // verified, so the journal is replayed up to the last COMMIT /
        // CHECKPOINT, whatever throwOnWalReplayFailure says (decided on
        // 2026-10-01). A corruption followed by more data is not this case.
        tornEnd = true;
    } catch (...) {
        // Any other failure is a corruption in a journal that may continue
        // with committed transactions: without record lengths we cannot look
        // past it, so by default the database refuses to open.
        if (throwOnWalReplayFailure) {
            throw;
        }
    }
    return {offsetDeserialized, isLastRecordCheckpoint, tornEnd};
}

void WALReplayer::replayWALRecord(WALRecord& walRecord) const {
    switch (walRecord.type) {
    case WALRecordType::BEGIN_TRANSACTION_RECORD: {
        TransactionContext::Get(clientContext)->beginRecoveryTransaction();
    } break;
    case WALRecordType::COMMIT_RECORD: {
        TransactionContext::Get(clientContext)->commit();
    } break;
    case WALRecordType::CREATE_CATALOG_ENTRY_RECORD: {
        replayCreateCatalogEntryRecord(walRecord);
    } break;
    case WALRecordType::DROP_CATALOG_ENTRY_RECORD: {
        replayDropCatalogEntryRecord(walRecord);
    } break;
    case WALRecordType::ALTER_TABLE_ENTRY_RECORD: {
        replayAlterTableEntryRecord(walRecord);
    } break;
    case WALRecordType::TABLE_INSERTION_RECORD: {
        replayTableInsertionRecord(walRecord);
    } break;
    case WALRecordType::NODE_DELETION_RECORD: {
        replayNodeDeletionRecord(walRecord);
    } break;
    case WALRecordType::NODE_UPDATE_RECORD: {
        replayNodeUpdateRecord(walRecord);
    } break;
    case WALRecordType::REL_DELETION_RECORD: {
        replayRelDeletionRecord(walRecord);
    } break;
    case WALRecordType::REL_DETACH_DELETE_RECORD: {
        replayRelDetachDeletionRecord(walRecord);
    } break;
    case WALRecordType::REL_UPDATE_RECORD: {
        replayRelUpdateRecord(walRecord);
    } break;
    case WALRecordType::COPY_TABLE_RECORD: {
        replayCopyTableRecord(walRecord);
    } break;
    case WALRecordType::UPDATE_SEQUENCE_RECORD: {
        replayUpdateSequenceRecord(walRecord);
    } break;
    case WALRecordType::LOAD_EXTENSION_RECORD: {
        replayLoadExtensionRecord(walRecord);
    } break;
    case WALRecordType::CHECKPOINT_RECORD: {
        // This record should not be replayed. It is only used to indicate that the previous records
        // had been replayed and shadow files are created.
        KU_UNREACHABLE;
    }
    default:
        KU_UNREACHABLE;
    }
}

void WALReplayer::replayCreateCatalogEntryRecord(WALRecord& walRecord) const {
    auto catalog = Catalog::Get(clientContext);
    auto transaction = transaction::Transaction::Get(clientContext);
    auto storageManager = StorageManager::Get(clientContext);
    auto& record = walRecord.cast<CreateCatalogEntryRecord>();
    switch (record.ownedCatalogEntry->getType()) {
    case CatalogEntryType::NODE_TABLE_ENTRY:
    case CatalogEntryType::REL_GROUP_ENTRY: {
        auto& entry = record.ownedCatalogEntry->constCast<TableCatalogEntry>();
        auto newEntry = catalog->createTableEntry(transaction,
            entry.getBoundCreateTableInfo(transaction, record.isInternal));
        storageManager->createTable(newEntry->ptrCast<TableCatalogEntry>());
    } break;
    case CatalogEntryType::SCALAR_MACRO_ENTRY: {
        auto& macroEntry = record.ownedCatalogEntry->constCast<ScalarMacroCatalogEntry>();
        catalog->addScalarMacroFunction(transaction, macroEntry.getName(),
            macroEntry.getMacroFunction()->copy());
    } break;
    case CatalogEntryType::SEQUENCE_ENTRY: {
        auto& sequenceEntry = record.ownedCatalogEntry->constCast<SequenceCatalogEntry>();
        catalog->createSequence(transaction,
            sequenceEntry.getBoundCreateSequenceInfo(record.isInternal));
    } break;
    case CatalogEntryType::TYPE_ENTRY: {
        auto& typeEntry = record.ownedCatalogEntry->constCast<TypeCatalogEntry>();
        catalog->createType(transaction, typeEntry.getName(), typeEntry.getLogicalType().copy());
    } break;
    case CatalogEntryType::INDEX_ENTRY: {
        catalog->createIndex(transaction, std::move(record.ownedCatalogEntry));
    } break;
    default: {
        KU_UNREACHABLE;
    }
    }
}

void WALReplayer::replayDropCatalogEntryRecord(const WALRecord& walRecord) const {
    auto& dropEntryRecord = walRecord.constCast<DropCatalogEntryRecord>();
    auto catalog = Catalog::Get(clientContext);
    auto transaction = transaction::Transaction::Get(clientContext);
    const auto entryID = dropEntryRecord.entryID;
    switch (dropEntryRecord.entryType) {
    case CatalogEntryType::NODE_TABLE_ENTRY:
    case CatalogEntryType::REL_GROUP_ENTRY: {
        KU_ASSERT(Catalog::Get(clientContext));
        catalog->dropTableEntry(transaction, entryID);
    } break;
    case CatalogEntryType::SEQUENCE_ENTRY: {
        catalog->dropSequence(transaction, entryID);
    } break;
    case CatalogEntryType::INDEX_ENTRY: {
        // À l'exécution, c'est la fonction qui retire l'index (DROP_VECTOR_INDEX, …) qui
        // l'ôte aussi de la table ; le journal ne garde que l'entrée du catalogue. Le rejeu
        // doit donc faire les deux, sinon la table garde un index que le catalogue ne
        // connaît plus, que personne ne chargera et qu'on ne peut plus recréer.
        for (const auto indexEntry : catalog->getIndexEntries(transaction)) {
            if (indexEntry->getOID() != entryID) {
                continue;
            }
            auto table = StorageManager::Get(clientContext)->getTable(indexEntry->getTableID());
            table->cast<NodeTable>().dropIndex(indexEntry->getIndexName());
            break;
        }
        catalog->dropIndex(transaction, entryID);
    } break;
    case CatalogEntryType::SCALAR_MACRO_ENTRY: {
        catalog->dropMacroEntry(transaction, entryID);
    } break;
    default: {
        KU_UNREACHABLE;
    }
    }
}

void WALReplayer::replayAlterTableEntryRecord(const WALRecord& walRecord) const {
    auto binder = Binder(&clientContext);
    auto& alterEntryRecord = walRecord.constCast<AlterTableEntryRecord>();
    auto catalog = Catalog::Get(clientContext);
    auto transaction = transaction::Transaction::Get(clientContext);
    auto storageManager = StorageManager::Get(clientContext);
    auto ownedAlterInfo = alterEntryRecord.ownedAlterInfo.get();
    catalog->alterTableEntry(transaction, *ownedAlterInfo);
    auto& pageAllocator = *PageManager::Get(clientContext);
    switch (ownedAlterInfo->alterType) {
    case AlterType::ADD_PROPERTY: {
        const auto exprBinder = binder.getExpressionBinder();
        const auto addInfo = ownedAlterInfo->extraInfo->constPtrCast<BoundExtraAddPropertyInfo>();
        // We don't implicit cast here since it must already be done the first time
        const auto boundDefault =
            exprBinder->bindExpression(*addInfo->propertyDefinition.defaultExpr);
        auto exprMapper = ExpressionMapper();
        const auto defaultValueEvaluator = exprMapper.getEvaluator(boundDefault);
        defaultValueEvaluator->init(ResultSet(0) /* dummy ResultSet */, &clientContext);
        const auto entry = catalog->getTableCatalogEntry(transaction, ownedAlterInfo->tableName);
        const auto& addedProp = entry->getProperty(addInfo->propertyDefinition.getName());
        TableAddColumnState state{addedProp, *defaultValueEvaluator};
        KU_ASSERT(StorageManager::Get(clientContext));
        switch (entry->getTableType()) {
        case TableType::REL: {
            for (auto& relEntryInfo : entry->cast<RelGroupCatalogEntry>().getRelEntryInfos()) {
                storageManager->getTable(relEntryInfo.oid)
                    ->addColumn(transaction, state, pageAllocator);
            }
        } break;
        case TableType::NODE: {
            storageManager->getTable(entry->getTableID())
                ->addColumn(transaction, state, pageAllocator);
        } break;
        default: {
            KU_UNREACHABLE;
        }
        }
    } break;
    case AlterType::ADD_FROM_TO_CONNECTION: {
        auto extraInfo = ownedAlterInfo->extraInfo->constPtrCast<BoundExtraAlterFromToConnection>();
        auto relGroupEntry = catalog->getTableCatalogEntry(transaction, ownedAlterInfo->tableName)
                                 ->ptrCast<RelGroupCatalogEntry>();
        auto relEntryInfo =
            relGroupEntry->getRelEntryInfo(extraInfo->fromTableID, extraInfo->toTableID);
        storageManager->addRelTable(relGroupEntry, *relEntryInfo);
    } break;
    default:
        break;
    }
}

void WALReplayer::replayTableInsertionRecord(const WALRecord& walRecord) const {
    const auto& insertionRecord = walRecord.constCast<TableInsertionRecord>();
    switch (insertionRecord.tableType) {
    case TableType::NODE: {
        replayNodeTableInsertRecord(walRecord);
    } break;
    case TableType::REL: {
        replayRelTableInsertRecord(walRecord);
    } break;
    default: {
        throw RuntimeException("Invalid table type for insertion replay in WAL record.");
    }
    }
}

void WALReplayer::replayNodeTableInsertRecord(const WALRecord& walRecord) const {
    const auto& insertionRecord = walRecord.constCast<TableInsertionRecord>();
    const auto tableID = insertionRecord.tableID;
    auto& table = StorageManager::Get(clientContext)->getTable(tableID)->cast<NodeTable>();
    KU_ASSERT(!insertionRecord.ownedVectors.empty());
    const auto anchorState = insertionRecord.ownedVectors[0]->state;
    const auto numNodes = anchorState->getSelVector().getSelSize();
    for (auto i = 0u; i < insertionRecord.ownedVectors.size(); i++) {
        insertionRecord.ownedVectors[i]->setState(anchorState);
    }
    std::vector<ValueVector*> propertyVectors(insertionRecord.ownedVectors.size());
    for (auto i = 0u; i < insertionRecord.ownedVectors.size(); i++) {
        propertyVectors[i] = insertionRecord.ownedVectors[i].get();
    }
    KU_ASSERT(table.getPKColumnID() < insertionRecord.ownedVectors.size());
    auto& pkVector = *insertionRecord.ownedVectors[table.getPKColumnID()];
    const auto nodeIDVector = std::make_unique<ValueVector>(LogicalType::INTERNAL_ID());
    nodeIDVector->setState(anchorState);
    const auto insertState =
        std::make_unique<NodeTableInsertState>(*nodeIDVector, pkVector, propertyVectors);
    KU_ASSERT(transaction::Transaction::Get(clientContext) &&
              transaction::Transaction::Get(clientContext)->isRecovery());
    table.initInsertState(&clientContext, *insertState);
    anchorState->getSelVectorUnsafe().setToFiltered(1);
    for (auto i = 0u; i < numNodes; i++) {
        anchorState->getSelVectorUnsafe()[0] = i;
        table.insert(transaction::Transaction::Get(clientContext), *insertState);
    }
}

void WALReplayer::replayRelTableInsertRecord(const WALRecord& walRecord) const {
    const auto& insertionRecord = walRecord.constCast<TableInsertionRecord>();
    const auto tableID = insertionRecord.tableID;
    auto& table = StorageManager::Get(clientContext)->getTable(tableID)->cast<RelTable>();
    KU_ASSERT(!insertionRecord.ownedVectors.empty());
    const auto anchorState = insertionRecord.ownedVectors[0]->state;
    const auto numRels = anchorState->getSelVector().getSelSize();
    anchorState->getSelVectorUnsafe().setToFiltered(1);
    for (auto i = 0u; i < insertionRecord.ownedVectors.size(); i++) {
        insertionRecord.ownedVectors[i]->setState(anchorState);
    }
    std::vector<ValueVector*> propertyVectors;
    for (auto i = 0u; i < insertionRecord.ownedVectors.size(); i++) {
        if (i < LOCAL_REL_ID_COLUMN_ID) {
            // Skip the first two vectors which are the src nodeID and the dst nodeID.
            continue;
        }
        propertyVectors.push_back(insertionRecord.ownedVectors[i].get());
    }
    const auto insertState = std::make_unique<RelTableInsertState>(
        *insertionRecord.ownedVectors[LOCAL_BOUND_NODE_ID_COLUMN_ID],
        *insertionRecord.ownedVectors[LOCAL_NBR_NODE_ID_COLUMN_ID], propertyVectors);
    KU_ASSERT(transaction::Transaction::Get(clientContext) &&
              transaction::Transaction::Get(clientContext)->isRecovery());
    for (auto i = 0u; i < numRels; i++) {
        anchorState->getSelVectorUnsafe()[0] = i;
        table.initInsertState(&clientContext, *insertState);
        table.insert(transaction::Transaction::Get(clientContext), *insertState);
    }
}

void WALReplayer::replayNodeDeletionRecord(const WALRecord& walRecord) const {
    const auto& deletionRecord = walRecord.constCast<NodeDeletionRecord>();
    const auto tableID = deletionRecord.tableID;
    auto& table = StorageManager::Get(clientContext)->getTable(tableID)->cast<NodeTable>();
    const auto anchorState = deletionRecord.ownedPKVector->state;
    KU_ASSERT(anchorState->getSelVector().getSelSize() == 1);
    const auto nodeIDVector = std::make_unique<ValueVector>(LogicalType::INTERNAL_ID());
    nodeIDVector->setState(anchorState);
    nodeIDVector->setValue<internalID_t>(0,
        internalID_t{deletionRecord.nodeOffset, deletionRecord.tableID});
    const auto deleteState =
        std::make_unique<NodeTableDeleteState>(*nodeIDVector, *deletionRecord.ownedPKVector);
    KU_ASSERT(transaction::Transaction::Get(clientContext) &&
              transaction::Transaction::Get(clientContext)->isRecovery());
    table.delete_(transaction::Transaction::Get(clientContext), *deleteState);
    // Ce que l'exécuteur fait en fin de suppression, et que le rejeu oubliait : laisser les
    // index réparer ce que la ligne retirée reliait. Sans cela un index chargé au rejeu
    // perdait des lignes après une suppression rejouée.
    table.finalizeDelete(transaction::Transaction::Get(clientContext), *deleteState);
}

void WALReplayer::replayNodeUpdateRecord(const WALRecord& walRecord) const {
    const auto& updateRecord = walRecord.constCast<NodeUpdateRecord>();
    const auto tableID = updateRecord.tableID;
    auto& table = StorageManager::Get(clientContext)->getTable(tableID)->cast<NodeTable>();
    const auto anchorState = updateRecord.ownedPropertyVector->state;
    KU_ASSERT(anchorState->getSelVector().getSelSize() == 1);
    const auto nodeIDVector = std::make_unique<ValueVector>(LogicalType::INTERNAL_ID());
    nodeIDVector->setState(anchorState);
    nodeIDVector->setValue<internalID_t>(0,
        internalID_t{updateRecord.nodeOffset, updateRecord.tableID});
    const auto updateState = std::make_unique<NodeTableUpdateState>(updateRecord.columnID,
        *nodeIDVector, *updateRecord.ownedPropertyVector);
    KU_ASSERT(transaction::Transaction::Get(clientContext) &&
              transaction::Transaction::Get(clientContext)->isRecovery());
    // Sans cela la mise à jour rejouée ne dit rien aux index de la colonne : un index chargé
    // garderait l'ancienne valeur, un index non chargé ne serait pas détaché.
    table.initUpdateState(&clientContext, *updateState);
    table.update(transaction::Transaction::Get(clientContext), *updateState);
}

void WALReplayer::replayRelDeletionRecord(const WALRecord& walRecord) const {
    const auto& deletionRecord = walRecord.constCast<RelDeletionRecord>();
    const auto tableID = deletionRecord.tableID;
    auto& table = StorageManager::Get(clientContext)->getTable(tableID)->cast<RelTable>();
    const auto anchorState = deletionRecord.ownedRelIDVector->state;
    KU_ASSERT(anchorState->getSelVector().getSelSize() == 1);
    const auto deleteState =
        std::make_unique<RelTableDeleteState>(*deletionRecord.ownedSrcNodeIDVector,
            *deletionRecord.ownedDstNodeIDVector, *deletionRecord.ownedRelIDVector);
    KU_ASSERT(transaction::Transaction::Get(clientContext) &&
              transaction::Transaction::Get(clientContext)->isRecovery());
    table.delete_(transaction::Transaction::Get(clientContext), *deleteState);
}

void WALReplayer::replayRelDetachDeletionRecord(const WALRecord& walRecord) const {
    const auto& deletionRecord = walRecord.constCast<RelDetachDeleteRecord>();
    const auto tableID = deletionRecord.tableID;
    auto& table = StorageManager::Get(clientContext)->getTable(tableID)->cast<RelTable>();
    KU_ASSERT(transaction::Transaction::Get(clientContext) &&
              transaction::Transaction::Get(clientContext)->isRecovery());
    const auto anchorState = deletionRecord.ownedSrcNodeIDVector->state;
    KU_ASSERT(anchorState->getSelVector().getSelSize() == 1);
    const auto dstNodeIDVector =
        std::make_unique<ValueVector>(LogicalType{LogicalTypeID::INTERNAL_ID});
    const auto relIDVector = std::make_unique<ValueVector>(LogicalType{LogicalTypeID::INTERNAL_ID});
    dstNodeIDVector->setState(anchorState);
    relIDVector->setState(anchorState);
    const auto deleteState = std::make_unique<RelTableDeleteState>(
        *deletionRecord.ownedSrcNodeIDVector, *dstNodeIDVector, *relIDVector);
    deleteState->detachDeleteDirection = deletionRecord.direction;
    table.detachDelete(transaction::Transaction::Get(clientContext), deleteState.get());
}

void WALReplayer::replayRelUpdateRecord(const WALRecord& walRecord) const {
    const auto& updateRecord = walRecord.constCast<RelUpdateRecord>();
    const auto tableID = updateRecord.tableID;
    auto& table = StorageManager::Get(clientContext)->getTable(tableID)->cast<RelTable>();
    const auto anchorState = updateRecord.ownedRelIDVector->state;
    KU_ASSERT(anchorState == updateRecord.ownedSrcNodeIDVector->state &&
              anchorState == updateRecord.ownedSrcNodeIDVector->state &&
              anchorState == updateRecord.ownedPropertyVector->state);
    KU_ASSERT(anchorState->getSelVector().getSelSize() == 1);
    const auto updateState = std::make_unique<RelTableUpdateState>(updateRecord.columnID,
        *updateRecord.ownedSrcNodeIDVector, *updateRecord.ownedDstNodeIDVector,
        *updateRecord.ownedRelIDVector, *updateRecord.ownedPropertyVector);
    KU_ASSERT(transaction::Transaction::Get(clientContext) &&
              transaction::Transaction::Get(clientContext)->isRecovery());
    table.update(transaction::Transaction::Get(clientContext), *updateState);
}

void WALReplayer::replayCopyTableRecord(const WALRecord&) const {
    // DO NOTHING.
}

void WALReplayer::replayUpdateSequenceRecord(const WALRecord& walRecord) const {
    auto& sequenceEntryRecord = walRecord.constCast<UpdateSequenceRecord>();
    const auto sequenceID = sequenceEntryRecord.sequenceID;
    const auto entry =
        Catalog::Get(clientContext)
            ->getSequenceEntry(transaction::Transaction::Get(clientContext), sequenceID);
    entry->nextKVal(transaction::Transaction::Get(clientContext), sequenceEntryRecord.kCount);
}

void WALReplayer::replayLoadExtensionRecord(const WALRecord& walRecord) const {
    const auto& loadExtensionRecord = walRecord.constCast<LoadExtensionRecord>();
    // Une extension dont le fichier a disparu n'empêche pas d'ouvrir la base : la reprise
    // continue sans elle, et un index qu'elle n'a pas pu tenir à jour se déclare à rebâtir.
    extension::ExtensionManager::Get(clientContext)
        ->loadExtensionForRecovery("" /* nom inconnu du journal */, loadExtensionRecord.path,
            &clientContext);
}

void WALReplayer::removeWALAndShadowFiles() const {
    removeFileIfExists(shadowFilePath);
    removeFileIfExists(walPath);
}

void WALReplayer::removeFileIfExists(const std::string& path) const {
    if (StorageManager::Get(clientContext)->isReadOnly()) {
        return;
    }
    auto vfs = VirtualFileSystem::GetUnsafe(clientContext);
    if (vfs->fileOrPathExists(path, &clientContext)) {
        vfs->removeFileIfExists(path);
    }
}

std::unique_ptr<FileInfo> WALReplayer::openWALFile() const {
    auto flag = FileFlags::READ_ONLY;
    if (!StorageManager::Get(clientContext)->isReadOnly()) {
        flag |= FileFlags::WRITE; // The write flag here is to ensure the file is opened with O_RDWR
                                  // so that we can sync it.
    }
    return VirtualFileSystem::GetUnsafe(clientContext)->openFile(walPath, FileOpenFlags(flag));
}

void WALReplayer::syncWALFile(const FileInfo& fileInfo) const {
    if (StorageManager::Get(clientContext)->isReadOnly()) {
        return;
    }
    fileInfo.syncFile();
}

// What follows the last complete COMMIT is about to be truncated: copy it next
// to the journal first, and say so. An absence is named. Without record lengths
// a corrupted length can read as a torn end, so the message does not claim the
// bytes were uncommitted.
void WALReplayer::setAsideCutBytes(FileInfo& fileInfo, uint64_t offsetDeserialized,
    bool tornEnd) const {
    const auto fileSize = fileInfo.getFileSize();
    if (fileSize <= offsetDeserialized) {
        return;
    }
    const auto cut = fileSize - offsetDeserialized;
    const auto why = tornEnd ? "the journal ends inside a record or without a COMMIT" :
                               "a record could not be replayed";
    // A reader opening while a live writer appends sees an incomplete end:
    // that is normal, not a tear, and it happens on every concurrent open.
    // Nothing is written or removed in read-only mode, so nothing is said.
    if (StorageManager::Get(clientContext)->isReadOnly()) {
        return;
    }
    const auto millis = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::system_clock::now().time_since_epoch())
                            .count();
    const auto asidePath = stringFormat("{}.ecarte-{}", walPath, millis);
    auto aside = VirtualFileSystem::GetUnsafe(clientContext)
                     ->openFile(asidePath, FileOpenFlags(FileFlags::WRITE |
                                                         FileFlags::CREATE_AND_TRUNCATE_IF_EXISTS));
    // By blocks: after a bulk ingestion the cut can be large.
    constexpr uint64_t BLOCK_SIZE = 1 << 20;
    std::vector<uint8_t> block(std::min(cut, BLOCK_SIZE));
    for (uint64_t done = 0; done < cut;) {
        const auto size = std::min(cut - done, BLOCK_SIZE);
        fileInfo.readFromFile(block.data(), size, offsetDeserialized + done);
        aside->writeFile(block.data(), size, done);
        done += size;
    }
    aside->syncFile();
    std::cerr << stringFormat("rag3db: WAL {}: {} bytes after offset {} were cut from the journal "
                              "({}) and copied to {}. They may hold committed transactions if the "
                              "journal was corrupted rather than cut; nothing was deleted.\n",
        walPath, cut, offsetDeserialized, why, asidePath);
}

void WALReplayer::truncateWALFile(FileInfo& fileInfo, uint64_t size) const {
    if (StorageManager::Get(clientContext)->isReadOnly()) {
        return;
    }
    if (fileInfo.getFileSize() > size) {
        fileInfo.truncate(size);
        fileInfo.syncFile();
    }
}

} // namespace storage
} // namespace rag3db
