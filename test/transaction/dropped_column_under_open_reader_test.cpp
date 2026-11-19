// Une colonne retirée par un DROP reste lisible par un lecteur ouvert avant lui, à travers un point
// de reprise (ticket docs/coeur-cpp/tickets/2026-11-18-le-point-de-reprise-detruit-une-colonne-
// retiree-qu-un-lecteur-plus-ancien-tient-encore.md). Depuis V2-2, un point de reprise n'attend
// plus un lecteur ouvert, qui garde son instantané. Mais NodeTable::checkpoint (et RelTableData)
// détruit chaque colonne dont la propriété n'est plus au catalogue (columns[id].reset()), alors que
// l'instantané du lecteur la voit encore et qu'il la relit : le processus mourait (SIGSEGV dans
// Column::getNullColumn, sous ScanNodeTable). Désormais le lecteur est refusé par un nom
// (« Schema changed, retry: »), et recommence sa transaction ; il ne balaie plus une colonne
// détruite.
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class DroppedColumnUnderOpenReader : public EmptyDBTest {
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
};

TEST_F(DroppedColumnUnderOpenReader, AReaderOpenBeforeADropReadsTheNodeColumnAfterACheckpoint) {
    // Sans les écritures multiples, un lecteur ne prend pas le verrou de sa table et le DROP ne
    // l'attend pas : le refus nommé de la liaison du balayage reste le filet (sous les écritures
    // multiples, le DROP attend le lecteur : drop_waits_for_readers_test.cpp).
    run(*conn, "CALL enable_multi_writes=false;");
    run(*conn, "CREATE NODE TABLE T(id INT64 PRIMARY KEY, n INT64, s STRING);");
    run(*conn,
        "UNWIND range(1, 20000) AS i CREATE (:T {id: i, n: i, s: 'v' + CAST(i AS STRING)});");
    run(*conn, "CHECKPOINT;");
    Connection reader(database.get());
    run(reader, "BEGIN TRANSACTION READ ONLY;");
    const auto q = "MATCH (t:T) RETURN count(t.s), sum(t.n), min(t.s);";
    const auto before = row(reader, q);
    EXPECT_EQ(before, "20000|200010000|v1\n");
    run(*conn, "ALTER TABLE T DROP s;");
    const auto checkpoint = row(*conn, "CHECKPOINT;");
    EXPECT_EQ(checkpoint, "AUCUNE LIGNE") << checkpoint;
    const auto after = row(reader, q);
    EXPECT_NE(after.find("Schema changed, retry:"), std::string::npos)
        << "[check: an-open-reader-of-a-dropped-column-is-refused-by-name] " << after;
    reader.query("ROLLBACK;");
    EXPECT_EQ(row(*conn, "MATCH (t:T) RETURN count(*), sum(t.n);"), "20000|200010000\n");
}

TEST_F(DroppedColumnUnderOpenReader, AReaderOpenBeforeADropReadsTheRelColumnAfterACheckpoint) {
    // Sans les écritures multiples, un lecteur ne prend pas le verrou de sa table et le DROP ne
    // l'attend pas : le refus nommé de la liaison du balayage reste le filet (sous les écritures
    // multiples, le DROP attend le lecteur : drop_waits_for_readers_test.cpp).
    run(*conn, "CALL enable_multi_writes=false;");
    run(*conn, "CREATE NODE TABLE A(id INT64 PRIMARY KEY);");
    run(*conn, "CREATE REL TABLE R(FROM A TO A, w INT64, name STRING);");
    run(*conn, "UNWIND range(1, 3000) AS i CREATE (:A {id: i});");
    run(*conn, "MATCH (a:A), (b:A) WHERE b.id = a.id % 3000 + 1 CREATE (a)-[:R {w: a.id, name: "
               "'r' + CAST(a.id AS STRING)}]->(b);");
    run(*conn, "CHECKPOINT;");
    Connection reader(database.get());
    run(reader, "BEGIN TRANSACTION READ ONLY;");
    const auto q = "MATCH ()-[r:R]->() RETURN count(r.name), sum(r.w), min(r.name);";
    const auto before = row(reader, q);
    EXPECT_EQ(before, "3000|4501500|r1\n");
    run(*conn, "ALTER TABLE R DROP name;");
    const auto checkpoint = row(*conn, "CHECKPOINT;");
    EXPECT_EQ(checkpoint, "AUCUNE LIGNE") << checkpoint;
    const auto after = row(reader, q);
    EXPECT_NE(after.find("Schema changed, retry:"), std::string::npos)
        << "[check: an-open-reader-of-a-dropped-column-is-refused-by-name] " << after;
    reader.query("ROLLBACK;");
    EXPECT_EQ(row(*conn, "MATCH ()-[r:R]->() RETURN count(*), sum(r.w);"), "3000|4501500\n");
}

} // namespace
