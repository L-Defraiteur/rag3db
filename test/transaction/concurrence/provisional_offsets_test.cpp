// Les décalages provisoires des nœuds créés dans une transaction (lot G1 de la seconde session
// cœur C++, 7 novembre 2026 ; ticket 2026-10-05-copy-de-relations-vers-des-noeuds-locaux-sans-
// remappage).
//
// Un nœud créé par le chemin ordinaire vit dans le stockage local de sa transaction, à un
// décalage provisoire qui commence au nombre de lignes validées de sa table. Au commit, il prend
// son décalage définitif, et les relations locales qui le désignent sont remappées
// (LocalRelTable::remapNodeOffsets). Un COPY de relations de la même transaction résout ses bouts
// par la clé et écrit le décalage provisoire directement dans la table de relations : s'il n'est
// pas remappé, et qu'un autre écrivain a validé entre-temps des nœuds dans la même table, les
// relations du COPY désignent les nœuds de l'autre.
//
// Le mode multi-écrivains, par défaut depuis le 11 octobre, rend l'entrelacement possible : une
// insertion tient l'index de sa table en partagé, un COPY l'index de sa propre table.

#include <sys/wait.h>
#include <unistd.h>

#include <filesystem>
#include <string>
#include <vector>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/storage_utils.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class ProvisionalOffsets : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        other = std::make_unique<Connection>(database.get());
    }
    void TearDown() override {
        other.reset();
        EmptyDBTest::TearDown();
    }
    static void ok(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    void ok(const std::string& query) { ok(*conn, query); }

    std::unique_ptr<Connection> other;
};

// Chaque relation du COPY, relue par les clés de ses deux bouts, relie deux nœuds de A ; les nœuds
// de B n'en ont aucune.
static void expectRelsBetweenTheirOwnNodes(Connection& conn) {
    auto result = conn.query("MATCH (a:N)-[r:R]->(b:N) RETURN a.id, b.id, r.k ORDER BY r.k;");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    int rows = 0;
    while (result->hasNext()) {
        auto row = result->getNext();
        const auto from = row->getValue(0)->getValue<int64_t>();
        const auto to = row->getValue(1)->getValue<int64_t>();
        const auto k = row->getValue(2)->getValue<int64_t>();
        EXPECT_EQ(from, k) << "la relation " << k << " part d'un autre nœud";
        EXPECT_EQ(to, k + 1) << "la relation " << k << " arrive à un autre nœud";
        rows++;
    }
    EXPECT_EQ(rows, 9);
    for (int64_t k = 1000; k < 1009; k++) {
        auto back =
            conn.query("MATCH (a:N)-[r:R]->(b:N {id: " + std::to_string(k + 1) + "}) RETURN a.id;");
        ASSERT_TRUE(back->isSuccess()) << back->getErrorMessage();
        ASSERT_TRUE(back->hasNext()) << "le nœud " << k + 1 << " n'a pas sa relation entrante";
        EXPECT_EQ(back->getNext()->getValue(0)->getValue<int64_t>(), k);
    }
    auto stray = conn.query("MATCH (a:N)-[r:R]-(b:N) WHERE a.id >= 2000 RETURN count(*);");
    ASSERT_TRUE(stray->isSuccess()) << stray->getErrorMessage();
    EXPECT_EQ(stray->getNext()->getValue(0)->getValue<int64_t>(), 0)
        << "des relations du COPY désignent les nœuds de l'autre écrivain";
}

// A crée dix nœuds dans sa transaction ; B valide cinq nœuds de la même table ; A les relie par un
// COPY et valide.
TEST_F(ProvisionalOffsets, ACopyOfRelsToLocalNodesKeepsItsEndpointsAfterAnotherWriterCommits) {
    ok("CREATE NODE TABLE N(id INT64, PRIMARY KEY(id));");
    ok("CREATE REL TABLE R(FROM N TO N, k INT64);");
    ok("UNWIND range(0, 9) AS i CREATE (:N {id: i});");

    ok("BEGIN TRANSACTION;");
    ok("UNWIND range(1000, 1009) AS i CREATE (:N {id: i});");
    ok(*other, "UNWIND range(2000, 2004) AS i CREATE (:N {id: i});");
    ok("COPY R FROM (UNWIND range(1000, 1008) AS i RETURN i, i + 1, i);");
    ok("COMMIT;");
    expectRelsBetweenTheirOwnNodes(*conn);
}

// Le même défaut dans l'autre ordre : B valide après le COPY de A, avant la validation de A.
TEST_F(ProvisionalOffsets,
    ACopyOfRelsToLocalNodesKeepsItsEndpointsWhenAnotherWriterCommitsAfterIt) {
    ok("CREATE NODE TABLE N(id INT64, PRIMARY KEY(id));");
    ok("CREATE REL TABLE R(FROM N TO N, k INT64);");
    ok("UNWIND range(0, 9) AS i CREATE (:N {id: i});");

    ok("BEGIN TRANSACTION;");
    ok("UNWIND range(1000, 1009) AS i CREATE (:N {id: i});");
    ok("COPY R FROM (UNWIND range(1000, 1008) AS i RETURN i, i + 1, i);");
    ok(*other, "UNWIND range(2000, 2004) AS i CREATE (:N {id: i});");
    ok("COMMIT;");
    expectRelsBetweenTheirOwnNodes(*conn);
}

