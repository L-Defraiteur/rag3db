#include <string>
#include <vector>

#include "common/constants.h"
#include "common/vector/value_vector.h"
#include "gtest/gtest.h"
#include "storage/buffer_manager/buffer_manager.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/table/string_chunk_data.h"

using namespace rag3db::common;
using namespace rag3db::storage;

// Le découpage d'un bloc de chaînes en segments (ColumnChunkData::split) doit borner chaque segment
// en octets réels : un segment de plus d'une valeur ne dépasse pas la taille visée, et une valeur
// plus grande que cette taille a son segment à elle. Sinon le point de reprise suivant relit et
// réécrit hors place un segment qui grandit avec toute la colonne (ticket 2026-10-04 « le point de
// reprise échoue quand le tampon est petit »).
namespace {

class StringSplitTest : public ::testing::Test {
protected:
    StringSplitTest()
        : bm(":memory:", "", 512ull * 1024 * 1024, 1024ull * 1024 * 1024, nullptr, true),
          mm(&bm, nullptr) {}

    std::unique_ptr<StringChunkData> chunkOf(const std::vector<std::string>& values) {
        auto chunk = std::make_unique<StringChunkData>(mm, LogicalType::BLOB(), values.size(),
            true /* enableCompression */, ResidencyState::IN_MEMORY);
        ValueVector input(LogicalType::BLOB(), &mm);
        for (size_t row = 0; row < values.size(); row++) {
            input.setNull(0, false);
            StringVector::addString(&input, 0, values[row]);
            chunk->write(&input, 0, row);
        }
        return chunk;
    }

    static std::string blob(size_t size, char seed) {
        std::string value(size, '\0');
        for (size_t i = 0; i < size; i++) {
            value[i] = static_cast<char>(seed + i * 31);
        }
        return value;
    }

    // Chaque valeur se retrouve, dans l'ordre ; un segment de plus d'une valeur reste sous la
    // borne ; les octets d'un segment sont ceux de ses valeurs, rien de perdu en route.
    void expectBoundedSegments(const std::vector<std::string>& values, bool targetMaxSize) {
        const auto chunk = chunkOf(values);
        const auto segments = chunk->split(targetMaxSize);
        size_t row = 0;
        for (const auto& segment : segments) {
            auto& strings = segment->cast<StringChunkData>();
            uint64_t bytesOfValues = 0;
            for (uint64_t i = 0; i < strings.getNumValues(); i++, row++) {
                ASSERT_LT(row, values.size());
                EXPECT_EQ(strings.getValue<std::string>(i), values[row]) << "row " << row;
                bytesOfValues += values[row].size();
            }
            const auto stringBytes =
                strings.getDictionaryChunk().getStringDataChunk()->getNumValues();
            EXPECT_LE(stringBytes, bytesOfValues)
                << "a segment carries string bytes that none of its values uses";
            if (strings.getNumValues() > 1) {
                EXPECT_LE(stringBytes, StorageConfig::MAX_SEGMENT_SIZE)
                    << "a segment of " << strings.getNumValues() << " values holds "
                    << stringBytes << " bytes of strings";
            }
        }
        EXPECT_EQ(row, values.size());
    }

    BufferManager bm;
    MemoryManager mm;
};

TEST_F(StringSplitTest, LargeBlobsGetOneSegmentEach) {
    std::vector<std::string> values;
    for (int i = 0; i < 16; i++) {
        values.push_back(blob(1024 * 1024, static_cast<char>(i)));
    }
    expectBoundedSegments(values, false);
    expectBoundedSegments(values, true);
    EXPECT_EQ(chunkOf(values)->split().size(), values.size());
}

TEST_F(StringSplitTest, SmallStringsAroundALargeBlobStayBounded) {
    std::vector<std::string> values;
    for (int i = 0; i < 3000; i++) {
        values.push_back(i == 1500 ? blob(1024 * 1024, 'b') : blob(100 + i % 7, static_cast<char>(i)));
    }
    expectBoundedSegments(values, false);
    expectBoundedSegments(values, true);
}

} // namespace
