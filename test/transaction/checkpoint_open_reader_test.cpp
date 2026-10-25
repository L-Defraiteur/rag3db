// V2-2 : un lecteur ouvert AVANT un point de reprise n'est plus attendu par lui, et garde son
// instantané : le point de reprise fond ce qui est validé au plus vieux instantané ouvert (F) et
// porte dans le bloc neuf les versions postérieures (mises à jour, suppressions, insertions avec
// leur horodatage). Le lecteur relit la même chose avant, pendant et après ; un lecteur nouveau
// et la base rouverte voient l'état validé. Sur master, le CHECKPOINT attend le lecteur, borné,
// puis échoue (« Timeout waiting for active transactions to leave ») : rouge par le refus.
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class CheckpointOpenReader : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "a checkpoint is the subject: on-disk database needed";
        }
        createDBAndConn();
        run(*conn, "CALL auto_checkpoint=false;");
    }
    static void run(Connection& connection, const std::string& q) {
        auto result = connection.query(q);
        ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
    }
    static std::string row(Connection& connection, const std::string& q) {
        auto result = connection.query(q);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        return result->hasNext() ? result->getNext()->toString() : "AUCUNE LIGNE";
    }
    void reopen() {
        conn.reset();
        database.reset();
        createDBAndConn();
    }
};

// Une table de nœuds : une ligne modifiée, une supprimée, une créée et validées après le BEGIN
// du lecteur ; le point de reprise ne l'attend pas ; il relit son instantané pendant et après.
TEST_F(CheckpointOpenReader, OpenReaderOnNodesIsNotWaitedForAndKeepsItsSnapshot) {
    run(*conn, "CREATE NODE TABLE T(id INT64 PRIMARY KEY, n INT64, s STRING);");
    run(*conn, "UNWIND range(1, 20000) AS i CREATE (:T {id: i, n: i, s: 'v' + CAST(i AS STRING)});");
    run(*conn, "CHECKPOINT;");
    Connection reader(database.get());
    run(reader, "BEGIN TRANSACTION READ ONLY;");
    const auto q = "MATCH (t:T) RETURN count(*), sum(t.n), sum(CASE WHEN t.id = 9 THEN 1 ELSE 0 "
                   "END), min(t.n);";
    const auto before = row(reader, q);
    const auto row7Before = row(reader, "MATCH (t:T {id: 7}) RETURN t.n, t.s;");
    EXPECT_EQ(before, "20000|200010000|1|1\n");
    EXPECT_EQ(row7Before, "7|v7\n");
    run(*conn, "MATCH (t:T {id: 7}) SET t.n = -1, t.s = 'changée';");
    run(*conn, "MATCH (t:T {id: 9}) DELETE t;");
    run(*conn, "CREATE (:T {id: 20001, n: 100, s: 'neuve'});");
    const auto checkpoint = row(*conn, "CHECKPOINT;");
    EXPECT_EQ(checkpoint, "AUCUNE LIGNE")
        << "[check: checkpoint-does-not-wait-for-an-open-reader] " << checkpoint;
    // Pendant : le même instantané (les versions postérieures sont portées, pas fondues).
    EXPECT_EQ(row(reader, q), before) << "[check: open-reader-keeps-its-snapshot]";
    EXPECT_EQ(row(reader, "MATCH (t:T {id: 7}) RETURN t.n, t.s;"), row7Before);
    EXPECT_EQ(row(reader, "MATCH (t:T {id: 20001}) RETURN t.n;"), "AUCUNE LIGNE");
    run(reader, "COMMIT;");
    // Après, un lecteur nouveau voit l'état validé, et la base rouverte aussi.
    const auto after = "20000|200010083|0|-1\n";
    EXPECT_EQ(row(*conn, q), after);
    EXPECT_EQ(row(*conn, "MATCH (t:T {id: 7}) RETURN t.n, t.s;"), "-1|changée\n");
    reopen();
    EXPECT_EQ(row(*conn, q), after);
    EXPECT_EQ(row(*conn, "MATCH (t:T {id: 7}) RETURN t.n, t.s;"), "-1|changée\n");
    EXPECT_EQ(row(*conn, "MATCH (t:T {id: 20001}) RETURN t.n, t.s;"), "100|neuve\n");
}

// Une table de relations : la même chose par le chemin CSR (une relation modifiée, une
// supprimée, une créée).
TEST_F(CheckpointOpenReader, OpenReaderOnRelsIsNotWaitedForAndKeepsItsSnapshot) {
    run(*conn, "CREATE NODE TABLE A(id INT64 PRIMARY KEY);");
    run(*conn, "CREATE REL TABLE R(FROM A TO A, w INT64, name STRING);");
    run(*conn, "UNWIND range(1, 3000) AS i CREATE (:A {id: i});");
    run(*conn, "MATCH (a:A), (b:A) WHERE b.id = a.id % 3000 + 1 CREATE (a)-[:R {w: a.id, name: "
               "'r' + CAST(a.id AS STRING)}]->(b);");
    run(*conn, "CHECKPOINT;");
    Connection reader(database.get());
    run(reader, "BEGIN TRANSACTION READ ONLY;");
    const auto q = "MATCH ()-[r:R]->() RETURN count(*), sum(r.w), min(r.w);";
    const auto before = row(reader, q);
    EXPECT_EQ(before, "3000|4501500|1\n");
    const auto rel1500Before = row(reader, "MATCH (a:A {id: 1500})-[r:R]->() RETURN r.w, r.name;");
    EXPECT_EQ(rel1500Before, "1500|r1500\n");
    run(*conn, "MATCH (a:A {id: 1500})-[r:R]->() SET r.w = -5, r.name = 'changée';");
    run(*conn, "MATCH (a:A {id: 1501})-[r:R]->() DELETE r;");
    run(*conn, "MATCH (a:A {id: 1}), (b:A {id: 1}) CREATE (a)-[:R {w: 7, name: 'neuve'}]->(b);");
    const auto checkpoint = row(*conn, "CHECKPOINT;");
    EXPECT_EQ(checkpoint, "AUCUNE LIGNE")
        << "[check: checkpoint-does-not-wait-for-an-open-reader] " << checkpoint;
    EXPECT_EQ(row(reader, q), before) << "[check: open-reader-keeps-its-snapshot]";
    EXPECT_EQ(row(reader, "MATCH (a:A {id: 1500})-[r:R]->() RETURN r.w, r.name;"), rel1500Before);
    EXPECT_EQ(row(reader, "MATCH (a:A {id: 1501})-[r:R]->() RETURN r.w;"), "1501\n");
    run(reader, "COMMIT;");
    // 4501500 - 1500 - 5 (1500 → -5) - 1501 (supprimée) + 7 (neuve)
    const auto after = "3000|4498501|-5\n";
    EXPECT_EQ(row(*conn, q), after);
    reopen();
    EXPECT_EQ(row(*conn, q), after);
    EXPECT_EQ(row(*conn, "MATCH (a:A {id: 1500})-[r:R]->() RETURN r.w, r.name;"), "-5|changée\n");
    EXPECT_EQ(row(*conn, "MATCH (a:A {id: 1501})-[r:R]->() RETURN r.w;"), "AUCUNE LIGNE");
}

} // namespace
