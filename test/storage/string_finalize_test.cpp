#include "common/exception/buffer_manager.h"
#include "common/vector/value_vector.h"
#include "gtest/gtest.h"
#include "storage/buffer_manager/buffer_manager.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/table/string_chunk_data.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace {
class FinalizeFailingBufferManager : public BufferManager {
public:
    FinalizeFailingBufferManager()
        : BufferManager(":memory:", "", 64 * 1024 * 1024, 128 * 1024 * 1024, nullptr, true) {}
    bool failLargeAllocation = false;
    bool reserve(uint64_t bytes) override {
        if (failLargeAllocation && bytes >= 1024 * 1024) {
            return false;
        }
        return BufferManager::reserve(bytes);
    }
};

void checkFinalizeFailure(LogicalType type) {
    FinalizeFailingBufferManager bm;
    MemoryManager mm(&bm, nullptr);
    StringChunkData chunk(mm, type.copy(), 4, true, ResidencyState::IN_MEMORY);
    ValueVector input(type.copy(), &mm);
    auto write = [&](uint64_t row, const std::string& text) {
        input.setNull(0, false);
        StringVector::addString(&input, 0, text);
        chunk.write(&input, 0, row);
    };
    const std::string large(1024 * 1024, 'z');
    write(0, "obsolete");
    write(1, large);
    write(2, "tail");
    input.setNull(0, true);
    chunk.write(&input, 0, 3);
    write(0, "replacement"); // Makes dictionary indices differ before/after pruning.

    bm.failLargeAllocation = true;
    EXPECT_THROW(chunk.finalize(), BufferManagerException);
    // Even after a partial rebuild, the original row->dictionary mapping must survive.
    EXPECT_EQ(chunk.getValue<std::string_view>(0), "replacement");
    EXPECT_EQ(chunk.getValue<std::string_view>(1), large);
    EXPECT_EQ(chunk.getValue<std::string_view>(2), "tail");
    EXPECT_TRUE(chunk.isNull(3));

    bm.failLargeAllocation = false;
    EXPECT_NO_THROW(chunk.finalize());
    EXPECT_EQ(chunk.getValue<std::string_view>(0), "replacement");
    EXPECT_EQ(chunk.getValue<std::string_view>(1), large);
    EXPECT_EQ(chunk.getValue<std::string_view>(2), "tail");
    EXPECT_TRUE(chunk.isNull(3));
    EXPECT_NO_THROW(chunk.finalize());
    EXPECT_EQ(chunk.getValue<std::string_view>(0), "replacement");
}
} // namespace

TEST(StringFinalizeTests, StringSurvivesAllocationFailureAndRetry) {
    checkFinalizeFailure(LogicalType::STRING());
}
TEST(StringFinalizeTests, BlobSurvivesAllocationFailureAndRetry) {
    checkFinalizeFailure(LogicalType::BLOB());
}
