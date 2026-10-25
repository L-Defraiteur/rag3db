// V2-1c : un point de reprise ne fige plus les lecteurs nouveaux. Pendant qu'il s'écrit (hors du
// verrou public, les blocs neufs bâtis hors du verrou de chaque groupe et échangés sous lui), une
// lecture commence, lit juste (le dernier validé, qui est ce que le point de reprise écrit), et
// rend la main sans l'attendre ; un lecteur ouvert pendant l'écriture relit la même chose après
// elle. Ce qui ne change pas en V2-1 : un écrivain nouveau attend la fin de l'écriture (V2-2), et
// un lecteur ouvert AVANT le point de reprise est attendu par lui (le banc le couvre).
//
// Le point de reprise est ralenti par une doublure (SlowCheckpointer) qui dort après l'écriture
// des tables, c'est-à-dire après l'échange des blocs : c'est le moment où un lecteur peut entrer.
// Sur master, la lecture attend la fin de l'écriture (waitWhileCheckpointPendingNoLock) : rouge
// par la durée.
#include <atomic>
#include <chrono>
#include <string>
#include <thread>

#include "flaky_checkpointer.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/checkpointer.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::storage;
using namespace rag3db::testing;

namespace {

using Clock = std::chrono::steady_clock;
constexpr auto HOLD = std::chrono::milliseconds(1500);
// Ce qu'une lecture peut mettre sans attendre le point de reprise, sur un poste chargé.
constexpr double MAX_READ_MS = 500;

double msSince(Clock::time_point start) {
    return std::chrono::duration<double, std::milli>(Clock::now() - start).count();
}

// Dort après l'écriture des tables : le point de reprise dure au moins HOLD de plus.
class SlowCheckpointer final : public Checkpointer {
public:
    explicit SlowCheckpointer(ClientContext& context) : Checkpointer(context) {}

protected:
    bool checkpointStorage() override {
        const auto changed = Checkpointer::checkpointStorage();
        std::this_thread::sleep_for(HOLD);
        return changed;
    }
};

class CheckpointUnderReaders : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "a checkpoint is the subject: on-disk database needed";
        }
        createDBAndConn();
        run(*conn, "CALL auto_checkpoint=false;");
        run(*conn, "CREATE NODE TABLE T(id INT64 PRIMARY KEY, n INT64);");
        run(*conn, "UNWIND range(1, 20000) AS i CREATE (:T {id: i, n: i});");
        run(*conn, "CHECKPOINT;");
        // Ce que le point de reprise lent écrira, et ce qu'un lecteur doit voir pendant lui.
        run(*conn, "MATCH (t:T {id: 7}) SET t.n = -1;");
        FlakyCheckpointer([](ClientContext& context) -> std::unique_ptr<Checkpointer> {
            return std::make_unique<SlowCheckpointer>(context);
        }).setCheckpointer(*getClientContext(*conn));
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
    // Le point de reprise lent sur `conn`, dans un fil ; `started` est posé juste avant.
    std::thread slowCheckpoint(std::atomic<bool>& started, double& checkpointMs,
        std::string& checkpointError) {
        return std::thread([this, &started, &checkpointMs, &checkpointError] {
            const auto start = Clock::now();
            started = true;
            auto result = conn->query("CHECKPOINT;");
            checkpointMs = msSince(start);
            checkpointError = result->isSuccess() ? "" : result->getErrorMessage();
        });
    }
    // Attend que le point de reprise ait commencé, puis un tiers de HOLD : il est dans son
    // sommeil, les blocs neufs échangés.
    static void waitUntilInsideTheHold(const std::atomic<bool>& started) {
        while (!started) {
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
        std::this_thread::sleep_for(HOLD / 3);
    }
};

