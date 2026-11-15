// Un ALTER attend les écrivains ouverts sur sa table (ticket
// docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-14-un-add-column-passe-sous-une-transaction-d-
// ecriture-ouverte.md ; conception docs/coeur-cpp/15-11-2026/01-rapport-de-session.md). Comme
// PostgreSQL (ACCESS EXCLUSIVE) : un écrivain tient sa table en partagé, de sa première instruction
// d'écriture jusqu'à sa fin ; un ALTER la prend en exclusif, et attend. L'écrivain ouvert ne voit donc
// jamais le schéma changer sous lui, et un écrivain qui arrive pendant l'ALTER attend sa fin.
#include <atomic>
#include <chrono>
#include <string>
#include <thread>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::main;

namespace rag3db {
namespace testing {

namespace {

using Clock = std::chrono::steady_clock;

double millisSince(Clock::time_point start) {
    return std::chrono::duration<double, std::milli>(Clock::now() - start).count();
}

std::string one(Connection& c, const std::string& q) {
    auto r = c.query(q);
    if (!r->isSuccess()) {
        return "REFUS " + r->getErrorMessage();
    }
    return r->hasNext() ? r->getNext()->toString() : "AUCUNE";
}

} // namespace

class AlterWaitsForWriters : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        run(*conn, "CREATE NODE TABLE T(id INT64, s STRING, c STRING, PRIMARY KEY(id));");
        run(*conn, "COPY T FROM (UNWIND range(0, 999) AS i RETURN i, 's' + CAST(i AS STRING), 'c');");
    }
    static void run(Connection& c, const std::string& q) {
        auto r = c.query(q);
        ASSERT_TRUE(r->isSuccess()) << q << " : " << r->getErrorMessage();
    }
    // A ouvre une écriture sur T ; B lance `ddl` dans un fil ; A valide après `holdMs`. Rend les
    // instants (depuis le début) où B a fini et où A a validé, et ce que B a rendu.
    struct Timeline {
        double ddlDone = -1;
        double commitDone = -1;
        std::string ddlResult;
    };
    Timeline openWriterThen(const std::string& ddl, const std::string& write, int holdMs) {
        Connection b(database.get());
        Timeline t;
        run(*conn, "BEGIN TRANSACTION;");
        run(*conn, write);
        const auto start = Clock::now();
        std::thread tb([&] {
            auto r = b.query(ddl);
            t.ddlResult = r->isSuccess() ? "OK" : "REFUS " + r->getErrorMessage();
            t.ddlDone = millisSince(start);
        });
        std::this_thread::sleep_for(std::chrono::milliseconds(holdMs));
        run(*conn, "COMMIT;");
        t.commitDone = millisSince(start);
        tb.join();
        return t;
    }
};

TEST_F(AlterWaitsForWriters, AnAddWaitsForTheOpenWriter) {
    const auto t = openWriterThen("ALTER TABLE T ADD note STRING DEFAULT 'n';",
        "MATCH (n:T) WHERE n.id < 500 SET n.s = 'réécrite';", 400);
    EXPECT_EQ(t.ddlResult, "OK");
    EXPECT_GT(t.ddlDone, t.commitDone)
        << "[check: an-add-waits-for-the-open-writer] the ADD ended at " << t.ddlDone
        << " ms, the writer committed at " << t.commitDone << " ms";
    EXPECT_EQ(one(*conn, "MATCH (n:T) WHERE n.s = 'réécrite' AND n.note = 'n' RETURN count(*);"),
        "500");
}

TEST_F(AlterWaitsForWriters, ADropOfAColumnTheOpenWriterWroteWaits) {
    const auto t = openWriterThen("ALTER TABLE T DROP c;",
        "MATCH (n:T) WHERE n.id < 300 SET n.c = 'écrite';", 400);
    EXPECT_EQ(t.ddlResult, "OK");
    EXPECT_GT(t.ddlDone, t.commitDone)
        << "[check: a-drop-of-a-written-column-waits] the DROP ended at " << t.ddlDone
        << " ms, the writer committed at " << t.commitDone << " ms";
    EXPECT_EQ(one(*conn, "MATCH (n:T) RETURN count(*);"), "1000");
}

TEST_F(AlterWaitsForWriters, AnAddWithAnOpenWriterKeepsTheDataAcrossACheckpointAndAReopening) {
    run(*conn, "CALL checkpoint_threshold=4096;");
    const auto t = openWriterThen("ALTER TABLE T ADD note STRING DEFAULT 'n';",
        "UNWIND range(1000, 1499) AS i CREATE (:T {id: i, s: 'neuve', c: 'c'});", 300);
    EXPECT_EQ(t.ddlResult, "OK");
    EXPECT_GT(t.ddlDone, t.commitDone) << "[check: an-add-waits-for-the-open-writer]";
    run(*conn, "CHECKPOINT;");
    createDBAndConn();
    EXPECT_EQ(one(*conn, "MATCH (n:T) WHERE n.s = 'neuve' AND n.note = 'n' RETURN count(*);"),
        "500")
        << "[check: an-add-with-an-open-writer-keeps-the-data]";
    EXPECT_EQ(one(*conn, "MATCH (n:T) RETURN count(*);"), "1500");
}

TEST_F(AlterWaitsForWriters, AWriterArrivingDuringTheAlterWaitsForIt) {
    Connection b(database.get());
    Connection c(database.get());
    run(*conn, "BEGIN TRANSACTION;");
    run(*conn, "MATCH (n:T) WHERE n.id = 1 SET n.s = 'a';");
    const auto start = Clock::now();
    double ddlDone = -1, cDone = -1;
    std::thread tb([&] {
        b.query("ALTER TABLE T ADD note STRING DEFAULT 'n';");
        ddlDone = millisSince(start);
    });
    // L'ALTER est en file derrière A ; C arrive ensuite.
    std::this_thread::sleep_for(std::chrono::milliseconds(150));
    std::thread tc([&] {
        c.query("MATCH (n:T) WHERE n.id = 2 SET n.s = 'c';");
        cDone = millisSince(start);
    });
    std::this_thread::sleep_for(std::chrono::milliseconds(300));
    run(*conn, "COMMIT;");
    const auto commitDone = millisSince(start);
    tb.join();
    tc.join();
    EXPECT_GT(ddlDone, commitDone) << "[check: an-add-waits-for-the-open-writer]";
    EXPECT_GE(cDone, ddlDone)
        << "[check: a-writer-arriving-during-the-alter-waits-for-it] C ended at " << cDone
        << " ms, the ALTER at " << ddlDone << " ms";
    EXPECT_EQ(one(*conn, "MATCH (n:T) WHERE n.id = 2 RETURN n.s, n.note;"), "c|n");
}

TEST_F(AlterWaitsForWriters, AnAlterThatWaitsTooLongSaysWhichWriterItWaitedFor) {
    Connection b(database.get());
    run(b, "CALL lock_timeout=300;");
    run(*conn, "BEGIN TRANSACTION;");
    run(*conn, "MATCH (n:T) WHERE n.id = 1 SET n.s = 'a';");
    auto r = b.query("ALTER TABLE T ADD note STRING DEFAULT 'n';");
    run(*conn, "COMMIT;");
    ASSERT_FALSE(r->isSuccess()) << "[check: an-alter-that-waits-too-long-is-refused]";
    const auto message = r->getErrorMessage();
    EXPECT_NE(message.find("lock timeout"), std::string::npos) << message;
    EXPECT_NE(message.find("waited for writer"), std::string::npos)
        << "[check: the-refusal-names-the-writer-it-waited-for] " << message;
}

} // namespace testing
} // namespace rag3db
