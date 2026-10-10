#include "storage/checkpointer.h"

#include <cstdio>

#include "storage/checkpoint_profile.h"

#include "catalog/catalog.h"
#include "common/file_system/file_system.h"
#include "common/file_system/virtual_file_system.h"
#include "common/serializer/buffered_file.h"
#include "common/serializer/deserializer.h"
#include "common/serializer/in_mem_file_writer.h"
#include "extension/extension_manager.h"
#include "main/client_context.h"
#include "main/db_config.h"
#include "storage/buffer_manager/buffer_manager.h"
#include "storage/database_header.h"
#include "storage/readers_lock.h"
#include "storage/shadow_utils.h"
#include "storage/storage_manager.h"
#include "storage/wal/local_wal.h"

namespace rag3db {
namespace storage {

Checkpointer::Checkpointer(main::ClientContext& clientContext)
    : clientContext{clientContext},
      isInMemory{main::DBConfig::isDBPathInMemory(clientContext.getDatabasePath())} {}

Checkpointer::~Checkpointer() = default;

PageRange Checkpointer::serializeCatalog(const catalog::Catalog& catalog,
    StorageManager& storageManager) {
    auto catalogWriter =
        std::make_shared<common::InMemFileWriter>(*MemoryManager::Get(clientContext));
    common::Serializer catalogSerializer(catalogWriter);
    catalog.serialize(catalogSerializer);
    auto pageAllocator = storageManager.getDataFH()->getPageManager();
    return catalogWriter->flush(*pageAllocator, storageManager.getShadowFile());
}

PageRange Checkpointer::serializeMetadata(const catalog::Catalog& catalog,
    StorageManager& storageManager) {
    auto metadataWriter =
        std::make_shared<common::InMemFileWriter>(*MemoryManager::Get(clientContext));
    common::Serializer metadataSerializer(metadataWriter);
    storageManager.serialize(catalog, metadataSerializer);

    // We need to preallocate the pages for the page manager before we actually serialize it,
    // this is because the page manager needs to track the pages used for itself.
    // The number of pages needed for the page manager should only decrease after making an
    // additional allocation, so we just calculate the number of pages needed to serialize the
    // current state of the page manager.
    // Thus, it is possible that we allocate an extra page that we won't end up writing to when we
    // flush the metadata writer. This may cause a discrepancy between the number of tracked pages
    // and the number of physical pages in the file but shouldn't cause any actual incorrect
    // behavior in the database.
    auto& pageManager = *storageManager.getDataFH()->getPageManager();
    const auto pagesForPageManager = pageManager.estimatePagesNeededForSerialize();
    auto pageAllocator = storageManager.getDataFH()->getPageManager();
    const auto allocatedPages = pageAllocator->allocatePageRange(
        metadataWriter->getNumPagesToFlush() + pagesForPageManager);
    pageManager.serialize(metadataSerializer);

    metadataWriter->flush(allocatedPages, pageAllocator->getDataFH(),
        storageManager.getShadowFile());
    return allocatedPages;
}

void Checkpointer::writeCheckpoint() {
    if (isInMemory) {
        return;
    }

    auto databaseHeader =
        *StorageManager::Get(clientContext)->getOrInitDatabaseHeader(clientContext);
    // Checkpoint storage. Note that we first checkpoint storage before serializing the catalog, as
    // checkpointing storage may overwrite columnIDs in the catalog.
    CheckpointProfile profile;
    bool hasStorageChanges = checkpointStorage();
    const auto storageMs = profile.lap();
    serializeCatalogAndMetadata(databaseHeader, hasStorageChanges);
    const auto serializeMs = profile.lap();
    writeDatabaseHeader(databaseHeader);
    const auto headerMs = profile.lap();
    logCheckpointAndApplyShadowPages();
    const auto applyMs = profile.lap();

    // This function will evict all pages that were freed during this checkpoint
    // It must be called before we remove all evicted candidates from the BM
    // Or else the evicted pages may end up appearing multiple times in the eviction queue
    auto storageManager = StorageManager::Get(clientContext);
    storageManager->finalizeCheckpoint();
    // When a page is freed by the FSM, it evicts it from the BM. However, if the page is freed,
    // then reused over and over, it can be appended to the eviction queue multiple times. To
    // prevent multiple entries of the same page from existing in the eviction queue, at the end of
    // each checkpoint we remove any already-evicted pages.
    auto bufferManager = MemoryManager::Get(clientContext)->getBufferManager();
    bufferManager->removeEvictedCandidates();

    catalog::Catalog::Get(clientContext)->resetVersion();
    auto* dataFH = storageManager->getDataFH();
    dataFH->getPageManager()->resetVersion();
    storageManager->getWAL().reset();
    storageManager->getShadowFile().reset();
    // Le journal est supprimé : une ouverture en lecture seule trouve maintenant l'état d'après.
    const auto readersWaitMs = readersLock.waitedMs();
    readersLock.release();
    if (CheckpointProfile::enabled()) {
        const auto endMs = profile.lap();
        fprintf(stderr,
            "[checkpoint-profile] total=%.1f ms : tables=%.1f catalogue+metadonnees=%.1f "
            "en-tete=%.1f journal+pages-ombres=%.1f fin=%.1f | fichier=%lu pages | attente des "
            "lecteurs=%lu ms\n",
            profile.total(), storageMs, serializeMs, headerMs, applyMs, endMs,
            static_cast<unsigned long>(dataFH->getNumPages()),
            static_cast<unsigned long>(readersWaitMs));
    }
}

void Checkpointer::holdReadersLock() {
    if (isInMemory) {
        return;
    }
    const auto databasePath = StorageManager::Get(clientContext)->getDatabasePath();
    readersLock = ReadersLock::acquire(databasePath, ReadersLock::Mode::EXCLUSIVE,
        ReadersLock::CHECKPOINT_WAIT);
    if (readersLock.timedOut()) {
        // Une ouverture tient le verrou au-delà de la borne : un lecteur bloqué, ou une base
        // ouverte en lecture seule dans ce même processus. Le point de reprise passe ; si
        // l'ouverture lit encore, le constat d'après coup la refuse
        // (CHECKPOINT_CROSSED_READ_ONLY_OPEN).
        fprintf(stderr,
            "checkpoint of %s waited %lu ms for read-only openings of the database and went "
            "ahead without them\n",
            databasePath.c_str(), static_cast<unsigned long>(readersLock.waitedMs()));
    }
}

bool Checkpointer::checkpointStorage() {
    const auto storageManager = StorageManager::Get(clientContext);
    auto pageAllocator = storageManager->getDataFH()->getPageManager();
    return storageManager->checkpoint(&clientContext, *pageAllocator);
}

void Checkpointer::serializeCatalogAndMetadata(DatabaseHeader& databaseHeader,
    bool hasStorageChanges) {
    const auto storageManager = StorageManager::Get(clientContext);
    const auto catalog = catalog::Catalog::Get(clientContext);
    auto* dataFH = storageManager->getDataFH();

    CheckpointProfile profile;
    auto catalogPages = 0u, metadataPages = 0u;
    // Serialize the catalog if there are changes
    if (databaseHeader.catalogPageRange.startPageIdx == common::INVALID_PAGE_IDX ||
        catalog->changedSinceLastCheckpoint()) {
        databaseHeader.updateCatalogPageRange(*dataFH->getPageManager(),
            serializeCatalog(*catalog, *storageManager));
        catalogPages = databaseHeader.catalogPageRange.numPages;
    }
    const auto catalogMs = profile.lap();
    // Serialize the storage metadata if there are changes
    if (databaseHeader.metadataPageRange.startPageIdx == common::INVALID_PAGE_IDX ||
        hasStorageChanges || catalog->changedSinceLastCheckpoint() ||
        dataFH->getPageManager()->changedSinceLastCheckpoint()) {
        // We must free the existing metadata page range before serializing
        // So that the freed pages are serialized by the FSM
        databaseHeader.freeMetadataPageRange(*dataFH->getPageManager());
        databaseHeader.metadataPageRange = serializeMetadata(*catalog, *storageManager);
        metadataPages = databaseHeader.metadataPageRange.numPages;
    }
    if (CheckpointProfile::enabled()) {
        const auto metadataMs = profile.lap();
        fprintf(stderr,
            "[checkpoint-profile]   catalogue+metadonnees=%.1f ms : catalogue=%.1f (%u pages) "
            "metadonnees=%.1f (%u pages)\n",
            profile.total(), catalogMs, catalogPages, metadataMs, metadataPages);
    }
}

void Checkpointer::writeDatabaseHeader(const DatabaseHeader& headerWithoutExtent) {
    const auto storageManager = StorageManager::Get(clientContext);
    auto dataFH = storageManager->getDataFH();
    // L'étendue du fichier à cet instant : tout ce que ce point de reprise désigne — les pages
    // des tables, des index, du catalogue et des métadonnées — est déjà alloué, donc en deçà.
    // L'en-tête passe par une page fantôme : l'étendue est atomique avec le point de reprise.
    auto header = headerWithoutExtent;
    header.numDataPages = dataFH->getNumPages();
    auto headerWriter =
        std::make_shared<common::InMemFileWriter>(*MemoryManager::Get(clientContext));
    common::Serializer headerSerializer(headerWriter);
    header.serialize(headerSerializer);
    auto headerPage = headerWriter->getPage(0);

    auto& shadowFile = storageManager->getShadowFile();
    auto shadowHeader = ShadowUtils::createShadowVersionIfNecessaryAndPinPage(
        common::StorageConstants::DB_HEADER_PAGE_IDX, true /* skipReadingOriginalPage */, *dataFH,
        shadowFile);
    memcpy(shadowHeader.frame, headerPage.data(), common::RAG3DB_PAGE_SIZE);
    shadowFile.getShadowingFH().unpinPage(shadowHeader.shadowPage);

    // Update the in-memory database header with the new version
    StorageManager::Get(clientContext)->setDatabaseHeader(std::make_unique<DatabaseHeader>(header));
}

void Checkpointer::logCheckpointAndApplyShadowPages() {
    const auto storageManager = StorageManager::Get(clientContext);
    auto& shadowFile = storageManager->getShadowFile();
    CheckpointProfile profile;
    const auto numShadowPages = shadowFile.getNumShadowPages();
    // Flush the shadow file.
    shadowFile.flushAll(clientContext);
    const auto flushMs = profile.lap();
    // Les pages neuves des colonnes et des débordements s'écrivent directement dans le fichier
    // de données, sans page fantôme ; l'en-tête et les métadonnées qui les désignent passent par
    // le fichier fantôme. Ces pages doivent être durables avant la marque CHECKPOINT : après
    // elle, la reprise rejoue l'en-tête et supprime le journal, et une page perdue ne pourrait
    // plus être refaite.
    storageManager->getDataFH()->getFileInfo()->syncFile();
    const auto syncDataMs = profile.lap();
    auto wal = WAL::Get(clientContext);
    // La marque CHECKPOINT est le premier changement qu'une ouverture en lecture seule verrait :
    // jusqu'ici, rien de ce que désignent l'en-tête et le journal courants n'a été touché.
    holdReadersLock();
    // Log the checkpoint to the WAL and flush WAL. This indicates that all shadow pages and
    // files (snapshots of catalog and metadata) have been written to disk. The part that is not
    // done is to replace them with the original pages or catalog and metadata files. If the
    // system crashes before this point, the WAL can still be used to recover the system to a
    // state where the checkpoint can be redone.
    wal->logAndFlushCheckpoint(&clientContext);
    const auto walMs = profile.lap();
    shadowFile.applyShadowPages(clientContext);
    const auto applyMs = profile.lap();
    // Clear the wal and also shadowing files.
    auto bufferManager = MemoryManager::Get(clientContext)->getBufferManager();
    wal->clear();
    shadowFile.clear(*bufferManager);
    if (CheckpointProfile::enabled()) {
        const auto clearMs = profile.lap();
        fprintf(stderr,
            "[checkpoint-profile]   journal+pages-ombres=%.1f ms : ecriture-ombres=%.1f (%lu "
            "pages) synchro-donnees=%.1f marque-au-journal=%.1f application=%.1f vidage=%.1f\n",
            profile.total(), flushMs, static_cast<unsigned long>(numShadowPages), syncDataMs,
            walMs, applyMs, clearMs);
    }
}

void Checkpointer::rollback() {
    if (isInMemory) {
        return;
    }
    const auto storageManager = StorageManager::Get(clientContext);
    auto catalog = catalog::Catalog::Get(clientContext);
    // Any pages freed during the checkpoint are no longer freed
    storageManager->rollbackCheckpoint(*catalog);
    readersLock.release();
}

bool Checkpointer::canAutoCheckpoint(const main::ClientContext& clientContext,
    const transaction::Transaction& transaction) {
    if (clientContext.isInMemory()) {
        return false;
    }
    if (!clientContext.getDBConfig()->autoCheckpoint) {
        return false;
    }
    if (transaction.isRecovery()) {
        // Recovery transactions are not allowed to trigger auto checkpoint.
        return false;
    }
    auto wal = WAL::Get(clientContext);
    const auto expectedSize = transaction.getLocalWAL().getSize() + wal->getFileSize();
    return expectedSize > clientContext.getDBConfig()->checkpointThreshold;
}

void Checkpointer::readCheckpoint() {
    auto storageManager = StorageManager::Get(clientContext);
    storageManager->initDataFileHandle(common::VirtualFileSystem::GetUnsafe(clientContext),
        &clientContext);
    if (!isInMemory && storageManager->getDataFH()->getNumPages() > 0) {
        readCheckpoint(&clientContext, catalog::Catalog::Get(clientContext), storageManager);
    }
    extension::ExtensionManager::Get(clientContext)->autoLoadLinkedExtensions(&clientContext);
}

void Checkpointer::readCheckpoint(main::ClientContext* context, catalog::Catalog* catalog,
    StorageManager* storageManager) {
    auto fileInfo = storageManager->getDataFH()->getFileInfo();
    auto reader = std::make_unique<common::BufferedFileReader>(*fileInfo);
    common::Deserializer deSer(std::move(reader));
    auto currentHeader = std::make_unique<DatabaseHeader>(DatabaseHeader::deserialize(deSer));
    // If the catalog page range is invalid, it means there is no catalog to read; thus, the
    // database is empty.
    if (currentHeader->catalogPageRange.startPageIdx != common::INVALID_PAGE_IDX) {
        deSer.getReader()->cast<common::BufferedFileReader>()->resetReadOffset(
            currentHeader->catalogPageRange.startPageIdx * common::RAG3DB_PAGE_SIZE);
        catalog->deserialize(deSer);
        deSer.getReader()->cast<common::BufferedFileReader>()->resetReadOffset(
            currentHeader->metadataPageRange.startPageIdx * common::RAG3DB_PAGE_SIZE);
        storageManager->deserialize(context, catalog, deSer);
        storageManager->getDataFH()->getPageManager()->deserialize(deSer);
    }
    returnOwnerlessPages(*currentHeader, storageManager);
    storageManager->setDatabaseHeader(std::move(currentHeader));
}

// Ce que le fichier porte au-delà de l'étendue que le dernier point de reprise a écrite est à
// personne : ni l'en-tête, ni l'espace libre, ni le journal — qui est logique — ne désignent une
// page physique au-delà. Un COPY tué avant sa validation (ou validé par le journal, rejoué plus
// loin), un point de reprise interrompu avant sa marque, une queue qu'un point de reprise a
// libérée sans tronquer le fichier : leurs pages restaient comptées, perdues pour toujours. Elles
// sont rendues à l'espace libre, réutilisables tout de suite, et le point de reprise suivant les
// persiste libres. Rien n'est tronqué (comme PostgreSQL : rendre, ne pas couper). À l'ouverture en
// écriture seulement ; après l'application des pages fantômes, qui a lieu avant cette lecture.
// Mesuré avant : un COPY de 200 000 lignes tué avant sa validation laissait 554 pages.
void Checkpointer::returnOwnerlessPages(const DatabaseHeader& header,
    StorageManager* storageManager) {
    if (storageManager->isReadOnly() || header.numDataPages == common::INVALID_PAGE_IDX) {
        return;
    }
    auto dataFH = storageManager->getDataFH();
    const auto numPagesInFile = dataFH->getNumPages();
    if (numPagesInFile <= header.numDataPages) {
        // Rien au-delà ; ou le fichier est plus court que l'étendue (une page allouée par le point
        // de reprise et jamais écrite) : rien à rendre.
        return;
    }
    dataFH->getPageManager()->freeImmediatelyRewritablePageRange(dataFH,
        PageRange{header.numDataPages, numPagesInFile - header.numDataPages});
}

} // namespace storage
} // namespace rag3db
