#include "storage/buffer_manager/vm_region.h"

#include <algorithm>

#include "common/assert.h"
#include "common/string_format.h"
#include "common/system_config.h"
#include "common/system_message.h"

#ifdef _WIN32
#include <errhandlingapi.h>
#include <handleapi.h>
#include <memoryapi.h>
#else
#include <sys/mman.h>
#endif

#include "common/exception/buffer_manager.h"

using namespace rag3db::common;

namespace rag3db {
namespace storage {

namespace {
uint64_t nextPowerOfTwo(uint64_t value) {
    uint64_t power = 1;
    while (power < value) {
        power <<= 1;
    }
    return power;
}
} // namespace

VMRegion::VMRegion(PageSizeClass pageSizeClass, uint64_t maxRegionSize, uint64_t chunkSize_)
    : numFrameGroups{0}, numChunks{0} {
    if (maxRegionSize > static_cast<std::size_t>(-1)) {
        throw BufferManagerException("maxRegionSize is beyond the max available mmap region size.");
    }
    frameSize = pageSizeClass == REGULAR_PAGE ? RAG3DB_PAGE_SIZE : TEMP_PAGE_SIZE;
    const auto numBytesForFrameGroup = frameSize * StorageConstants::PAGE_GROUP_SIZE;
    maxNumFrameGroups = (maxRegionSize + numBytesForFrameGroup - 1) / numBytesForFrameGroup;
    // La tranche : une puissance de deux, au moins un groupe de cadres, au plus ce que la région
    // demande, et assez grande pour que MAX_NUM_CHUNKS tranches couvrent la région.
    const auto regionBytes = std::max<uint64_t>(maxNumFrameGroups, 1) * numBytesForFrameGroup;
    chunkSize = nextPowerOfTwo(std::max<uint64_t>(chunkSize_, numBytesForFrameGroup));
    chunkSize = std::min(chunkSize, nextPowerOfTwo(regionBytes));
    chunkSize = std::max(chunkSize, nextPowerOfTwo((regionBytes + MAX_NUM_CHUNKS - 1) / MAX_NUM_CHUNKS));
    framesPerChunk = chunkSize / frameSize;
    framesPerChunkLog2 = 0;
    while ((static_cast<uint64_t>(1) << framesPerChunkLog2) < framesPerChunk) {
        framesPerChunkLog2++;
    }
    frameGroupsPerChunk = chunkSize / numBytesForFrameGroup;
    for (auto& chunk : chunks) {
        chunk.store(nullptr, std::memory_order_relaxed);
    }
}

VMRegion::~VMRegion() {
    for (auto& chunk : chunks) {
        if (auto* reserved = chunk.load(std::memory_order_acquire)) {
            releaseChunk(reserved, chunkSize);
        }
    }
}

uint8_t* VMRegion::reserveChunk(uint64_t size) {
#ifdef _WIN32
    return static_cast<uint8_t*>(VirtualAlloc(NULL, size, MEM_RESERVE, PAGE_READWRITE));
#else
    // Create a private anonymous mapping. The mapping is not shared with other processes and not
    // backed by any file, and its content are initialized to zero.
    auto* chunk = mmap(NULL, size, PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1 /* fd */, 0 /* offset */);
    return chunk == MAP_FAILED ? nullptr : static_cast<uint8_t*>(chunk);
#endif
}

void VMRegion::releaseChunk(uint8_t* chunk, [[maybe_unused]] uint64_t size) {
#ifdef _WIN32
    VirtualFree(chunk, 0, MEM_RELEASE);
#else
    munmap(chunk, size);
#endif
}

bool VMRegion::contains(const uint8_t* address) const {
    for (const auto& chunk : chunks) {
        const auto* reserved = chunk.load(std::memory_order_acquire);
        if (reserved == nullptr) {
            return false;
        }
        if (address >= reserved && address < reserved + chunkSize) {
            return true;
        }
    }
    return false;
}

void VMRegion::releaseFrame(frame_idx_t frameIdx) const {
#ifdef _WIN32
    // TODO: VirtualAlloc(..., MEM_RESET, ...) may be faster
    // See https://arvid.io/2018/04/02/memory-mapping-on-windows/#1
    // Not sure what the differences are
    if (!VirtualFree(getFrame(frameIdx), frameSize, MEM_DECOMMIT)) {
        auto code = GetLastError();
        throw BufferManagerException(stringFormat(
            "Releasing physical memory associated with a frame failed with error code {}: {}.",
            code, systemErrMessage(code)));
    }

#else
    int error = madvise(getFrame(frameIdx), frameSize, MADV_DONTNEED);
    if (error != 0) {
        // LCOV_EXCL_START
        throw BufferManagerException(stringFormat(
            "Releasing physical memory associated with a frame failed with error code {}: {}.",
            error, posixErrMessage()));
        // LCOV_EXCL_STOP
    }
#endif
}

frame_group_idx_t VMRegion::addNewFrameGroup() {
    std::unique_lock xLck{mtx};
    if (numFrameGroups >= maxNumFrameGroups) {
        // LCOV_EXCL_START
        throw BufferManagerException("No more frame groups can be added to the allocator.");
        // LCOV_EXCL_STOP
    }
    const auto chunkIdx = numFrameGroups / frameGroupsPerChunk;
    if (chunkIdx >= numChunks) {
        KU_ASSERT(chunkIdx == numChunks && chunkIdx < MAX_NUM_CHUNKS);
        auto* reserved = reserveChunk(chunkSize);
        if (reserved == nullptr) {
            // Une réservation refusée en cours de route, et non plus à l'ouverture : un refus du
            // tampon comme un autre. Avant tout changement du point de reprise, il ne condamne pas
            // la base (StorageManager::markCheckpointChangedMemory).
            throw BufferManagerException(stringFormat(
                "Cannot reserve {} GiB more of address space for this database: the process has "
                "no more virtual address space.",
                chunkSize >> 30));
        }
        chunks[chunkIdx].store(reserved, std::memory_order_release);
        numChunks++;
    }
    return numFrameGroups++;
}

} // namespace storage
} // namespace rag3db
