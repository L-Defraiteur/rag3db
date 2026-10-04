// Un point de reprise après un COPY refusé dans un groupe de nœuds encore en mémoire.
//
// L'annulation d'un ajout ramenait le compte de lignes du bloc, pas ses colonnes : les lignes
// annulées y restaient. Le point de reprise suivant dimensionnait son bloc de sortie pour les
// lignes valides et y recopiait aussi les annulées — une écriture au-delà du bloc
// (AddressSanitizer : heap-buffer-overflow dans NullMask::copyNullMask, sous
// NodeGroup::checkpointInMemOnly). Défaut d'origine, sans rapport avec l'index vectoriel ;
// trouvé en reprenant l'essai déterministe de la session du banc (4 octobre 2026).
//
// En Release ce cas est un témoin probabiliste du défaut (une corruption de tas ne tue pas à
// coup sûr) ; sous AddressSanitizer il est déterministe.

#include <filesystem>
#include <fstream>
#include <string>

#include <unistd.h>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::testing;

namespace {

class CheckpointAfterFailedCopyTest : public EmptyDBTest {
protected:
    static constexpr int64_t NUM_ROWS = 200;

    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "needs an on-disk database: the defect is in what a checkpoint writes";
        }
        createDBAndConn();
        ok("CALL auto_checkpoint=false;");
        ok("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
        ok("UNWIND range(0, " + std::to_string(NUM_ROWS - 1) +
            ") AS i CREATE (:Doc {id: i, name: 'row ' + CAST(i AS STRING)});");
        csvPath = (std::filesystem::temp_directory_path() /
                   ("checkpoint_after_failed_copy_" + std::to_string(::getpid()) + ".csv"))
                      .string();
    }

    void TearDown() override {
        std::filesystem::remove(csvPath);
        EmptyDBTest::TearDown();
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    int64_t single(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    // numRows lignes neuves à partir de firstId ; si duplicate, une de plus dont la clé existe.
    void writeCsv(int64_t firstId, int64_t numRows, bool duplicate) const {
        std::ofstream csv(csvPath);
        for (int64_t i = 0; i < numRows; i++) {
            csv << firstId + i << ",copied " << firstId + i << "\n";
        }
        if (duplicate) {
            csv << 5 << ",duplicate\n";
        }
    }

    void expectTheRowsOfTheTable(int64_t numRows) {
        EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), numRows);
        EXPECT_EQ(single("MATCH (n:Doc) WHERE n.name STARTS WITH 'row ' RETURN count(*);"),
            NUM_ROWS);
        // La clé en double, puis ses voisines : chaque ligne d'avant le COPY se retrouve par sa
        // clé (l'annulation relit des blocs entiers, pas la seule plage annulée).
        EXPECT_EQ(single("MATCH (n:Doc {id: 5}) RETURN count(*);"), 1);
        for (const auto id : {0, 7, 100, 199}) {
            EXPECT_EQ(single("MATCH (n:Doc {id: " + std::to_string(id) + "}) RETURN count(*);"),
                1)
                << id;
        }
    }

    std::string csvPath;
};

TEST_F(CheckpointAfterFailedCopyTest, TheCheckpointWritesOnlyTheRowsThatAreLeft) {
    writeCsv(NUM_ROWS, NUM_ROWS, true /* duplicate */);
    auto refused = conn->query("COPY Doc FROM '" + csvPath + "';");
    ASSERT_FALSE(refused->isSuccess()) << "the COPY carries a duplicated key";
    expectTheRowsOfTheTable(NUM_ROWS);
    ok("CHECKPOINT;");
    expectTheRowsOfTheTable(NUM_ROWS);
    createDBAndConn(); // et après une réouverture
    expectTheRowsOfTheTable(NUM_ROWS);
    // La table reste utilisable : le même COPY, sans la clé en double, aboutit.
    writeCsv(NUM_ROWS, NUM_ROWS, false /* duplicate */);
    ok("COPY Doc FROM '" + csvPath + "';");
    ok("CHECKPOINT;");
    expectTheRowsOfTheTable(2 * NUM_ROWS);
}

} // namespace
