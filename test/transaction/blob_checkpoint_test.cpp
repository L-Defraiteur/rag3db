#include <atomic>
#include <filesystem>
#include <fstream>
#include <string>
#include <thread>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "test_helper/test_helper.h"

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

// Le même seuil, mais par COPY en point de reprise forcé : le chemin du magasin de blobs du plein
// texte de rag3weaver (un COPY de _index_blobs par paquet). Le correctif des segments bornés couvre
// les écritures ligne à ligne, pas ce chemin : la première indexation en blobs sous 2 Gio restait
// rouge (ticket 2026-10-11-copy-de-blobs-en-point-de-reprise-force-sous-un-petit-tampon). Des
// blobs de caractères imprimables, sans virgule, guillemet, barre oblique inverse ni fin de ligne,
// se lisent tels quels depuis un CSV.
class BlobCopyCheckpointTest : public BlobCheckpointTest {
protected:
    static std::string printableBlob(uint64_t seed, size_t size) {
        std::string value(size, 'a');
        uint64_t x = seed * 6364136223846793005ull + 1442695040888963407ull;
        for (auto& c : value) {
            x = x * 6364136223846793005ull + 1442695040888963407ull;
            c = static_cast<char>('a' + (x >> 33) % 26);
        }
        return value;
    }

    // Écrit `numRows` blobs à partir de la clé `firstKey`, puis les charge par un COPY forcé.
    void copyBlobs(uint64_t firstKey, uint64_t numRows, uint64_t blobSize) {
        const auto csv = databasePath + ".blobs.csv";
        {
            std::ofstream out(csv, std::ios::trunc);
            for (auto key = firstKey; key < firstKey + numRows; key++) {
                out << "blob-" << key << "," << printableBlob(key, blobSize) << "\n";
            }
        }
        auto result = conn->query("COPY Blobs FROM '" + csv + "' (HEADER=false);");
        ASSERT_TRUE(result->isSuccess())
            << "[check: forced-copy-under-a-small-pool] rows " << firstKey << " to "
            << firstKey + numRows << " (" << ((firstKey + numRows) * blobSize >> 20)
            << " MiB in the table): " << result->getErrorMessage();
    }

    // forced : le COPY tient sa durabilité de son point de reprise ; sinon il est journalisé, le
    // défaut depuis ff9bad960.
    void setUpTable(bool forced = true) {
        systemConfig->bufferPoolSize = 256ull << 20;
        createDBAndConn();
        ASSERT_TRUE(
            conn->query("CREATE NODE TABLE Blobs(_key STRING, _data BLOB, PRIMARY KEY(_key));")
                ->isSuccess());
        ASSERT_TRUE(conn->query(std::string("CALL force_checkpoint_on_copy=") +
                                (forced ? "true" : "false") + ";")
                        ->isSuccess());
    }

    void expectBlobs(uint64_t numRows, uint64_t blobSize) {
        createDBAndConn();
        auto count = conn->query("MATCH (b:Blobs) RETURN count(*), sum(octet_length(b._data));");
        ASSERT_TRUE(count->isSuccess()) << count->getErrorMessage();
        auto row = count->getNext();
        EXPECT_EQ(row->getValue(0)->getValue<int64_t>(), static_cast<int64_t>(numRows));
        EXPECT_EQ(row->getValue(1)->getValue<int64_t>(), static_cast<int64_t>(numRows * blobSize));
        for (const uint64_t probe : {uint64_t{0}, numRows / 2, numRows - 1}) {
            auto value = conn->query(
                "MATCH (b:Blobs {_key: 'blob-" + std::to_string(probe) + "'}) RETURN b._data;");
            ASSERT_TRUE(value->isSuccess()) << value->getErrorMessage();
            EXPECT_TRUE(value->getNext()->getValue(0)->getValue<std::string>() ==
                        printableBlob(probe, blobSize))
                << "[check: blob-content] blob " << probe;
        }
    }
};

// Des COPY successifs de 16 blobs d'1 Mio, chacun avec son point de reprise forcé, jusqu'à 112 Mio
// sous 256 Mio de tampon : la forme du produit, un COPY par paquet.
TEST_F(BlobCopyCheckpointTest, SuccessiveForcedCopiesBeyondAQuarterOfTheBufferPool) {
    if (inMemMode) {
        GTEST_SKIP() << "on-disk database needed";
    }
    setUpTable();
    constexpr uint64_t blobSize = 1ull << 20;
    constexpr uint64_t rowsPerCopy = 16;
    constexpr uint64_t numRows = 112;
    for (uint64_t key = 0; key < numRows; key += rowsPerCopy) {
        copyBlobs(key, rowsPerCopy, blobSize);
        if (HasFatalFailure()) {
            return;
        }
    }
    expectBlobs(numRows, blobSize);
}

