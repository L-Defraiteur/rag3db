// Un ADD COLUMN refusé à mi-chemin ne laisse rien au stockage (ticket
// docs/coeur-cpp/tickets/2026-11-15-ddl-every-minute-meurt-d-un-sigsegv-pile-perdue.md). Dans le
// scénario des vingt écrivains, l'ADD rang de la minute 2 est refusé par le pool plein, et le tour
// meurt d'un SIGSEGV en relisant la colonne b ajoutée à la minute 6 : les quatre tours morts, et
// eux seuls, ont eu cet ADD refusé. Le catalogue annule la colonne refusée, mais le stockage la
// gardait à moitié : NodeTable::addColumn la pose dans ses colonnes, puis groupe par groupe, et
// n'inscrit son annulation qu'à la fin. La colonne suivante se rangeait un cran trop loin, et sa
// lecture tombait sur la morte. Le point de faute add-column-between-chunked-groups refuse l'ADD
// après son premier groupe de lignes, comme le pool plein.
#include <string>

#include "common/test_hooks.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::main;

namespace rag3db {
namespace testing {

class AddColumnRefusedMidway : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
    }
    void TearDown() override {
        TestHooks::armFailPoint(TestFailPoint::NONE);
        EmptyDBTest::TearDown();
    }
    void run(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    std::string one(const std::string& query) {
        auto result = conn->query(query);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        return result->hasNext() ? result->getNext()->toString() : "AUCUNE";
    }
    // Un ADD refusé après son premier groupe de lignes, puis l'ADD suivant, avec son défaut.
    void refuseThenAdd(const std::string& table) {
        TestHooks::armFailPoint(TestFailPoint::ADD_COLUMN_BETWEEN_CHUNKED_GROUPS);
        auto refused = conn->query("ALTER TABLE " + table + " ADD rang INT64 DEFAULT 0;");
        TestHooks::armFailPoint(TestFailPoint::NONE);
        ASSERT_FALSE(refused->isSuccess()) << "[check: setup] the ADD rang was not refused";
        ASSERT_NE(refused->getErrorMessage().find("add-column-between-chunked-groups"),
            std::string::npos)
            << refused->getErrorMessage();
        run("ALTER TABLE " + table + " ADD b STRING DEFAULT 'b-defaut';");
    }
};

TEST_F(AddColumnRefusedMidway, ANodeColumnAddedAfterARefusedAddReadsBack) {
    run("CREATE NODE TABLE T(id INT64, s STRING, PRIMARY KEY(id));");
    run("COPY T FROM (UNWIND range(0, 2999) AS i RETURN i, 's' + CAST(i AS STRING));");
    run("CHECKPOINT;");
    // Un second groupe de lignes, en mémoire, après celui du point de reprise.
    run("UNWIND range(3000, 3099) AS i CREATE (:T {id: i, s: 'neuve'});");
    refuseThenAdd("T");
    EXPECT_EQ(one("MATCH (n:T) WHERE n.b = 'b-defaut' RETURN count(*);"), "3100\n")
        << "[check: a-column-added-after-a-refused-add-reads-its-default]";
    run("MATCH (n:T) WHERE n.id < 10 OR n.id >= 3090 SET n.b = 'b-ecrit';");
    EXPECT_EQ(one("MATCH (n:T) WHERE n.b = 'b-ecrit' RETURN count(*);"), "20\n")
        << "[check: a-column-added-after-a-refused-add-reads-what-was-set]";
    EXPECT_EQ(one("MATCH (n:T) WHERE n.id = 5 RETURN n.s, n.b;"), "s5|b-ecrit\n");
    run("CHECKPOINT;");
    createDBAndConn();
    EXPECT_EQ(one("MATCH (n:T) WHERE n.b = 'b-defaut' RETURN count(*);"), "3080\n")
        << "[check: a-column-added-after-a-refused-add-survives-a-reopening]";
    EXPECT_EQ(one("MATCH (n:T) WHERE n.id = 3095 RETURN n.s, n.b;"), "neuve|b-ecrit\n");
    EXPECT_EQ(one("CALL TABLE_INFO('T') WHERE name = 'rang' RETURN count(*);"), "0\n")
        << "[check: a-refused-add-leaves-nothing]";
}

TEST_F(AddColumnRefusedMidway, ARelColumnAddedAfterARefusedAddReadsBack) {
    run("CREATE NODE TABLE N(id INT64, PRIMARY KEY(id));");
    run("CREATE REL TABLE R(FROM N TO N, w INT64);");
    run("COPY N FROM (UNWIND range(0, 999) AS i RETURN i);");
    run("COPY R FROM (UNWIND range(0, 1999) AS i RETURN i % 1000, (i * 7) % 1000, i);");
    run("CHECKPOINT;");
    run("UNWIND range(0, 49) AS i MATCH (x:N {id: i}), (y:N {id: 999 - i}) CREATE "
        "(x)-[:R {w: 5000 + i}]->(y);");
    refuseThenAdd("R");
    EXPECT_EQ(one("MATCH ()-[r:R]->() WHERE r.b = 'b-defaut' RETURN count(*);"), "2050\n")
        << "[check: a-column-added-after-a-refused-add-reads-its-default] forward";
    EXPECT_EQ(one("MATCH (y:N)<-[r:R]-(x:N) WHERE r.b = 'b-defaut' RETURN count(*);"), "2050\n")
        << "[check: a-column-added-after-a-refused-add-reads-its-default] backward";
    run("MATCH ()-[r:R]->() WHERE r.w >= 5000 SET r.b = 'b-ecrit';");
    EXPECT_EQ(one("MATCH ()-[r:R]->() WHERE r.b = 'b-ecrit' RETURN count(*);"), "50\n")
        << "[check: a-column-added-after-a-refused-add-reads-what-was-set]";
    run("CHECKPOINT;");
    createDBAndConn();
    EXPECT_EQ(one("MATCH ()-[r:R]->() WHERE r.b = 'b-defaut' RETURN count(*);"), "2000\n")
        << "[check: a-column-added-after-a-refused-add-survives-a-reopening]";
    EXPECT_EQ(one("MATCH (y:N)<-[r:R]-(x:N) WHERE r.b = 'b-ecrit' RETURN count(*);"), "50\n")
        << "[check: a-column-added-after-a-refused-add-survives-a-reopening] backward";
}

} // namespace testing
} // namespace rag3db
