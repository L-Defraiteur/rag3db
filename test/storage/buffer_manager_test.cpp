#include <cstdint>
#include <memory>
#include <vector>

#include "common/constants.h"
#include "common/exception/buffer_manager.h"
#include "common/system_config.h"
#include "common/types/types.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "gtest/gtest.h"
#include "spdlog/spdlog.h"
#include "storage/buffer_manager/buffer_manager.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/buffer_manager/spiller.h"
#include "storage/buffer_manager/vm_region.h"
#include "storage/enums/residency_state.h"
#include "storage/storage_manager.h"
#include "storage/table/chunked_node_group.h"
#include "storage/table/column_chunk.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace rag3db {
namespace testing {

class BufferManagerTest : public DBTest {
public:
    std::string getInputDir() override {
        return TestHelper::appendRag3dbRootPath("dataset/tinysnb/");
    }
    void reserveAll() {
        auto* bm = getBufferManager(*database);
        // Can't use UINT64_MAX since it will overflow the usedMemory
        ASSERT_FALSE(bm->reserve(UINT64_MAX / 2));
    }
};

TEST_F(BufferManagerTest, TestBMUsageForIdenticalQueries) {
    auto bm = getBufferManager(*database);
    auto initialMemoryUsage = bm->getUsedMemory();
    auto numRuns = 10u;
    auto query = "MATCH (p:person) WHERE p.ID = 0 RETURN p.fName";
    for (auto i = 0u; i < numRuns; ++i) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << result->toString();
    }
    auto memoryUsed = bm->getUsedMemory();
    for (auto i = 0u; i < numRuns; ++i) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << result->toString();
    }
    ASSERT_EQ(memoryUsed, bm->getUsedMemory())
        << "Memory usage after two identical queries should be identical";
    spdlog::info("Memory used initially: {}", initialMemoryUsage);
    spdlog::info("Memory used after transactions: {}", memoryUsed);
}

class EmptyBufferManagerTest : public DBTest {
public:
    std::string getInputDir() override {
        return TestHelper::appendRag3dbRootPath("dataset/empty-db/");
    }
};

TEST_F(EmptyBufferManagerTest, TestSpillToDiskMemoryUsage) {
    if (inMemMode) {
        GTEST_SKIP();
    }
    auto bm = getBufferManager(*database);
    auto mm = getMemoryManager(*database);
    auto initialUsedMemory = bm->getUsedMemory();
    {
        auto buffer = mm->allocateBuffer(false, 1024);
        ASSERT_EQ(initialUsedMemory + 1024, bm->getUsedMemory());
        ASSERT_NE(buffer->getBuffer().data(), nullptr);
    }
    ASSERT_EQ(initialUsedMemory, bm->getUsedMemory());

    {
        std::vector<std::unique_ptr<ColumnChunkData>> chunks;
        chunks.push_back(std::make_unique<ColumnChunkData>(*mm, LogicalType(LogicalTypeID::INT64),
            1024, false, ResidencyState::IN_MEMORY, false));
        auto chunkedNodeGroup = InMemChunkedNodeGroup(std::move(chunks), 0);
        chunkedNodeGroup.setUnused(*mm);
        auto memoryWithChunks = bm->getUsedMemory();
        SpillResult memorySpilled{};
        bm->getSpillerOrSkip([&](auto& spiller) {
            spiller.addUnusedChunk(&chunkedNodeGroup);
            // Claim memory from unused chunks
            memorySpilled = spiller.claimNextGroup();
        });
        ASSERT_NE(memorySpilled.memoryFreed + memorySpilled.memoryNowEvictable, 0);
        // The chunks should be entirely on disk
        ASSERT_EQ(initialUsedMemory, bm->getUsedMemory() - memorySpilled.memoryFreed);
        chunkedNodeGroup.loadFromDisk(*mm);
        // The chunks should be back in memory, but Spiller::claimNextGroup does not update
        // the amount of used memory itself, so we end up with the spilled memory recorded twice
        ASSERT_EQ(memoryWithChunks, bm->getUsedMemory() - memorySpilled.memoryFreed);
    }

    {
        // The memory manager uses the buffer manager for allocations of size TEMP_PAGE_SIZE
        std::vector<std::unique_ptr<ColumnChunkData>> chunks;
        chunks.push_back(std::make_unique<ColumnChunkData>(*mm, LogicalType(LogicalTypeID::INT64),
            TEMP_PAGE_SIZE / sizeof(int64_t), false, ResidencyState::IN_MEMORY, false));
        auto chunkedNodeGroup = InMemChunkedNodeGroup(std::move(chunks), 0);
        chunkedNodeGroup.setUnused(*mm);
        SpillResult memorySpilled{};
        bm->getSpillerOrSkip([&](auto& spiller) {
            spiller.addUnusedChunk(&chunkedNodeGroup);
            // Claim memory from unused chunks
            memorySpilled = spiller.claimNextGroup();
        });
        ASSERT_EQ(memorySpilled.memoryFreed, 0);
        ASSERT_EQ(memorySpilled.memoryNowEvictable, TEMP_PAGE_SIZE);
    }
}

