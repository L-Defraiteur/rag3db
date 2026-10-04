// Construire un index vectoriel après un COPY refusé.
//
// Un COPY refusé (clé en double) est annulé, mais les statistiques de la table gardent ses
// lignes : sa cardinalité estimée dépasse alors le nombre de lignes. CREATE_VECTOR_INDEX
// bornait son parcours par cette cardinalité, dans un graphe dimensionné par le nombre de
// lignes : il insérait des décalages hors du graphe, et le processus mourait (SIGSEGV).
// Trouvé par l'essai déterministe de la session du banc (4 octobre 2026, 200 lignes, un COPY
// de 201 refusé, cardinalité 401). Le parcours est maintenant borné par le nombre de lignes ;
// un décalage qui dépasserait quand même est refusé par son nom (« is beyond the in-memory
// graph »). Se saute si l'extension vector n'est pas bâtie.

#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

#include <unistd.h>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "test_helper/test_helper.h"

using namespace rag3db::testing;

namespace {

class VectorIndexAfterFailedCopyTest : public EmptyDBTest {
protected:
    static constexpr int64_t NUM_ROWS = 200;

    void SetUp() override {
        EmptyDBTest::SetUp();
        const auto extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        if (!std::filesystem::exists(extension)) {
            GTEST_SKIP() << "the vector extension is not built (-DBUILD_EXTENSIONS=vector)";
        }
        createDBAndConn();
        ok("LOAD EXTENSION '" + extension + "';");
        ok("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
        ok("UNWIND range(0, " + std::to_string(NUM_ROWS - 1) +
            ") AS i CREATE (:Doc {id: i, vec: [CAST(i % 17 AS FLOAT), CAST(i % 23 AS FLOAT), "
            "CAST(i % 29 AS FLOAT), CAST(i AS FLOAT)]});");
        csvPath = (std::filesystem::temp_directory_path() /
                   ("vector_index_after_failed_copy_" + std::to_string(::getpid()) + ".csv"))
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

    // Un fichier de NUM_ROWS + 1 lignes neuves, sauf la dernière, dont la clé existe déjà.
    void writeCsvWithOneDuplicateKey() const {
        std::ofstream csv(csvPath);
        for (int64_t i = 0; i <= NUM_ROWS; i++) {
            const auto id = i == NUM_ROWS ? 5 : NUM_ROWS + i;
            csv << id << ",\"[1.0,2.0,3.0," << id << ".0]\"\n";
        }
    }

    std::string csvPath;
};

TEST_F(VectorIndexAfterFailedCopyTest, TheIndexIsBuiltOverTheRowsOfTheTable) {
    writeCsvWithOneDuplicateKey();
    auto refused = conn->query("COPY Doc FROM '" + csvPath + "';");
    ASSERT_FALSE(refused->isSuccess()) << "the COPY carries a duplicated key";
    ASSERT_EQ(single("MATCH (n:Doc) RETURN count(*);"), NUM_ROWS);

    ok("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    auto result = conn->query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [8.0, 11.0, 14.0, "
                              "100.0], 10) RETURN node.id;");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    EXPECT_EQ(result->getNumTuples(), 10u);
    // La table reste utilisable, et l'index suit une insertion.
    ok("CREATE (:Doc {id: 100000, vec: [8.0, 11.0, 14.0, 100.5]});");
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), NUM_ROWS + 1);
}

// Des insertions ordinaires puis un COPY dans une transaction, sur une table qui porte un index
// vectoriel. Le COPY verse d'abord les lignes locales dans la table, et l'index les reçoit à
// ce moment, comme il les recevrait au commit : validée, la transaction laisse chaque ligne
// dans l'index, les versées, les copiées et celles insérées après.
// (La même transaction annulée n'est pas couverte ici : un COPY annulé sur une table indexée
// laisse l'index en avance sur sa table, avec ou sans insertions — ticket du 5 octobre 2026.)
TEST_F(VectorIndexAfterFailedCopyTest, InsertsThenACopyInOneTransactionAreAllIndexed) {
    ok("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    {
        std::ofstream csv(csvPath);
        for (int64_t id = 2000; id < 2100; id++) {
            csv << id << ",\"[1.0,2.0,3.0," << id << ".0]\"\n";
        }
    }
    const auto nearest = [&](const std::string& vec) -> int64_t {
        auto result = conn->query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', " + vec +
                                  ", 1) RETURN node.id ORDER BY distance;");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    };
    ok("BEGIN TRANSACTION;");
    ok("UNWIND range(1000, 1002) AS i CREATE (:Doc {id: i, vec: [50.0, 50.0, 50.0, "
       "CAST(4000 + i AS FLOAT)]});");
    ok("COPY Doc FROM '" + csvPath + "';");
    ok("CREATE (:Doc {id: 3000, vec: [70.0, 70.0, 70.0, 9000.0]});");
    ok("COMMIT;");
    const auto check = [&] {
        EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), NUM_ROWS + 104);
        EXPECT_EQ(nearest("[50.0, 50.0, 50.0, 5001.0]"), 1001);
        EXPECT_EQ(nearest("[1.0, 2.0, 3.0, 2050.0]"), 2050);
        EXPECT_EQ(nearest("[70.0, 70.0, 70.0, 9000.0]"), 3000);
        // Une ligne d'avant la transaction, par son vecteur exact.
        EXPECT_EQ(nearest("[15.0, 8.0, 13.0, 100.0]"), 100);
    };
    check();
    createDBAndConn();
    ok("LOAD EXTENSION '" +
        TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension") +
        "';");
    check();
}

} // namespace
