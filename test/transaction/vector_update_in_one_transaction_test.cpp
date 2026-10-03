// Beaucoup de mises à jour de vecteur dans une même transaction, sur une table indexée par
// HNSW. Chaque mise à jour retire les arêtes de la ligne et la réinsère ; l'index relit alors
// des arêtes que la transaction vient d'écrire. Tant que cette lecture était fausse (voir
// uncommitted_rels_scan_test.cpp), une ligne semblait avoir des milliers de voisins, et la
// transaction finissait par échouer : « bitset::set: __position (which is 2049) >= _Nb (which
// is 2048) » — mille SET en une instruction, ou quelques centaines dans un BEGIN … COMMIT.
//
// Ces cas tiennent ce que la correction garantit : la transaction aboutit, la table porte
// les nouveaux vecteurs, l'index répond. Ce sont des témoins probabilistes de l'échec : sans la
// correction, la forme en une instruction a rougi une passe sur trois, la forme en un bloc
// n'a pas rougi en trois passes (elle avait échoué vers la 465e ligne dans une autre mesure) ;
// le témoin déterministe du défaut est uncommitted_rels_scan_test.cpp. Ils ne tiennent PAS
// que toutes les lignes restent joignables dans l'index après une mise à jour massive : ce
// n'est pas encore vrai (journal des chantiers, §6). Ils se sautent si l'extension vector
// n'est pas bâtie.

#include <filesystem>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "test_helper/test_helper.h"

using namespace rag3db::testing;

namespace {

class VectorUpdateInOneTransactionTest : public EmptyDBTest {
protected:
    static constexpr int64_t NUM_ROWS = 1000;

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
        ok("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
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

    // Le nouveau vecteur d'une ligne, en Cypher, à partir de n.id.
    const std::string updated = "[CAST(n.id % 13 AS FLOAT) + 0.5, CAST(n.id % 19 AS FLOAT), "
                                "CAST(n.id % 31 AS FLOAT), CAST(n.id AS FLOAT) + 0.25]";

    void expectEveryRowUpdatedAndTheIndexAnswers() {
        // La dernière composante de chaque nouveau vecteur finit par .25.
        EXPECT_EQ(single("MATCH (n:Doc) WHERE n.vec[4] = CAST(n.id AS FLOAT) + 0.25 RETURN "
                         "count(*);"),
            NUM_ROWS);
        auto result = conn->query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [8.5, 11.0, 14.0, "
                                  "500.25], 10) RETURN node.id;");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        EXPECT_EQ(result->getNumTuples(), 10u);
    }
};

TEST_F(VectorUpdateInOneTransactionTest, AThousandUpdatesInOneStatement) {
    ok("MATCH (n:Doc) WHERE n.id >= 0 AND n.id < 1000 SET n.vec = " + updated + ";");
    expectEveryRowUpdatedAndTheIndexAnswers();
}

TEST_F(VectorUpdateInOneTransactionTest, AThousandUpdatesInOneBlock) {
    ok("BEGIN TRANSACTION;");
    for (int64_t id = 0; id < NUM_ROWS; id++) {
        ok("MATCH (n:Doc) WHERE n.id >= " + std::to_string(id) + " AND n.id < " +
            std::to_string(id + 1) + " SET n.vec = " + updated + ";");
    }
    ok("COMMIT;");
    expectEveryRowUpdatedAndTheIndexAnswers();
}

} // namespace