// Simulates the case where we try to evict a page during an optimistic read
TEST_F(BufferManagerTest, TestBMEvictionSlowRead) {
    if (inMemMode) {
        GTEST_SKIP();
    }
    auto* fh = StorageManager::Get(*getClientContext(*conn))->getDataFH();

    auto* page = fh->pinPage(0, PageReadPolicy::READ_PAGE);
    *page = 112;
    fh->getPageState(0)->setDirty();
    fh->unpinPage(0);
    bool firstTry = true;
    // The mmap version will fail the first time, but will be run again since the page state was
    // changed during the read
#if BM_MALLOC
    fh->optimisticReadPage(0, [&](auto* frame) {
        ASSERT_TRUE(firstTry);
        firstTry = false;
        ASSERT_EQ(*frame, 112);
        // Should evict all evictable candidates in the eviction queue before failing
        // Should *not* evict the page we're currently reading
        reserveAll();
        // Probably will still succeed, but with ASAN (and BM_MALLOC) it should catch a read after
        // free if this frame was evicted
        ASSERT_EQ(*frame, 112);
    });
#else
    fh->optimisticReadPage(0, [&](auto* frame) {
        if (firstTry) {
            firstTry = false;
            ASSERT_EQ(*frame, 112);
            // Should evict all evictable candidates in the eviction queue before failing
            // Should evict the page we're currently reading
            reserveAll();
            // Frame was evicted and should now be zeroed
            // (not on macos, which does this lazily)
#ifndef __APPLE__
            ASSERT_EQ(*frame, 0);
#endif
        } else {
            // Second try it should be correct again
            ASSERT_EQ(*frame, 112);
        }
    });
#endif
}

// La réservation par tranches (ticket 2026-10-19-une-base-reserve-8-tio-d-adressage-un-processus-
// n-en-ouvre-qu-une-quinzaine). Une base sur disque réservait d'un bloc sa taille maximale, 8 Tio
// par défaut : un processus, qui adresse 128 Tio, n'en ouvrait qu'une quinzaine.
class VMRegionTest : public EmptyDBTest {};

TEST_F(VMRegionTest, ThirtyTwoDatabasesOpenInOneProcess) {
    if (inMemMode) {
        GTEST_SKIP() << "on-disk databases needed";
    }
    // La taille maximale par défaut (8 Tio), un petit tampon.
    main::SystemConfig config(64ull * 1024 * 1024 /*bufferPoolSize*/);
    std::vector<std::unique_ptr<main::Database>> databases;
    std::string refused;
    for (auto i = 0; i < 32 && refused.empty(); ++i) {
        try {
            databases.push_back(std::make_unique<main::Database>(
                databasePath + "-" + std::to_string(i), config));
            main::Connection connection(databases.back().get());
            auto created = connection.query("CREATE NODE TABLE T(id INT64 PRIMARY KEY);");
            if (!created->isSuccess()) {
                refused = "base " + std::to_string(i) + " : " + created->getErrorMessage();
            }
        } catch (const std::exception& e) {
            refused = "base " + std::to_string(i) + " : " + e.what();
        }
    }
    EXPECT_TRUE(refused.empty()) << "[check: thirty-two-databases-open] " << refused;
}