// Une lecture par clé et un balayage entier pendant l'écriture : justes, et sans attendre.
TEST_F(CheckpointUnderReaders, AReadDuringACheckpointDoesNotWaitAndReadsRight) {
    std::atomic<bool> started{false};
    double checkpointMs = 0;
    std::string checkpointError;
    auto checkpoint = slowCheckpoint(started, checkpointMs, checkpointError);
    waitUntilInsideTheHold(started);
    Connection reader(database.get());
    const auto start = Clock::now();
    const auto byKey = row(reader, "MATCH (t:T {id: 7}) RETURN t.n;");
    const auto byKeyMs = msSince(start);
    const auto whole = row(reader, "MATCH (t:T) RETURN count(*), sum(t.n);");
    const auto bothMs = msSince(start);
    checkpoint.join();
    EXPECT_EQ(checkpointError, "");
    ASSERT_GE(checkpointMs, HOLD.count()) << "the slow checkpointer was not in place";
    EXPECT_EQ(byKey, "-1\n");
    EXPECT_EQ(whole, "20000|200009992\n");
    EXPECT_LE(byKeyMs, MAX_READ_MS) << "the read by key waited for the checkpoint: " << byKeyMs
                                    << " ms during a " << checkpointMs << " ms checkpoint";
    EXPECT_LE(bothMs, MAX_READ_MS) << "the scan waited for the checkpoint: " << bothMs << " ms";
}

// Un lecteur ouvert pendant l'écriture lit la même chose avant et après la fin du point de
// reprise, dans la même transaction, et son BEGIN n'a pas attendu.
TEST_F(CheckpointUnderReaders, AReaderOpenedDuringACheckpointReadsTheSameAfterIt) {
    std::atomic<bool> started{false};
    double checkpointMs = 0;
    std::string checkpointError;
    auto checkpoint = slowCheckpoint(started, checkpointMs, checkpointError);
    waitUntilInsideTheHold(started);
    Connection reader(database.get());
    const auto start = Clock::now();
    const auto begin = row(reader, "BEGIN TRANSACTION READ ONLY;");
    const auto beginMs = msSince(start);
    const auto during = row(reader, "MATCH (t:T) RETURN count(*), sum(t.n), min(t.n);");
    checkpoint.join();
    EXPECT_EQ(checkpointError, "");
    ASSERT_GE(checkpointMs, HOLD.count()) << "the slow checkpointer was not in place";
    const auto after = row(reader, "MATCH (t:T) RETURN count(*), sum(t.n), min(t.n);");
    const auto commit = row(reader, "COMMIT;");
    EXPECT_EQ(begin, "AUCUNE LIGNE") << begin;
    EXPECT_EQ(commit, "AUCUNE LIGNE") << commit;
    EXPECT_EQ(during, "20000|200009992|-1\n");
    EXPECT_EQ(after, during);
    EXPECT_LE(beginMs, MAX_READ_MS) << "BEGIN TRANSACTION READ ONLY waited for the checkpoint: "
                                    << beginMs << " ms";
    // Et la base rouverte porte ce que le point de reprise a écrit.
    conn.reset();
    database.reset();
    createDBAndConn();
    EXPECT_EQ(row(*conn, "MATCH (t:T) RETURN count(*), sum(t.n), min(t.n);"),
        "20000|200009992|-1\n");
}

// Le périmètre de V2-1 : un écrivain nouveau attend encore la fin de l'écriture (ses blocs sont
// ceux que le point de reprise échange ; V2-2). Ce cas dit ce qui est voulu, pour qu'un lot qui
// le change le fasse en le sachant.
TEST_F(CheckpointUnderReaders, AWriterDuringACheckpointStillWaits) {
    std::atomic<bool> started{false};
    double checkpointMs = 0;
    std::string checkpointError;
    auto checkpoint = slowCheckpoint(started, checkpointMs, checkpointError);
    waitUntilInsideTheHold(started);
    Connection writer(database.get());
    const auto start = Clock::now();
    const auto begin = row(writer, "BEGIN TRANSACTION;");
    const auto beginMs = msSince(start);
    checkpoint.join();
    EXPECT_EQ(checkpointError, "");
    ASSERT_GE(checkpointMs, HOLD.count()) << "the slow checkpointer was not in place";
    EXPECT_EQ(begin, "AUCUNE LIGNE") << begin;
    EXPECT_GE(beginMs, HOLD.count() / 3) << "a writer entered during the checkpoint";
    EXPECT_EQ(row(writer, "ROLLBACK;"), "AUCUNE LIGNE");
}

} // namespace