// Une relation insérée par un COPY de relations journalisé, puis mise à jour dans la même
// transaction : en service, le COPY l'écrit dans les listes CSR et la mise à jour l'y trouve ; au
// rejeu, l'insertion journalisée (TABLE_INSERTION au milieu de la transaction) la rejouait dans le
// stockage local de la transaction de reprise, et la mise à jour (REL_UPDATE, par son décalage
// définitif) la cherchait dans les listes : « not found in the forward direction », la base ne se
// rouvrait plus (code-rag3db-depot, 8 novembre 2026, lu enregistrement par enregistrement par le
// cœur). L'arête de la même paire est d'abord supprimée, comme dans ce journal.
class RelCopyThenUpdateReplay : public ProvisionalOffsets,
                                public ::testing::WithParamInterface<bool> {};

TEST_P(RelCopyThenUpdateReplay, TheReplayFindsTheCopiedRelToUpdate) {
    const bool withLocalNodes = GetParam();
    other.reset();
    conn.reset();
    database.reset();
    const auto child = fork();
    if (child == 0) {
        int code = 0;
        {
            rag3db::main::Database db(databasePath, *systemConfig);
            Connection a(&db);
            auto run = [&](const std::string& q) {
                auto r = a.query(q);
                if (!r->isSuccess()) {
                    fprintf(stderr, "[child] %s : %s\n", q.c_str(), r->getErrorMessage().c_str());
                    code = 10;
                }
            };
            run("CALL auto_checkpoint=false;");
            run("CREATE NODE TABLE N(id INT64, PRIMARY KEY(id));");
            run("CREATE REL TABLE R(FROM N TO N, k INT64);");
            run("UNWIND range(0, 9) AS i CREATE (:N {id: i});");
            run("MATCH (x:N {id: 1}), (y:N {id: 2}) CREATE (x)-[:R {k: 1}]->(y);");
            run("CHECKPOINT;");
            run("BEGIN TRANSACTION;");
            run("MATCH (:N {id: 1})-[r:R]->(:N {id: 2}) DELETE r;");
            if (withLocalNodes) {
                run("UNWIND range(1000, 1004) AS i CREATE (:N {id: i});");
            }
            run(std::string("COPY R FROM (UNWIND [0] AS i RETURN 1 AS f, 2 AS t, 2 AS k") +
                (withLocalNodes ? " UNION ALL UNWIND [0] AS i RETURN 1000 AS f, 1001 AS t, 3 AS k" :
                                  "") +
                ");");
            run("MATCH (:N {id: 1})-[r:R]->(:N {id: 2}) SET r.k = 20;");
            run("COMMIT;");
            const auto wal = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
            if (code == 0 &&
                (!std::filesystem::exists(wal) || std::filesystem::file_size(wal) == 0)) {
                fprintf(stderr, "[child] le journal est vide : rien à rejouer\n");
                code = 11;
            }
            _exit(code);
        }
    }
    int status = 0;
    waitpid(child, &status, 0);
    ASSERT_TRUE(WIFEXITED(status) && WEXITSTATUS(status) == 0)
        << "[check: setup] le processus de la transaction a rendu " << status;
    createDBAndConn();
    auto forward = conn->query("MATCH (x:N {id: 1})-[r:R]->(y:N) RETURN y.id, r.k;");
    ASSERT_TRUE(forward->isSuccess()) << "[check: replay] " << forward->getErrorMessage();
    ASSERT_TRUE(forward->hasNext()) << "[check: rels-on-their-nodes] la relation 1 -> 2 manque";
    auto row = forward->getNext();
    EXPECT_EQ(row->getValue(0)->getValue<int64_t>(), 2);
    EXPECT_EQ(row->getValue(1)->getValue<int64_t>(), 20) << "[check: rels-on-their-nodes]";
    EXPECT_FALSE(forward->hasNext())
        << "[check: rels-on-their-nodes] l'arête supprimée est revenue";
    auto back = conn->query("MATCH (y:N {id: 2})<-[r:R]-(x:N) RETURN x.id, r.k;");
    ASSERT_TRUE(back->isSuccess()) << back->getErrorMessage();
    ASSERT_TRUE(back->hasNext()) << "[check: directions-agree] la relation manque au sens arrière";
    EXPECT_EQ(back->getNext()->getValue(1)->getValue<int64_t>(), 20) << "[check: directions-agree]";
}

INSTANTIATE_TEST_SUITE_P(Cases, RelCopyThenUpdateReplay, ::testing::Values(false, true),
    [](const ::testing::TestParamInfo<bool>& info) {
        return info.param ? std::string("WithLocalNodes") : std::string("BetweenCommittedNodes");
    });

} // namespace
