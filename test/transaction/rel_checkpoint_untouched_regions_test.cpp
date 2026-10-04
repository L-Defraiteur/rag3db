// Un point de reprise ne doit pas effacer les relations des régions qu'il n'a pas regardées.
//
// Dans un sens donné, les relations d'une table sont rangées par groupes de 131 072 nœuds
// d'origine, découpés en régions de 1 024 nœuds. Le point de reprise ne réécrit que les
// régions modifiées. Il comptait les relations restantes dans ces seules régions et, à zéro,
// concluait que le groupe entier était vide : tout ce que le groupe avait sur disque dans ce
// sens était libéré, relations des autres régions comprises. L'autre sens gardait les
// siennes ; rien ne signalait la perte, et elle tenait à une réouverture.
//
// Un seul écrivain, en service. Trouvé par la revue des amonts de la session du banc
// (4 octobre 2026) ; Ladybug l'a corrigé le 13 mars 2026 (0be6fa597), Vela aussi (e5e700e73).

#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::testing;

namespace {

class RelCheckpointUntouchedRegionsTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "needs an on-disk database: the defect is in what a checkpoint writes";
        }
        createDBAndConn();
        ok("CALL auto_checkpoint=false;");
        ok("CREATE NODE TABLE person(id INT64 PRIMARY KEY);");
        ok("CREATE REL TABLE knows(FROM person TO person);");
        // 3 000 nœuds : trois régions de 1 024 dans le même groupe.
        ok("UNWIND range(0, 2999) AS i CREATE (:person {id: i});");
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

    void link(int64_t from, int64_t to) {
        ok("MATCH (a:person {id: " + std::to_string(from) + "}), (b:person {id: " +
            std::to_string(to) + "}) CREATE (a)-[:knows]->(b);");
    }

    // Les relations sortantes d'un nœud, puis ses relations entrantes : les deux sens de la
    // table.
    int64_t outgoing(int64_t id) {
        return single("MATCH (a:person {id: " + std::to_string(id) +
                      "})-[r:knows]->() RETURN count(r);");
    }
    int64_t incoming(int64_t id) {
        return single("MATCH (a:person {id: " + std::to_string(id) +
                      "})<-[r:knows]-() RETURN count(r);");
    }

    void expectTheOtherRegionsKeptTheirRelations() {
        EXPECT_EQ(outgoing(1500), 1);
        EXPECT_EQ(outgoing(2900), 1);
        EXPECT_EQ(incoming(1), 2);
        EXPECT_EQ(single("MATCH (:person)-[r:knows]->(:person) RETURN count(r);"), 2);
    }
};

// La recette : trois nœuds de trois régions mènent au nœud 1 ; point de reprise ; la relation
// de la première région est supprimée ; point de reprise. Les deux autres doivent rester.
TEST_F(RelCheckpointUntouchedRegionsTest, DeletingTheOnlyRelOfOneRegion) {
    link(0, 1);
    link(1500, 1);
    link(2900, 1);
    ok("CHECKPOINT;");
    ok("MATCH (a:person {id: 0})-[r:knows]->() DELETE r;");
    ok("CHECKPOINT;");
    expectTheOtherRegionsKeptTheirRelations();
    createDBAndConn(); // et après une réouverture
    expectTheOtherRegionsKeptTheirRelations();
}

// Une relation créée puis supprimée entre deux points de reprise : aucune région n'est à
// réécrire, mais la relation est encore en mémoire, marquée invalide. Le point de reprise
// sortait sans vider cette partie, et le balayage suivant plantait (SIGSEGV dans
// CSRNodeGroup::scanCommittedInMemRandom). Ladybug a gardé chaque lecture (ba5f38815) ; ici
// c'est le point de reprise qui finit son travail.
TEST_F(RelCheckpointUntouchedRegionsTest, CreatingThenDeletingARelInAnEmptyRegion) {
    link(1500, 1);
    link(2900, 1);
    ok("CHECKPOINT;");
    link(0, 1);
    ok("MATCH (a:person {id: 0})-[r:knows]->() DELETE r;");
    ok("CHECKPOINT;");
    expectTheOtherRegionsKeptTheirRelations();
}

// Le garde-fou : quand tout le groupe est réellement vide, le point de reprise le libère, et
// la table reste utilisable.
TEST_F(RelCheckpointUntouchedRegionsTest, AGroupThatIsReallyEmptyIsStillEmptied) {
    link(0, 1);
    link(1500, 1);
    ok("CHECKPOINT;");
    ok("MATCH (:person)-[r:knows]->(:person) DELETE r;");
    ok("CHECKPOINT;");
    EXPECT_EQ(single("MATCH (:person)-[r:knows]->(:person) RETURN count(r);"), 0);
    link(2900, 1);
    ok("CHECKPOINT;");
    EXPECT_EQ(outgoing(2900), 1);
    EXPECT_EQ(incoming(1), 1);
}

// La même, répétée, avec des écritures entre les deux : la table reste utilisable.
TEST_F(RelCheckpointUntouchedRegionsTest, CreatingThenDeletingRepeatedly) {
    link(1500, 1);
    ok("CHECKPOINT;");
    for (auto round = 0; round < 3; round++) {
        link(0, 1);
        link(2900, 1);
        ok("MATCH (a:person)-[r:knows]->() WHERE a.id IN [0, 2900] DELETE r;");
        ok("CHECKPOINT;");
        EXPECT_EQ(outgoing(0), 0);
        EXPECT_EQ(outgoing(1500), 1);
        EXPECT_EQ(incoming(1), 1);
    }
    link(2900, 1);
    ok("CHECKPOINT;");
    expectTheOtherRegionsKeptTheirRelations();
}

} // namespace