// Un seul COPY de 112 blobs d'1 Mio sous 256 Mio de tampon.
TEST_F(BlobCopyCheckpointTest, OneForcedCopyBeyondAQuarterOfTheBufferPool) {
    if (inMemMode) {
        GTEST_SKIP() << "on-disk database needed";
    }
    setUpTable();
    constexpr uint64_t blobSize = 1ull << 20;
    constexpr uint64_t numRows = 112;
    copyBlobs(0, numRows, blobSize);
    if (HasFatalFailure()) {
        return;
    }
    expectBlobs(numRows, blobSize);
}

// Le même COPY de 112 blobs, journalisé (le défaut) : le ticket demande si le même chemin
// d'assemblage le fait échouer aussi.
TEST_F(BlobCopyCheckpointTest, OneJournaledCopyBeyondAQuarterOfTheBufferPool) {
    if (inMemMode) {
        GTEST_SKIP() << "on-disk database needed";
    }
    setUpTable(false);
    constexpr uint64_t blobSize = 1ull << 20;
    constexpr uint64_t numRows = 112;
    copyBlobs(0, numRows, blobSize);
    if (HasFatalFailure()) {
        return;
    }
    expectBlobs(numRows, blobSize);
}

// Le même COPY pendant qu'un lecteur balaie la table en boucle : les lignes vidées par lots
// s'ajoutent à la tête par copie-échange, jamais en place sous les pieds du lecteur.
TEST_F(BlobCopyCheckpointTest, OneCopyBeyondAQuarterOfTheBufferPoolUnderAReaderInLoop) {
    if (inMemMode) {
        GTEST_SKIP() << "on-disk database needed";
    }
    setUpTable(false);
    std::atomic<bool> stop{false};
    std::atomic<int> scans{0}, failedScans{0};
    std::thread reader([&] {
        rag3db::main::Connection connection(database.get());
        while (!stop.load()) {
            auto result = connection.query("MATCH (b:Blobs) RETURN count(*), sum(octet_length(b._data));");
            (result->isSuccess() ? scans : failedScans)++;
        }
    });
    constexpr uint64_t blobSize = 1ull << 20;
    constexpr uint64_t numRows = 112;
    copyBlobs(0, numRows, blobSize);
    stop = true;
    reader.join();
    std::cout << "balayages pendant le COPY : " << scans << " réussis, " << failedScans
              << " refusés\n";
    if (HasFatalFailure()) {
        return;
    }
    EXPECT_GT(scans.load(), 0);
    expectBlobs(numRows, blobSize);
}

// Un gros COPY annulé : aucune de ses lignes ne reste, le point de reprise suivant passe, et le
// même COPY repasse ensuite.
TEST_F(BlobCopyCheckpointTest, ARolledBackCopyBeyondAQuarterOfTheBufferPoolLeavesNoRow) {
    if (inMemMode) {
        GTEST_SKIP() << "on-disk database needed";
    }
    setUpTable(false);
    constexpr uint64_t blobSize = 1ull << 20;
    constexpr uint64_t numRows = 112;
    ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
    const auto sizeBefore = std::filesystem::file_size(databasePath);
    ASSERT_TRUE(conn->query("BEGIN TRANSACTION;")->isSuccess());
    copyBlobs(0, numRows, blobSize);
    if (HasFatalFailure()) {
        return;
    }
    ASSERT_TRUE(conn->query("ROLLBACK;")->isSuccess());
    {
        // Les résultats meurent ici, avant que expectBlobs ne rouvre la base qu'ils tiennent.
        auto count = conn->query("MATCH (b:Blobs) RETURN count(*);");
        ASSERT_TRUE(count->isSuccess()) << count->getErrorMessage();
        EXPECT_EQ(count->getNext()->getValue(0)->getValue<int64_t>(), 0)
            << "des lignes du COPY annulé restent";
        auto checkpoint = conn->query("CHECKPOINT;");
        ASSERT_TRUE(checkpoint->isSuccess()) << checkpoint->getErrorMessage();
    }
    copyBlobs(0, numRows, blobSize);
    if (HasFatalFailure()) {
        return;
    }
    ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
    // Les pages des segments vidés par le COPY annulé sont reprises : le second COPY les réemploie.
    // Si elles fuyaient, le fichier porterait deux fois les 112 Mio.
    const auto sizeAfter = std::filesystem::file_size(databasePath);
    std::cout << "fichier de données : " << (sizeBefore >> 20) << " Mio avant, " << (sizeAfter >> 20)
              << " Mio après le COPY annulé puis le COPY validé\n";
    EXPECT_LT(sizeAfter - sizeBefore, numRows * blobSize * 3 / 2)
        << "les pages du COPY annulé n'ont pas été reprises";
    expectBlobs(numRows, blobSize);
}

