#include "gtest/gtest.h"
#include "storage/buffer_manager/buffer_manager.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/local_storage/local_hash_index.h"
#include "storage/overflow_file.h"

using namespace rag3db::common;
using namespace rag3db::storage;

bool isVisible(offset_t) {
    return true;
}

TEST(LocalHashIndexTests, LocalInserts) {
    BufferManager bm(":memory:", "", 256 * 1024 * 1024 /*bufferPoolSize*/,
        512 * 1024 * 1024 /*maxDBSize*/, nullptr, true);
    MemoryManager memoryManager(&bm, nullptr);
    auto overflowFile = std::make_unique<InMemOverflowFile>(memoryManager);
    auto* overflowFileHandle = overflowFile->addHandle();
    auto hashIndex =
        std::make_unique<LocalHashIndex>(memoryManager, PhysicalTypeID::INT64, overflowFileHandle);

    for (int64_t i = 0u; i < 100000; i++) {
        ASSERT_TRUE(hashIndex->insert(i, i * 2, isVisible));
    }
    for (int64_t i = 0u; i < 100000; i++) {
        ASSERT_FALSE(hashIndex->insert(i, i, isVisible));
    }

    for (int64_t i = 0u; i < 100000; i++) {
        hashIndex->delete_(i);
    }
    for (int64_t i = 0u; i < 100000; i++) {
        ASSERT_TRUE(hashIndex->insert(i, i, isVisible));
    }
}

std::string gen_random(const int len) {
    static const char alphanum[] = "0123456789"
                                   "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
                                   "abcdefghijklmnopqrstuvwxyz";
    std::string tmp_s;
    tmp_s.reserve(len);

    for (int i = 0; i < len; ++i) {
        tmp_s += alphanum[rand() % (sizeof(alphanum) - 1)];
    }

    return tmp_s;
}

TEST(LocalHashIndexTests, LocalStringInserts) {
    BufferManager bm(":memory:", "", 256 * 1024 * 1024 /*bufferPoolSize*/,
        512 * 1024 * 1024 /*maxDBSize*/, nullptr, true);
    MemoryManager memoryManager(&bm, nullptr);
    auto overflowFile = std::make_unique<InMemOverflowFile>(memoryManager);
    auto* overflowFileHandle = overflowFile->addHandle();
    auto hashIndex =
        std::make_unique<LocalHashIndex>(memoryManager, PhysicalTypeID::STRING, overflowFileHandle);

    std::vector<std::string> keys;
    for (int64_t i = 0u; i < 100; i++) {
        keys.push_back(gen_random(14));
        ASSERT_TRUE(hashIndex->insert(keys.back(), i * 2, isVisible));
    }
    for (int64_t i = 0u; i < 100; i++) {
        ASSERT_FALSE(hashIndex->insert(keys[i], i * 2, isVisible));
    }
}

TEST(LocalHashIndexTests, DiscardOffsetsKeepsCollidingLongKeysAndReusesOverflowSlots) {
    BufferManager bm(":memory:", "", 64 * 1024 * 1024, 512 * 1024 * 1024, nullptr, true);
    MemoryManager mm(&bm, nullptr);
    InMemHashIndex<ku_string_t> index(mm, nullptr);
    index.reserve(40);
    std::vector<std::string> keys;
    // Une seule chaîne de collision : la suppression doit laisser un préfixe compact
    // même quand les entrées conservées traversent plusieurs slots de débordement.
    for (uint64_t i = 0; keys.size() < 40; ++i) {
        auto key = std::string(128, 'k') + std::to_string(i);
        if (HashIndexUtils::getPrimarySlotIdForHash(index.getIndexHeader(),
                HashIndexUtils::hash(std::string_view{key})) == 0) {
            keys.push_back(std::move(key));
        }
    }
    for (offset_t i = 0; i < keys.size(); ++i) {
        ASSERT_TRUE(index.append(std::string{keys[i]}, i * 2, isVisible));
    }
    index.discardRange(0, 0);
    EXPECT_EQ(index.size(), 40);
    index.discardRange(10, 20);
    index.discardRange(10, 20); // Une seconde annulation ne retire pas de voisin.
    EXPECT_EQ(index.size(), 30);
    for (offset_t i = 0; i < keys.size(); ++i) {
        offset_t found = INVALID_OFFSET;
        const bool removed = i >= 5 && i < 15;
        EXPECT_EQ(index.lookup(keys[i], found, isVisible), !removed) << i;
        if (!removed) {
            EXPECT_EQ(found, i * 2);
        } else {
            ASSERT_TRUE(index.append(std::string{keys[i]}, i * 2, isVisible));
        }
    }
    EXPECT_EQ(index.size(), 40);
    index.discardRange(0, 80);
    EXPECT_TRUE(index.empty());
    for (offset_t i = 0; i < keys.size(); ++i) {
        ASSERT_TRUE(index.append(std::string{keys[i]}, i, isVisible));
        offset_t found = INVALID_OFFSET;
        ASSERT_TRUE(index.lookup(keys[i], found, isVisible));
        EXPECT_EQ(found, i);
    }
}

TEST(LocalHashIndexTests, DiscardOffsetRangeDoesNotWrapAtTheLastOffset) {
    BufferManager bm(":memory:", "", 64 * 1024 * 1024, 512 * 1024 * 1024, nullptr, true);
    MemoryManager mm(&bm, nullptr);
    InMemHashIndex<int64_t> index(mm, nullptr);
    const auto last = std::numeric_limits<offset_t>::max();
    ASSERT_TRUE(index.append(int64_t{1}, 0, isVisible));
    ASSERT_TRUE(index.append(int64_t{2}, last - 2, isVisible));
    ASSERT_TRUE(index.append(int64_t{3}, last - 1, isVisible));
    index.discardRange(last - 2, 7);
    offset_t found = INVALID_OFFSET;
    EXPECT_TRUE(index.lookup(1, found, isVisible));
    EXPECT_EQ(found, 0);
    EXPECT_FALSE(index.lookup(2, found, isVisible));
    EXPECT_FALSE(index.lookup(3, found, isVisible));
    EXPECT_EQ(index.size(), 1);
}
