// L'index HNSW suit-il l'instantané du lecteur ? (question de recherche, page 30 des vues, 13
// novembre 2026 : le socle du masque avant le top-k.) Un lecteur ouvre une transaction READ ONLY et
// cherche les trois vecteurs les plus proches ; un autre écrivain valide ensuite un vecteur plus
// proche, la suppression d'un des trois et le déplacement d'un autre ; le lecteur relance la même
// recherche dans son instantané. Elle doit rendre exactement ce qu'elle rendait : la recherche lit
// chaque nœud sous la transaction du lecteur (OnDiskEmbeddings::getEmbedding, un nœud invisible a
// un vecteur nul) ; seul le graphe n'est pas versionné. Avec et sans point de reprise avant.
#include <filesystem>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "test_helper/test_helper.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class VectorSearchUnderSnapshot : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        const auto extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        if (!std::filesystem::exists(extension)) {
            GTEST_SKIP() << "the vector extension is not built (-DBUILD_EXTENSIONS=vector)";
        }
        createDBAndConn();
        reader = std::make_unique<Connection>(database.get());
        run(*conn, "LOAD EXTENSION '" + extension + "';");
        run(*conn, "CREATE NODE TABLE Doc(id INT64, vec FLOAT[4], PRIMARY KEY(id));");
        run(*conn, "UNWIND range(0, 19) AS i CREATE (:Doc {id: i, vec: [CAST(i AS FLOAT), 0.0, "
                   "0.0, 0.0]});");
        run(*conn, "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    }
    void TearDown() override {
        reader.reset();
        EmptyDBTest::TearDown();
    }
    static void run(Connection& c, const std::string& q) {
        auto result = c.query(q);
        ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
    }
    // Les trois plus proches de [0.1, 0, 0, 0], par identifiant croissant.
    static std::string top3(Connection& c) {
        auto result = c.query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', CAST([0.1, 0.0, 0.0, "
                              "0.0] AS FLOAT[4]), 3) RETURN node.id AS id ORDER BY id;");
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        std::string ids;
        while (result->hasNext()) {
            ids += result->getNext()->toString();
        }
        return ids;
    }
    void readerKeepsItsSnapshot() {
        run(*reader, "BEGIN TRANSACTION READ ONLY;");
        const auto before = top3(*reader);
        ASSERT_EQ(before, "0\n1\n2\n");
        run(*conn, "CREATE (:Doc {id: 100, vec: [0.1, 0.0, 0.0, 0.0]});");
        run(*conn, "MATCH (d:Doc {id: 0}) DELETE d;");
        run(*conn, "MATCH (d:Doc {id: 1}) SET d.vec = [50.0, 0.0, 0.0, 0.0];");
        EXPECT_EQ(top3(*reader), before)
            << "the reader's search saw writes committed after its snapshot";
        run(*reader, "COMMIT;");
        EXPECT_EQ(top3(*reader), "2\n3\n100\n") << "a new snapshot sees the committed writes";
    }
    std::unique_ptr<Connection> reader;
};

TEST_F(VectorSearchUnderSnapshot, ASearchKeepsItsSnapshotWhileOthersCommit) {
    readerKeepsItsSnapshot();
}

TEST_F(VectorSearchUnderSnapshot, ASearchKeepsItsSnapshotAfterACheckpoint) {
    run(*conn, "CHECKPOINT;");
    readerKeepsItsSnapshot();
}

} // namespace
