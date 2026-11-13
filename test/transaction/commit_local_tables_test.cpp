// Un COMMIT réussi perdait les nœuds de certaines tables (question d'arbre principal, 13 novembre
// 2026, réduite par ddmin depuis la trace de rag3weaver). LocalStorage::commit parcourt la table de
// hachage de ses tables locales et valide chaque table de nœuds ; valider une table sous index
// vectoriel insère ses lignes dans l'index HNSW, qui pose des arêtes dans ses tables de relations
// internes, et crée pour elles une table locale neuve dans cette même table de hachage, en plein
// parcours. Quand l'insertion la réorganise, le parcours saute des tables : leurs nœuds ne sont
// jamais publiés, alors que leurs relations, parcourues ensuite, le sont.
//
// Le témoin : une transaction qui insère une ligne dans k tables de nœuds et une ligne vectorielle
// sous index HNSW, puis valide ; chaque table doit avoir gardé sa ligne, pour k de 1 à 40 (assez
// pour franchir plusieurs seuils de réorganisation).
#include <filesystem>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "test_helper/test_helper.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

constexpr int NUM_TABLES = 40;

class CommitLocalTablesTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        if (!std::filesystem::exists(extension)) {
            GTEST_SKIP() << "the vector extension is not built (-DBUILD_EXTENSIONS=vector)";
        }
        createDBAndConn();
        run("LOAD EXTENSION '" + extension + "';");
        for (auto t = 0; t < NUM_TABLES; t++) {
            run("CREATE NODE TABLE T" + std::to_string(t) + "(id INT64, PRIMARY KEY(id));");
        }
        run("CREATE NODE TABLE Doc(id INT64, vec FLOAT[4], PRIMARY KEY(id));");
        run("CREATE (:Doc {id: 0, vec: [1.0, 0.0, 0.0, 0.0]});");
        run("CALL CREATE_VECTOR_INDEX('Doc', 'doc_vec', 'vec', metric := 'l2');");
    }
    void run(const std::string& q) const {
        auto result = conn->query(q);
        ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
    }
    int64_t count(const std::string& table) const {
        auto result = conn->query("MATCH (n:" + table + ") RETURN count(n);");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        return result->isSuccess() ? result->getNext()->getValue(0)->getValue<int64_t>() : -1;
    }
    std::string extension;
};

TEST_F(CommitLocalTablesTest, ACommitWithAVectorIndexKeepsTheRowsOfEveryTable) {
    for (auto k = 1; k <= NUM_TABLES; k++) {
        run("BEGIN TRANSACTION;");
        for (auto t = 0; t < k; t++) {
            run("CREATE (:T" + std::to_string(t) + " {id: " + std::to_string(k) + "});");
        }
        run("CREATE (:Doc {id: " + std::to_string(k) + ", vec: [" + std::to_string(k) +
            ".0, 1.0, 0.5, 0.25]});");
        run("COMMIT;");
        EXPECT_EQ(count("Doc"), k + 1) << "after the transaction of " << k << " tables";
        for (auto t = 0; t < k; t++) {
            EXPECT_EQ(count("T" + std::to_string(t)), k - t)
                << "table T" << t << " lost the row of the transaction of " << k << " tables";
        }
    }
}

} // namespace
