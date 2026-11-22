#include <string>

#include "bench_harness.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

// Les tables d'un index vectoriel quand la table ou l'index ont un nom spécial (ticket
// docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-22-des-noms-ecrits-dans-du-cypher-genere-sans-echappement.md,
// famille 3). CREATE_VECTOR_INDEX et DROP_VECTOR_INDEX se réécrivent en Cypher (CREATE REL TABLE
// _<id>_<index>_UPPER (FROM <table> TO <table>), DROP TABLE …) qui écrivait ces noms sans accents graves :
// une espace, un tiret ou un accent grave dans le nom de la table ou de l'index cassait l'instruction. Le
// témoin : une table « my ` docs » et un index « idx-1 `b » ; l'index se crée, répond, et se supprime sans
// laisser ses tables de relations.

namespace rag3db {
namespace testing {

class VectorIndexSpecialNames : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        concurrency::loadVectorExtension(*conn);
    }
    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    std::string tableNames() {
        auto result = conn->query("CALL show_tables() RETURN name ORDER BY name;");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        std::string names;
        while (result->isSuccess() && result->hasNext()) {
            names += "[" + result->getNext()->getValue(0)->toString() + "]";
        }
        return names;
    }
};

TEST_F(VectorIndexSpecialNames, AnIndexOnATableWithSpecialNamesIsCreatedQueriedAndDropped) {
    ok("CREATE NODE TABLE `my `` docs`(id INT64 PRIMARY KEY, vec FLOAT[3]);");
    ok("CREATE (:`my `` docs` {id: 1, vec: [1.0, 0.0, 0.0]}), (:`my `` docs` {id: 2, vec: [0.0, 1.0, "
       "0.0]}), (:`my `` docs` {id: 3, vec: [0.0, 0.0, 1.0]});");
    if (HasFatalFailure()) {
        return;
    }
    auto create = conn->query("CALL CREATE_VECTOR_INDEX('my ` docs', 'idx-1 `b', 'vec');");
    ASSERT_TRUE(create->isSuccess())
        << "[check: vector-index-special-names-create] " << create->getErrorMessage();
    auto query = conn->query(
        "CALL QUERY_VECTOR_INDEX('my ` docs', 'idx-1 `b', [1.0, 0.0, 0.0], 1) RETURN node.id;");
    ASSERT_TRUE(query->isSuccess())
        << "[check: vector-index-special-names-query] " << query->getErrorMessage();
    ASSERT_TRUE(query->hasNext()) << "[check: vector-index-special-names-query] no row";
    EXPECT_EQ(query->getNext()->getValue(0)->getValue<int64_t>(), 1)
        << "[check: vector-index-special-names-query] the nearest row";
    auto drop = conn->query("CALL DROP_VECTOR_INDEX('my ` docs', 'idx-1 `b');");
    ASSERT_TRUE(drop->isSuccess())
        << "[check: vector-index-special-names-drop] " << drop->getErrorMessage();
    EXPECT_EQ(tableNames(), "[my ` docs]")
        << "[check: vector-index-special-names-drop] the index leaves no table behind";
}

} // namespace testing
} // namespace rag3db
