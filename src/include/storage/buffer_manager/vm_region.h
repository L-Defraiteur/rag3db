#pragma once

#include <array>
#include <atomic>
#include <mutex>

#include "common/constants.h"
#include "common/types/types.h"

namespace rag3db {
namespace storage {

// A VMRegion holds a virtual memory region of a certain size allocated through mmap.
// The region is divided into frame groups, each of which is a group of frames of the same size.
// Each FileHandle should grab a frame group each time when they add a new file page group (see
// `FileHandle::addNewPageGroupWithoutLock`). In this way, each file page group uniquely
// corresponds to a frame group, thus, a page also uniquely corresponds to a frame in a VMRegion.
//
// La région se réserve par tranches de taille fixe, à mesure que les groupes de cadres
// s'ajoutent, au lieu d'un seul bloc de sa taille maximale (8 Tio par défaut) : un processus
// n'ouvrait qu'une quinzaine de bases sur disque (ticket 2026-10-19-une-base-reserve-8-tio-d-
// adressage-un-processus-n-en-ouvre-qu-une-quinzaine). Une tranche est une puissance de deux et
// un multiple de la taille d'un groupe de cadres : un groupe ne chevauche jamais deux tranches,
// et l'adresse d'un cadre se calcule par un décalage et un masque. La table des tranches est
// écrite (release) avant que addNewFrameGroup ne rende l'indice, et lue (acquire) sans verrou.
class VMRegion {
    friend class BufferManager;

public:
    // 16 Gio : une base de 50 Gio en réserve 64, une petite 16 (cœur C++, 19 octobre 2026).
    static constexpr uint64_t DEFAULT_CHUNK_SIZE = static_cast<uint64_t>(1) << 34;
    static constexpr uint64_t MAX_NUM_CHUNKS = 1024;

    VMRegion(common::PageSizeClass pageSizeClass, uint64_t maxRegionSize,
        uint64_t chunkSize = DEFAULT_CHUNK_SIZE);
    virtual ~VMRegion();

    common::frame_group_idx_t addNewFrameGroup();

    // Use `MADV_DONTNEED` to release physical memory associated with this frame.
    void releaseFrame(common::frame_idx_t frameIdx) const;

    // Returns true if the memory address is within the reserved virtual memory region
    bool contains(const uint8_t* address) const;
    inline uint8_t* getFrame(common::frame_idx_t frameIdx) const {
        const auto idx = static_cast<std::uint64_t>(frameIdx);
        return chunks[idx >> framesPerChunkLog2].load(std::memory_order_acquire) +
               (idx & (framesPerChunk - 1)) * frameSize;
    }
    uint64_t getChunkSize() const { return chunkSize; }

protected:
    // Réserve une tranche d'adresses (sans mémoire) ; nul si le système la refuse.
    virtual uint8_t* reserveChunk(uint64_t size);

private:
    static void releaseChunk(uint8_t* chunk, uint64_t size);

private:
    std::mutex mtx;
    uint32_t frameSize;
    uint64_t chunkSize;
    uint64_t framesPerChunk;
    uint32_t framesPerChunkLog2;
    uint64_t frameGroupsPerChunk;
    uint64_t numFrameGroups;
    uint64_t maxNumFrameGroups;
    uint64_t numChunks;
    std::array<std::atomic<uint8_t*>, MAX_NUM_CHUNKS> chunks;
};

} // namespace storage
} // namespace rag3db