// Un COPY de vecteurs dans une table à index HNSW (ticket
// docs/tickets/2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-concurrente-pendant-une-indexation.md),
// témoin écrit par hotfixs. À la finalisation de l'index par le COPY, OnDiskHNSWIndex::shrinkForNode
// prenait le vecteur du nœud par pointeur dans le vecteur de données de l'état de lecture, puis lisait
// les voisins sur le même état (getEmbeddings → ListAuxiliaryBuffer::resize) : le vecteur de données
// réalloué, le pointeur désignait un tampon libéré (heap-use-after-free sous ASAN, une lecture de
// 3 072 octets). ≈ 3 Ko par ligne (FLOAT[768]), comme la réindexation de la montre ; sous un petit
// tampon et à plusieurs fils, le COPY se vide aussi par son budget en octets (la copie-échange de
// G2b, hors de cause). Le défaut ne se voit que sous ASAN ; en Release le témoin juge que l'index
// atteint chaque vecteur et survit à un point de reprise. Environ 7 à 8 minutes en Release.
class VectorCopyUnderASmallPoolTest : public EmptyDBTest {
protected:
    static constexpr uint64_t NUM_ROWS = 30000;
    static constexpr uint64_t DIMENSION = 768;

    static std::string vectorOf(uint64_t id) {
        std::string out = "[";
        for (auto j = 0u; j < DIMENSION; j++) {
            // La première composante est l'identifiant : aucun vecteur n'en double un autre.
            out += (j ? "," : "") + std::to_string(j == 0 ? id : (id * 31 + j * 17) % 97);
        }
        return out + "]";
    }

    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        // 512 Mio : sous 256, le COPY lui-même bute sur « buffer pool is full » (l'index HNSW de
        // 30 000 vecteurs de 768 floats n'y tient pas), et le témoin ne juge plus le mélange.
        // Budget du COPY : 512 Mio / (3 × 5) ≈ 34 Mio, soit trois vidages au moins sur 92 Mio.
        systemConfig->bufferPoolSize = 512ull << 20;
        systemConfig->maxNumThreads = 4;
        createDBAndConn();
        const auto extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        if (!std::filesystem::exists(extension)) {
            GTEST_SKIP() << "the vector extension is not built";
        }
        must("LOAD EXTENSION '" + extension + "';");
        must("CREATE NODE TABLE V(id INT64, v FLOAT[768], PRIMARY KEY (id));");
        must("CALL CREATE_VECTOR_INDEX('V', 'vi', 'v', metric := 'l2');");
    }

    void must(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << "[check: query] " << query << " : "
                                         << result->getErrorMessage();
    }

    int64_t single(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << "[check: query] " << query << " : "
                                         << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }
};

TEST_F(VectorCopyUnderASmallPoolTest, TheIndexReadsItsVectorsWhileTheHeadIsExchanged) {
    const auto csv = databasePath + ".vectors.csv";
    {
        std::ofstream out(csv, std::ios::trunc);
        for (auto id = 0u; id < NUM_ROWS; id++) {
            out << id << "|" << vectorOf(id) << "\n";
        }
    }
    must("COPY V FROM '" + csv + "' (DELIM='|', HEADER=false);");
    std::filesystem::remove(csv);
    EXPECT_EQ(single("MATCH (n:V) RETURN count(*);"), static_cast<int64_t>(NUM_ROWS));
    // Chaque vecteur est atteint par l'index : le plus proche de lui-même, c'est lui.
    for (const uint64_t probe : {uint64_t{0}, NUM_ROWS / 2, NUM_ROWS - 1}) {
        EXPECT_EQ(single("CALL QUERY_VECTOR_INDEX('V', 'vi', " + vectorOf(probe) +
                         ", 1) RETURN node.id;"),
            static_cast<int64_t>(probe))
            << "[check: nearest-is-itself] " << probe;
    }
    must("CHECKPOINT;");
    createDBAndConn();
    EXPECT_EQ(single("MATCH (n:V) RETURN count(*);"), static_cast<int64_t>(NUM_ROWS));
}
