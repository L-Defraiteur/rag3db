#include "storage/page_manager.h"

#include <cstdio>

#include "common/stack_to_stderr.h"
#include "common/uniq_lock.h"
#include "storage/file_handle.h"
#include "storage/storage_manager.h"

namespace rag3db::storage {
static constexpr bool ENABLE_FSM = true;

// Une plage qui dépasse le fichier n'a rien à faire dans l'espace libre : la servir ferait écrire
// au-delà de numPages (un trou dans le fichier, des états de page absents pour les lecteurs).
static bool beyondTheFile(const PageRange& entry, const FileHandle& fileHandle) {
    return static_cast<uint64_t>(entry.startPageIdx) + entry.numPages > fileHandle.getNumPages();
}

static void reportBeyondTheFile(const char* what, const PageRange& entry,
    const FileHandle& fileHandle) {
    fprintf(stderr,
        "page manager: %s refused, pages [%u, +%u) lie beyond the %u pages of the data file\n",
        what, entry.startPageIdx, entry.numPages, fileHandle.getNumPages());
    common::printStackToStderr(what);
}

PageRange PageManager::allocatePageRange(common::page_idx_t numPages) {
    if constexpr (ENABLE_FSM) {
        common::UniqLock lck{mtx};
        auto allocatedFreeChunk = freeSpaceManager->popFreePages(numPages);
        while (allocatedFreeChunk.has_value() && beyondTheFile(*allocatedFreeChunk, *fileHandle)) {
            // L'entrée fausse est retirée de la liste (popFreePages en a remis le reste, qui
            // dépasse lui aussi et sera retiré à son tour s'il est servi).
            reportBeyondTheFile("a free range to allocate from", *allocatedFreeChunk, *fileHandle);
            ++version;
            allocatedFreeChunk = freeSpaceManager->popFreePages(numPages);
        }
        if (allocatedFreeChunk.has_value()) {
            ++version;
            return {*allocatedFreeChunk};
        }
    }
    auto startPageIdx = fileHandle->addNewPages(numPages);
    KU_ASSERT(fileHandle->getNumPages() >= startPageIdx + numPages);
    return PageRange(startPageIdx, numPages);
}

void PageManager::freePageRange(PageRange entry) {
    if constexpr (ENABLE_FSM) {
        common::UniqLock lck{mtx};
        if (beyondTheFile(entry, *fileHandle)) {
            // Appelée aussi depuis un destructeur (~ColumnChunkData) : pas d'exception ici.
            reportBeyondTheFile("a range to free", entry, *fileHandle);
            return;
        }
        // Freed pages cannot be immediately reused to ensure checkpoint recovery works
        // Instead they are reusable after the end of the next checkpoint
        freeSpaceManager->addUncheckpointedFreePages(entry);
        ++version;
    }
}

common::page_idx_t PageManager::estimatePagesNeededForSerialize() {
    return freeSpaceManager->getMaxNumPagesForSerialization();
}

void PageManager::freeImmediatelyRewritablePageRange(FileHandle* fileHandle, PageRange entry) {
    if constexpr (ENABLE_FSM) {
        common::UniqLock lck{mtx};
        if (beyondTheFile(entry, *fileHandle)) {
            reportBeyondTheFile("a range to free at once", entry, *fileHandle);
            return;
        }
        freeSpaceManager->evictAndAddFreePages(fileHandle, entry);
        ++version;
    }
}

void PageManager::serialize(common::Serializer& serializer) {
    freeSpaceManager->serialize(serializer);
}

void PageManager::deserialize(common::Deserializer& deSer) {
    freeSpaceManager->deserialize(deSer);
}

// Under the same lock as freePageRange: a query thread that drops the last retired segment frees its
// pages (addUncheckpointedFreePages) while the checkpoint evicts, moves and clears that same list.
// Unordered, a range pushed between the move and the clear was lost, and a push_back that reallocated
// during the move wrote into a block malloc may have handed out again (ticket 2026-11-21-le-point-
// de-reprise-vide-les-pages-libres-non-reprises-sans-verrou, seen by TSan under DdlEveryMinute).
// Lock order: mtx, then the buffer manager's frames (evictPages), as freeImmediatelyRewritablePageRange
// already does; nothing under mtx takes a checkpoint or transaction manager lock.
void PageManager::finalizeCheckpoint() {
    common::UniqLock lck{mtx};
    freeSpaceManager->finalizeCheckpoint(fileHandle);
}

void PageManager::rollbackCheckpoint() {
    common::UniqLock lck{mtx};
    freeSpaceManager->rollbackCheckpoint();
}

void PageManager::clearEvictedBMEntriesIfNeeded(BufferManager* bufferManager) {
    freeSpaceManager->clearEvictedBufferManagerEntriesIfNeeded(bufferManager);
}

PageManager* PageManager::Get(const main::ClientContext& context) {
    return StorageManager::Get(context)->getDataFH()->getPageManager();
}

} // namespace rag3db::storage