// Une région à petites tranches (deux groupes de cadres chacune) : les cadres des groupes ajoutés
// au-delà de plusieurs tranches s'écrivent et se relisent, contains est juste aux bords, et la
// région refuse au-delà de sa taille maximale comme avant.
TEST(VMRegionChunks, FramesBeyondSeveralChunksAreWritable) {
    const uint64_t groupBytes = RAG3DB_PAGE_SIZE * StorageConstants::PAGE_GROUP_SIZE;
    VMRegion region(REGULAR_PAGE, 7 * groupBytes /*maxRegionSize*/, 2 * groupBytes /*chunkSize*/);
    ASSERT_EQ(region.getChunkSize(), 2 * groupBytes);
    std::vector<uint8_t*> frames;
    for (auto group = 0u; group < 7; ++group) {
        ASSERT_EQ(region.addNewFrameGroup(), group);
        for (const uint64_t page : {uint64_t{0}, StorageConstants::PAGE_GROUP_SIZE - 1}) {
            const auto frameIdx = group * StorageConstants::PAGE_GROUP_SIZE + page;
            auto* frame = region.getFrame(frameIdx);
            frame[0] = static_cast<uint8_t>(group + 1);
            frame[RAG3DB_PAGE_SIZE - 1] = static_cast<uint8_t>(page % 251);
            EXPECT_TRUE(region.contains(frame)) << "[check: contains] group " << group;
            EXPECT_TRUE(region.contains(frame + RAG3DB_PAGE_SIZE - 1))
                << "[check: contains] group " << group;
            frames.push_back(frame);
        }
    }
    for (auto i = 0u; i < frames.size(); ++i) {
        EXPECT_EQ(frames[i][0], static_cast<uint8_t>(i / 2 + 1)) << "[check: frames-keep-their-bytes]";
    }
    uint8_t outside = 0;
    EXPECT_FALSE(region.contains(&outside)) << "[check: contains]";
    EXPECT_THROW(region.addNewFrameGroup(), BufferManagerException) << "[check: max-frame-groups]";
}

// Une tranche refusée par le système (ici la troisième, par une région qui fait échouer sa
// réservation) : addNewFrameGroup lève une erreur nommée, et les cadres déjà réservés restent
// bons.
class RegionRefusingItsThirdChunk final : public VMRegion {
public:
    using VMRegion::VMRegion;

protected:
    uint8_t* reserveChunk(uint64_t size) override {
        return ++numReserved == 3 ? nullptr : VMRegion::reserveChunk(size);
    }

private:
    int numReserved = 0;
};

TEST(VMRegionChunks, ARefusedChunkIsANamedError) {
    const uint64_t groupBytes = RAG3DB_PAGE_SIZE * StorageConstants::PAGE_GROUP_SIZE;
    RegionRefusingItsThirdChunk region(REGULAR_PAGE, 8 * groupBytes, 2 * groupBytes);
    for (auto group = 0u; group < 4; ++group) {
        ASSERT_EQ(region.addNewFrameGroup(), group);
        region.getFrame(group * StorageConstants::PAGE_GROUP_SIZE)[0] = 7;
    }
    try {
        region.addNewFrameGroup();
        ADD_FAILURE() << "[check: refused-chunk-is-named] no error";
    } catch (const BufferManagerException& e) {
        EXPECT_NE(std::string(e.what()).find("Cannot reserve"), std::string::npos)
            << "[check: refused-chunk-is-named] " << e.what();
    }
    EXPECT_EQ(region.getFrame(3 * StorageConstants::PAGE_GROUP_SIZE)[0], 7)
        << "[check: frames-keep-their-bytes]";
}

} // namespace testing
} // namespace rag3db
