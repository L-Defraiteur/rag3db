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

#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

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

} // namespace
