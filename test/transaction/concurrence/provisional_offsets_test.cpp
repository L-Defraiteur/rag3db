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

#include <filesystem>
#include <string>
#include <vector>

#include <sys/wait.h>
#include <unistd.h>

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
        auto back = conn.query("MATCH (a:N)-[r:R]->(b:N {id: " + std::to_string(k + 1) +
                               "}) RETURN a.id;");
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
TEST_F(ProvisionalOffsets, ACopyOfRelsToLocalNodesKeepsItsEndpointsWhenAnotherWriterCommitsAfterIt) {
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

// Deux versements des nœuds locaux dans une même transaction (G1 verse à chaque COPY de
// relations), sous un second écrivain qui valide des nœuds entre les deux, avec des relations
// locales créées avant le premier et entre les deux, puis la mise à jour validée de l'une d'elles :
// la forme de la réindexation de la montre (Scope → Scope par CONSUMED_BY) après laquelle le
// journal de code-rag3db-depot ne se rejouait plus (8 novembre 2026, ticket
// 2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-concurrente-pendant-une-indexation :
// « Relationship … was not found in the forward direction »). Le processus meurt base ouverte ; la
// réouverture rejoue le journal, et chaque relation est à ses deux bouts, dans les deux sens.
TEST_F(ProvisionalOffsets, TwoFlushesInOneTransactionKeepEveryRelOnItsNodesThroughTheJournal) {
    other.reset();
    conn.reset();
    database.reset();
    const auto child = fork();
    if (child == 0) {
        int code = 0;
        {
            rag3db::main::Database db(databasePath, *systemConfig);
            Connection a(&db);
            Connection b(&db);
            auto run = [&](Connection& c, const std::string& q) {
                auto r = c.query(q);
                if (!r->isSuccess()) {
                    fprintf(stderr, "[child] %s : %s\n", q.c_str(), r->getErrorMessage().c_str());
                    code = 10;
                }
            };
            run(a, "CALL auto_checkpoint=false;");
            run(a, "CREATE NODE TABLE N(id INT64, PRIMARY KEY(id));");
            run(a, "CREATE REL TABLE R(FROM N TO N, k INT64);");
            run(a, "UNWIND range(0, 9) AS i CREATE (:N {id: i});");
            run(a, "CHECKPOINT;");
            run(a, "BEGIN TRANSACTION;");
            run(a, "UNWIND range(1000, 1009) AS i CREATE (:N {id: i});");
            run(a, "MATCH (x:N {id: 1000}), (y:N {id: 1001}) CREATE (x)-[:R {k: 1}]->(y);");
            run(b, "UNWIND range(2000, 2004) AS i CREATE (:N {id: i});");
            run(a, "COPY R FROM (UNWIND [0] AS i RETURN 1, 2, 100);");
            run(a, "UNWIND range(1100, 1109) AS i CREATE (:N {id: i});");
            run(a, "MATCH (x:N {id: 1100}), (y:N {id: 1101}) CREATE (x)-[:R {k: 2}]->(y);");
            run(a, "MATCH (x:N {id: 1000}), (y:N {id: 1105}) CREATE (x)-[:R {k: 3}]->(y);");
            run(b, "UNWIND range(3000, 3004) AS i CREATE (:N {id: i});");
            run(a, "COPY R FROM (UNWIND [0] AS i RETURN 3, 4, 101);");
            run(a, "COMMIT;");
            run(a, "MATCH ()-[r:R {k: 3}]->() SET r.k = 30;");
            run(a, "MATCH ()-[r:R {k: 1}]->() SET r.k = 10;");
            const auto wal = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
            if (code == 0 && (!std::filesystem::exists(wal) || std::filesystem::file_size(wal) == 0)) {
                fprintf(stderr, "[child] le journal est vide : rien à rejouer\n");
                code = 11;
            }
            // Mourir base ouverte : le journal est tout ce qui reste.
            _exit(code);
        }
    }
    int status = 0;
    waitpid(child, &status, 0);
    ASSERT_TRUE(WIFEXITED(status) && WEXITSTATUS(status) == 0)
        << "[check: setup] le processus de la transaction a rendu " << status;
    createDBAndConn();
    // Le sens avant : chaque relation depuis sa source.
    const std::vector<std::vector<int64_t>> expected = {
        {1, 2, 100}, {3, 4, 101}, {1000, 1001, 10}, {1000, 1105, 30}, {1100, 1101, 2}};
    auto forward = conn->query("MATCH (x:N)-[r:R]->(y:N) RETURN x.id, y.id, r.k ORDER BY x.id, y.id;");
    ASSERT_TRUE(forward->isSuccess()) << "[check: replay] " << forward->getErrorMessage();
    std::vector<std::vector<int64_t>> seen;
    while (forward->hasNext()) {
        auto row = forward->getNext();
        seen.push_back({row->getValue(0)->getValue<int64_t>(), row->getValue(1)->getValue<int64_t>(),
            row->getValue(2)->getValue<int64_t>()});
    }
    EXPECT_EQ(seen, expected) << "[check: rels-on-their-nodes]";
    // Le sens arrière : chaque relation depuis sa destination.
    for (const auto& rel : expected) {
        auto back = conn->query("MATCH (y:N {id: " + std::to_string(rel[1]) +
                                "})<-[r:R]-(x:N) RETURN x.id, r.k ORDER BY x.id;");
        ASSERT_TRUE(back->isSuccess()) << back->getErrorMessage();
        bool found = false;
        while (back->hasNext()) {
            auto row = back->getNext();
            found |= row->getValue(0)->getValue<int64_t>() == rel[0] &&
                     row->getValue(1)->getValue<int64_t>() == rel[2];
        }
        EXPECT_TRUE(found) << "[check: directions-agree] la relation " << rel[0] << " -> " << rel[1]
                           << " manque au sens arrière";
    }
}

} // namespace
