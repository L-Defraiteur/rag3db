// Un graphe projeté est un objet de la connexion (GraphEntrySet), pas une écriture de la base :
// PROJECT_GRAPH_CYPHER, PROJECT_GRAPH et DROP_PROJECTED_GRAPH passent dans une transaction READ
// ONLY, comme une lecture, et une recherche vectorielle sur le graphe projeté aussi. C'est le
// chemin de la branche dense de rag3weaver sous l'instantané de lecture d'un appel (L6). Sur
// master, le CALL autonome d'une fonction est classé écriture sans regarder la fonction : « Can
// not execute a write query inside a read-only transaction » (rouge). Une vraie écriture reste
// refusée sous READ ONLY.
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "test_helper/test_helper.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class ProjectedGraphReadOnly : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        const auto extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        run("LOAD EXTENSION '" + extension + "';");
        run("CREATE NODE TABLE T(id INT64 PRIMARY KEY, v FLOAT[4]);");
        run("UNWIND range(1, 50) AS i CREATE (:T {id: i, v: [CAST(i AS FLOAT), 0.0, 0.0, 0.0]});");
        run("CALL CREATE_VECTOR_INDEX('T', 'idx', 'v', metric := 'l2');");
    }
    void run(const std::string& q) const {
        auto result = conn->query(q);
        ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
    }
    std::string row(const std::string& q) const {
        auto result = conn->query(q);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        return result->hasNext() ? result->getNext()->toString() : "AUCUNE LIGNE";
    }
};

TEST_F(ProjectedGraphReadOnly, ProjectingAndQueryingAGraphIsAllowedInAReadOnlyTransaction) {
    run("BEGIN TRANSACTION READ ONLY;");
    EXPECT_EQ(row("CALL DROP_PROJECTED_GRAPH('g', skip_if_not_exists := true);"), "AUCUNE LIGNE")
        << "[check: drop-if-exists-is-read-only]";
    EXPECT_EQ(row("CALL PROJECT_GRAPH_CYPHER('g', 'MATCH (n:T) WHERE n.id > 10 RETURN n');"),
        "AUCUNE LIGNE")
        << "[check: project-graph-cypher-is-read-only]";
    EXPECT_EQ(row("CALL QUERY_VECTOR_INDEX('g', 'idx', CAST([12.0, 0.0, 0.0, 0.0] AS FLOAT[4]), 1) "
                  "RETURN node.id;"),
        "12\n")
        << "[check: vector-search-on-projected-graph-is-read-only]";
    EXPECT_EQ(row("CALL PROJECT_GRAPH('h', ['T'], []);"), "AUCUNE LIGNE")
        << "[check: project-graph-is-read-only]";
    EXPECT_EQ(row("CALL DROP_PROJECTED_GRAPH('h');"), "AUCUNE LIGNE");
    EXPECT_EQ(row("CALL DROP_PROJECTED_GRAPH('g');"), "AUCUNE LIGNE")
        << "[check: drop-is-read-only]";
    // Une vraie écriture reste une écriture.
    const auto refused = row("CREATE (:T {id: 1000, v: [0.0, 0.0, 0.0, 0.0]});");
    EXPECT_TRUE(refused.starts_with("REFUS") &&
                refused.find("read-only transaction") != std::string::npos)
        << "[check: a-write-is-still-refused] " << refused;
    run("ROLLBACK;");
    EXPECT_EQ(row("MATCH (t:T) RETURN count(*);"), "50\n");
}

} // namespace
