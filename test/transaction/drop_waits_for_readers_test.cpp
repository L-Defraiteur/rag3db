// Un DROP de colonne attend les lecteurs ouverts de sa table, comme sous PostgreSQL (ticket
// docs/coeur-cpp/tickets/2026-11-18-un-alter-ou-un-drop-n-attend-pas-les-lecteurs-de-sa-table.md ;
// conception docs/coeur-cpp/19-11-2026/01-conception-le-drop-attend-les-lecteurs.md). Le lecteur
// garde le verrou de schéma de la table en partagé jusqu'à la fin de sa transaction ; le DROP le
// prend en exclusif et attend. Le lecteur relit donc sa colonne, son instantané honoré, au lieu
// d'être refusé.
#include <atomic>
#include <chrono>
#include <string>
#include <thread>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

using Clock = std::chrono::steady_clock;

class DropWaitsForReaders : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
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

TEST_F(DropWaitsForReaders, ADropOfAColumnWaitsForAnOpenReaderWhoKeepsReadingIt) {
    run(*conn, "CREATE NODE TABLE T(id INT64 PRIMARY KEY, n INT64, s STRING);");
    run(*conn,
        "UNWIND range(1, 20000) AS i CREATE (:T {id: i, n: i, s: 'v' + CAST(i AS STRING)});");
    run(*conn, "CHECKPOINT;");
    Connection reader(database.get());
    run(reader, "BEGIN TRANSACTION READ ONLY;");
    const auto q = "MATCH (t:T) RETURN count(t.s), sum(t.n), min(t.s);";
    const auto before = row(reader, q);
    ASSERT_EQ(before, "20000|200010000|v1\n");
    const auto start = Clock::now();
    std::atomic<double> dropDone{-1};
    std::string dropResult;
    std::thread dropper([&] {
        Connection ddl(database.get());
        auto result = ddl.query("ALTER TABLE T DROP s;");
        dropResult = result->isSuccess() ? "OK" : result->getErrorMessage();
        dropDone = std::chrono::duration<double, std::milli>(Clock::now() - start).count();
        if (result->isSuccess()) {
            ddl.query("CHECKPOINT;");
        }
    });
    std::this_thread::sleep_for(std::chrono::milliseconds(400));
    EXPECT_LT(dropDone.load(), 0) << "[check: a-drop-waits-for-an-open-reader] the DROP ended at "
                                  << dropDone.load() << " ms, while the reader was open";
    EXPECT_EQ(row(reader, q), before) << "[check: an-open-reader-keeps-reading-its-column]";
    const auto commitAt = std::chrono::duration<double, std::milli>(Clock::now() - start).count();
    run(reader, "COMMIT;");
    dropper.join();
    EXPECT_EQ(dropResult, "OK");
    EXPECT_GT(dropDone.load(), commitAt)
        << "[check: a-drop-waits-for-an-open-reader] DROP " << dropDone.load()
        << " ms, reader commit " << commitAt << " ms";
    EXPECT_EQ(row(*conn, "MATCH (t:T) RETURN count(*), sum(t.n);"), "20000|200010000\n");
}

TEST_F(DropWaitsForReaders, ADropGivesUpAfterItsTimeoutNamingTheReaderItWaitedFor) {
    run(*conn, "CREATE NODE TABLE T(id INT64 PRIMARY KEY, n INT64, s STRING);");
    run(*conn, "UNWIND range(1, 1000) AS i CREATE (:T {id: i, n: i, s: 'v'});");
    Connection reader(database.get());
    run(reader, "BEGIN TRANSACTION READ ONLY;");
    ASSERT_EQ(row(reader, "MATCH (t:T) RETURN count(t.s);"), "1000\n");
    run(*conn, "CALL alter_lock_timeout=300;");
    const auto refused = row(*conn, "ALTER TABLE T DROP s;");
    EXPECT_NE(refused.find("read-only, open for"), std::string::npos)
        << "[check: a-drop-timeout-names-the-reader] " << refused;
    EXPECT_EQ(row(reader, "MATCH (t:T) RETURN count(t.s);"), "1000\n");
    run(reader, "COMMIT;");
    run(*conn, "ALTER TABLE T DROP s;");
}

TEST_F(DropWaitsForReaders, ANewReaderWaitsBehindAWaitingDropAtMostItsTimeout) {
    run(*conn, "CREATE NODE TABLE T(id INT64 PRIMARY KEY, n INT64, s STRING);");
    run(*conn, "UNWIND range(1, 1000) AS i CREATE (:T {id: i, n: i, s: 'v'});");
    Connection longReader(database.get());
    run(longReader, "BEGIN TRANSACTION READ ONLY;");
    ASSERT_EQ(row(longReader, "MATCH (t:T) RETURN count(t.s);"), "1000\n");
    std::string dropResult;
    std::thread dropper([&] {
        Connection ddl(database.get());
        ddl.query("CALL alter_lock_timeout=1500;");
        dropResult = row(ddl, "ALTER TABLE T DROP s;");
    });
    std::this_thread::sleep_for(std::chrono::milliseconds(300));
    // Le DROP attend le lecteur long ; un lecteur neuf attend derrière le DROP (file FIFO), au plus
    // jusqu'à ce que le DROP renonce.
    Connection newReader(database.get());
    const auto start = Clock::now();
    const auto read = row(newReader, "MATCH (t:T) RETURN count(t.s);");
    const auto waited = std::chrono::duration<double, std::milli>(Clock::now() - start).count();
    dropper.join();
    EXPECT_EQ(read, "1000\n");
    EXPECT_GT(waited, 600) << "[check: a-new-reader-queues-behind-a-waiting-drop]";
    EXPECT_LT(waited, 5000) << "[check: the-freeze-is-bounded-by-the-drop-timeout]";
    EXPECT_NE(dropResult.find("read-only, open for"), std::string::npos) << dropResult;
    run(longReader, "COMMIT;");
}

} // namespace
