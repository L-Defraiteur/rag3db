// CALL lock_tables([...]) : un paquet prend, en première instruction de sa transaction, le verrou
// de schéma de chaque table qu'il écrira, en partagé. Un ALTER validé entre son BEGIN et cette
// prise la refuse avant toute écriture (le paquet recommence sur place) ; une fois la prise faite,
// un ALTER ou un DROP de ces tables attend le COMMIT du paquet, et deux paquets ne s'attendent pas
// (question d'everything declarative, 2.7 (i), 19 novembre 2026).
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

double millisSince(Clock::time_point start) {
    return std::chrono::duration<double, std::milli>(Clock::now() - start).count();
}

class LockTables : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        run(*conn, "CALL enable_multi_writes=true;");
        run(*conn, "CREATE NODE TABLE T(id INT64 PRIMARY KEY, v INT64);");
        run(*conn, "CREATE REL TABLE R(FROM T TO T, w INT64);");
        run(*conn, "UNWIND range(1, 10) AS i CREATE (:T {id: i, v: i});");
    }
    static void run(Connection& connection, const std::string& q) {
        auto result = connection.query(q);
        ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
    }
    static std::string outcome(Connection& connection, const std::string& q) {
        auto result = connection.query(q);
        return result->isSuccess() ? "OK" : "REFUS " + result->getErrorMessage();
    }
};

TEST_F(LockTables, AnAlterCommittedAfterTheBeginRefusesTheLockBeforeAnyWrite) {
    Connection other(database.get());
    run(*conn, "BEGIN TRANSACTION;");
    run(other, "ALTER TABLE T ADD note STRING DEFAULT 'n';");
    const auto refused = outcome(*conn, "CALL lock_tables(['T', 'R']);");
    EXPECT_NE(refused.find("Schema changed, retry:"), std::string::npos)
        << "[check: lock-tables-is-refused-by-an-alter-after-the-begin] " << refused;
    run(*conn, "ROLLBACK;");
    // Recommencé sur place : le paquet se lie sur le nouveau schéma et va au bout.
    run(*conn, "BEGIN TRANSACTION;");
    EXPECT_EQ(outcome(*conn, "CALL lock_tables(['T', 'R']);"), "OK")
        << "[check: the-batch-started-again-locks-its-tables]";
    run(*conn, "MATCH (n:T) WHERE n.id = 1 SET n.note = 'reprise';");
    run(*conn, "COMMIT;");
}

TEST_F(LockTables, AnAlterWaitsForTheCommitOfABatchThatLockedItsTables) {
    run(*conn, "BEGIN TRANSACTION;");
    ASSERT_EQ(outcome(*conn, "CALL lock_tables(['R', 'T']);"), "OK");
    const auto start = Clock::now();
    std::atomic<double> alterDone{-1};
    std::string alterResult;
    std::thread alterer([&] {
        Connection ddl(database.get());
        alterResult = outcome(ddl, "ALTER TABLE R ADD note STRING DEFAULT 'n';");
        alterDone = millisSince(start);
    });
    std::this_thread::sleep_for(std::chrono::milliseconds(400));
    EXPECT_LT(alterDone.load(), 0)
        << "[check: an-alter-waits-for-a-locked-batch] the ALTER ended at " << alterDone.load()
        << " ms: " << alterResult;
    // Le paquet écrit après coup, sans être refusé : l'ALTER n'a pas pu valider.
    EXPECT_EQ(outcome(*conn, "MATCH (a:T), (b:T) WHERE a.id = 1 AND b.id = 2 CREATE "
                             "(a)-[:R {w: 1}]->(b);"),
        "OK")
        << "[check: a-locked-batch-is-not-refused]";
    const auto commitAt = millisSince(start);
    run(*conn, "COMMIT;");
    alterer.join();
    EXPECT_EQ(alterResult, "OK");
    EXPECT_GT(alterDone.load(), commitAt) << "[check: an-alter-waits-for-a-locked-batch]";
}

TEST_F(LockTables, TwoBatchesThatLockTheSameTablesDoNotWaitForEachOther) {
    Connection second(database.get());
    run(*conn, "BEGIN TRANSACTION;");
    ASSERT_EQ(outcome(*conn, "CALL lock_tables(['T', 'R']);"), "OK");
    run(second, "BEGIN TRANSACTION;");
    const auto start = Clock::now();
    EXPECT_EQ(outcome(second, "CALL lock_tables(['R', 'T']);"), "OK");
    EXPECT_LT(millisSince(start), 1000) << "[check: two-batches-do-not-serialize]";
    run(*conn, "CREATE (:T {id: 100, v: 0});");
    run(second, "CREATE (:T {id: 200, v: 0});");
    run(*conn, "COMMIT;");
    run(second, "COMMIT;");
}

TEST_F(LockTables, AnUnknownTableIsRefusedByName) {
    run(*conn, "BEGIN TRANSACTION;");
    const auto refused = outcome(*conn, "CALL lock_tables(['T', 'Chunk']);");
    EXPECT_NE(refused.find("Chunk"), std::string::npos)
        << "[check: an-unknown-table-is-named] " << refused;
    EXPECT_EQ(refused.find("Schema changed"), std::string::npos) << refused;
    run(*conn, "ROLLBACK;");
}

TEST_F(LockTables, InAutoCommitItIsRefused) {
    const auto refused = outcome(*conn, "CALL lock_tables(['T']);");
    EXPECT_NE(refused.find("explicit transaction"), std::string::npos)
        << "[check: lock-tables-needs-an-explicit-transaction] " << refused;
}

} // namespace
