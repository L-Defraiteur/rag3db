#pragma once

#include "storage/buffer_manager/memory_manager.h"
#include "storage/file_handle.h"

namespace rag3db {
namespace common {
class VirtualFileSystem;
};
namespace storage {
class InMemChunkedNodeGroup;

class BufferManager;
class ColumnChunkData;

// This should only be used with a LocalFileSystem
class Spiller {
public:
    Spiller(std::string tmpFilePath, BufferManager& bufferManager, common::VirtualFileSystem* vfs);
    void addUnusedChunk(InMemChunkedNodeGroup* nodeGroup);
    void clearUnusedChunk(InMemChunkedNodeGroup* nodeGroup);
    SpillResult spillToDisk(ColumnChunkData& chunk) const;
    void loadFromDisk(ColumnChunkData& chunk) const;
    // reclaims memory from the next full partitioner group in the set
    // and returns the amount of memory reclaimed
    // If the set is empty, returns zero
    SpillResult claimNextGroup();
    // Empties the spill file when no spilled buffer is left on it. Every query calls it when it
    // ends, while a query of another connection may still have groups spilled (16 Nov. 2026): the
    // file is then kept as it is, and emptied by a later call once they are all read back or freed.
    void clearFile();
    // A spilled buffer freed without being read back (its group was dropped).
    void forgetBufferOnDisk() const;
    ~Spiller();

private:
    FileHandle* getOrCreateDataFH() const;
    FileHandle* getDataFH() const;

private:
    std::string tmpFilePath;
    BufferManager& bufferManager;
    common::VirtualFileSystem* vfs;
    std::unordered_set<InMemChunkedNodeGroup*> fullPartitionerGroups;
    std::atomic<FileHandle*> dataFH;
    std::mutex partitionerGroupsMtx;
    mutable std::mutex fileCreationMutex;
    // Held while a buffer is written to the file and while the file is emptied: a spill and a clear
    // never interleave.
    mutable std::mutex fileMtx;
    // The buffers written to the file and neither read back nor freed yet.
    mutable std::atomic<uint64_t> numBuffersOnDisk{0};
};

} // namespace storage
} // namespace rag3db
