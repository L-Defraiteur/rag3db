#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::testing;

// Une table de BLOB plus grande que le quart du tampon, un point de reprise après chaque
// validation : chacun doit passer. Avant les segments de chaînes bornés en octets, le point de
// reprise relisait et réécrivait hors place un segment qui grandissait avec toute la colonne,
// agrandi par doublements : il échouait dès que la colonne franchissait le quart du tampon, en
// puissance de 2 (256 Mio sous 1 Gio, 512 sous 2, 1 024 sous 4), et la base était à rouvrir
// (ticket 2026-10-04 « le point de reprise échoue quand le tampon est petit »).
class BlobCheckpointTest : public EmptyDBTest {
protected:
    static std::string blob(uint64_t seed, size_t size) {
        std::string value(size, '\0');
        uint64_t x = seed * 6364136223846793005ull + 1442695040888963407ull;
        for (auto& c : value) {
            x = x * 6364136223846793005ull + 1442695040888963407ull;
            c = static_cast<char>(x >> 33);
        }
        return value;
    }
};

TEST_F(BlobCheckpointTest, ATableLargerThanAQuarterOfTheBufferPoolCheckpoints) {
    if (inMemMode) {
        GTEST_SKIP() << "on-disk database needed";
    }
    constexpr uint64_t bufferPoolSize = 256ull << 20;
    constexpr uint64_t blobSize = 1ull << 20;
    constexpr uint64_t blobsPerCommit = 8;
    constexpr uint64_t totalBytes = 112ull << 20; // au-delà de 64 Mio, le quart du tampon
    systemConfig->bufferPoolSize = bufferPoolSize;
    createDBAndConn();
    ASSERT_TRUE(conn->query("CREATE NODE TABLE Blobs(_key STRING, _data BLOB, PRIMARY KEY(_key));")
                    ->isSuccess());
    ASSERT_TRUE(conn->query("CALL auto_checkpoint=false;")->isSuccess());
    auto insert = conn->prepare("CREATE (:Blobs {_key: $k, _data: $d});");
    ASSERT_TRUE(insert->isSuccess()) << insert->getErrorMessage();
    uint64_t key = 0;
    for (uint64_t written = 0; written < totalBytes;) {
        ASSERT_TRUE(conn->query("BEGIN TRANSACTION;")->isSuccess());
        for (uint64_t i = 0; i < blobsPerCommit; i++, key++, written += blobSize) {
            auto result = conn->execute(insert.get(),
                std::make_pair(std::string("k"), Value("blob-" + std::to_string(key))),
                std::make_pair(std::string("d"), Value(LogicalType::BLOB(), blob(key, blobSize))));
            ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        }
        ASSERT_TRUE(conn->query("COMMIT;")->isSuccess());
        auto checkpoint = conn->query("CHECKPOINT;");
        ASSERT_TRUE(checkpoint->isSuccess())
            << "[check: checkpoint-under-a-small-pool] " << (written >> 20) << " MiB of BLOB in a "
            << (bufferPoolSize >> 20) << " MiB buffer pool: " << checkpoint->getErrorMessage();
    }
    // Les blobs se relisent, juste, après réouverture.
    createDBAndConn();
    auto count = conn->query("MATCH (b:Blobs) RETURN count(*), sum(octet_length(b._data));");
    ASSERT_TRUE(count->isSuccess()) << count->getErrorMessage();
    auto row = count->getNext();
    EXPECT_EQ(row->getValue(0)->getValue<int64_t>(), static_cast<int64_t>(key));
    EXPECT_EQ(row->getValue(1)->getValue<int64_t>(), static_cast<int64_t>(key * blobSize));
    for (const uint64_t probe : {uint64_t{0}, key / 2, key - 1}) {
        auto value = conn->query(
            "MATCH (b:Blobs {_key: 'blob-" + std::to_string(probe) + "'}) RETURN b._data;");
        ASSERT_TRUE(value->isSuccess()) << value->getErrorMessage();
        EXPECT_TRUE(value->getNext()->getValue(0)->getValue<std::string>() == blob(probe, blobSize))
            << "[check: blob-content] blob " << probe;
    }
}
