// Le remappage des offsets locaux au commit (marche A2). Deux transactions ouvertes en même
// temps numérotent leurs nouveaux nœuds à partir de la même taille de table ; la seconde à
// valider atterrit plus loin que prévu. Tout ce qu'elle a écrit en désignant ses propres
// nœuds par leur numéro provisoire doit suivre : les relations, mais aussi le journal, sans
// quoi un rejeu après panne rattacherait ces écritures aux nœuds de l'autre transaction.
//
// Le banc de concurrence (C3) couvre les relations entre nœuds neufs. Ce fichier couvre le
// reste : mise à jour et suppression d'un nœud neuf, relation neuve mise à jour ou supprimée,
// relation vers un nœud ancien, DETACH DELETE d'un nœud neuf — à chaud, après rejeu du
// journal, puis après point de reprise et réouverture. Deux connexions dans un seul fil :
// aucun hasard d'ordonnancement.

#include <algorithm>
#include <string>
#include <vector>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::testing;

namespace {

class LocalOffsetRemapTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
    }

    static void ok(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    void ok(const std::string& query) { ok(*conn, query); }

    // Tout le contenu, trié : les nœuds, puis chaque relation avec les clés de ses extrémités
    // telles que le moteur les rend et telles que la relation les déclare.
    std::vector<std::string> dump() {
        std::vector<std::string> lines;
        for (const auto* query :
            {"MATCH (n:Item) RETURN 'node', n.id, n.v;",
                "MATCH (a:Item)-[r:Link]->(b:Item) RETURN 'rel', a.id, b.id, r.src, r.dst, r.w;"}) {
            auto result = conn->query(query);
            EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
            while (result->isSuccess() && result->hasNext()) {
                auto line = result->getNext()->toString();
                while (!line.empty() && line.back() == '\n') {
                    line.pop_back();
                }
                lines.push_back(std::move(line));
            }
        }
        std::sort(lines.begin(), lines.end());
        return lines;
    }

    static void link(rag3db::main::Connection& connection, int64_t src, int64_t dst) {
        ok(connection, "MATCH (a:Item {id: " + std::to_string(src) + "}), (b:Item {id: " +
                           std::to_string(dst) + "}) CREATE (a)-[:Link {src: " +
                           std::to_string(src) + ", dst: " + std::to_string(dst) + ", w: 0}]->(b);");
    }

    // Le scénario : deux transactions ouvertes ensemble, validées l'une après l'autre.
    void runOverlappingTransactions(bool secondCommitsFirst) {
        ok("CALL debug_enable_multi_writes=true;");
        ok("CALL auto_checkpoint=false;");
        ok("CALL force_checkpoint_on_close=false;");
        ok("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);");
        ok("CREATE REL TABLE Link(FROM Item TO Item, src INT64, dst INT64, w INT64);");
        ok("CREATE (:Item {id: 1, v: 0});");
        rag3db::main::Connection w0(database.get());
        rag3db::main::Connection w1(database.get());
        ok(w0, "BEGIN TRANSACTION;");
        ok(w1, "BEGIN TRANSACTION;");
        for (auto id = 100; id < 104; id++) {
            ok(w0, "CREATE (:Item {id: " + std::to_string(id) + ", v: 0});");
        }
        for (auto id = 200; id < 204; id++) {
            ok(w1, "CREATE (:Item {id: " + std::to_string(id) + ", v: 0});");
        }
        link(w0, 100, 101);
        link(w0, 101, 102);
        link(w0, 102, 1); // vers un nœud ancien
        link(w1, 200, 201);
        link(w1, 201, 202);
        link(w1, 202, 203);
        link(w1, 203, 1);
        // Ce que la seconde transaction fait à ses propres nœuds et relations.
        ok(w1, "MATCH (n:Item {id: 201}) SET n.v = 7;");
        ok(w1, "MATCH (:Item {id: 202})-[r:Link]->(:Item {id: 203}) DELETE r;");
        ok(w1, "MATCH (:Item {id: 200})-[r:Link]->(:Item {id: 201}) SET r.w = 9;");
        ok(w1, "MATCH (n:Item {id: 203}) DETACH DELETE n;");
        ok(w0, "MATCH (n:Item {id: 103}) SET n.v = 3;");
        if (secondCommitsFirst) {
            ok(w1, "COMMIT;");
            ok(w0, "COMMIT;");
        } else {
            ok(w0, "COMMIT;");
            ok(w1, "COMMIT;");
        }
    }

    void expectTheRightContent(const std::string& moment) {
        std::vector<std::string> expected = {"node|1|0", "node|100|0", "node|101|0",
            "node|102|0", "node|103|3", "node|200|0", "node|201|7", "node|202|0",
            "rel|100|101|100|101|0", "rel|101|102|101|102|0", "rel|102|1|102|1|0",
            "rel|200|201|200|201|9", "rel|201|202|201|202|0"};
        std::sort(expected.begin(), expected.end());
        EXPECT_EQ(dump(), expected) << moment;
    }

    void runAndCheckEverywhere(bool secondCommitsFirst) {
        if (inMemMode) {
            GTEST_SKIP() << "le rejeu et le point de reprise demandent une base sur disque";
        }
        runOverlappingTransactions(secondCommitsFirst);
        expectTheRightContent("à chaud");
        // Arrêt sans point de reprise : la réouverture rejoue le journal.
        createDBAndConn();
        expectTheRightContent("après rejeu du journal");
        ok("CHECKPOINT;");
        createDBAndConn();
        expectTheRightContent("après point de reprise et réouverture");
    }
};

TEST_F(LocalOffsetRemapTest, FirstOpenedCommitsFirst) {
    runAndCheckEverywhere(false /* secondCommitsFirst */);
}

TEST_F(LocalOffsetRemapTest, SecondOpenedCommitsFirst) {
    runAndCheckEverywhere(true /* secondCommitsFirst */);
}

} // namespace
