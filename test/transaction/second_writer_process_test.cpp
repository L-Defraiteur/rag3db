// B2 : le second processus écrivain attend au fichier au lieu d'être refusé (page coeur-cpp/11,
// §2, la voie Neo4j : le relais, pas la simultanéité). Les témoins du banc, écrits avant le code.
//
// Deux processus, chacun par fork, ouvrent la même base en écriture. Aujourd'hui, le verrou du
// fichier (fcntl F_SETLK) refuse le second tout de suite : « Could not set lock on file ». Avec B2,
// le second attend que le premier ferme, pendant une borne qu'il choisit
// (SystemConfig::fileLockWaitMs), puis ouvre, rejoue le journal, écrit. Au-delà de la borne, il
// échoue par une erreur nommée.
//
// Ces témoins compilent sur master : la borne n'est posée que si le champ existe. Sur master, ils
// sont donc rouges, puisque le second est refusé sans attendre.
//
// - TheSecondWriterProcessWaitsThenWrites : le premier tient la base 1,5 s puis ferme ; le second,
//   borne 10 s, attend, puis écrit, et les lignes des deux sont là.
// - TheSecondWriterProcessGivesUpAfterTheBoundWithANamedError : le premier tient 3 s ; le second,
//   borne 300 ms, échoue après au moins 300 ms, sur le fragment nommé.
// - AfterTheFirstDiesTheSecondOpensOnItsReplay : le premier valide une ligne puis meurt base
//   ouverte (SIGKILL) ; le second, qui attendait, ouvre sur le rejeu et voit la ligne avant d'écrire.

#include <fcntl.h>
#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <chrono>
#include <filesystem>
#include <fstream>
#include <memory>
#include <sstream>
#include <string>
#include <thread>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

// Le fragment de l'erreur au-delà de la borne, convenu avec le cœur C++ (11 octobre 2026) :
// « waited <N> ms for the database file held by another writer process », N étant la borne.
constexpr auto WAIT_EXCEEDED_FRAGMENT = "for the database file held by another writer process";

// La borne d'attente, seulement si le moteur la connaît : sur master le champ n'existe pas, et le
// témoin compile quand même.
template<typename Config>
void setFileLockWait(Config& config, uint64_t milliseconds) {
    if constexpr (requires { config.fileLockWaitMs = milliseconds; }) {
        config.fileLockWaitMs = milliseconds;
    }
}

int64_t countItems(Connection& connection) {
    auto result = connection.query("MATCH (n:Item) RETURN count(n);");
    if (!result->isSuccess() || !result->hasNext()) {
        return -1;
    }
    return result->getNext()->getValue(0)->getValue<int64_t>();
}

class SecondWriterProcess : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "needs an on-disk database: two processes share nothing in memory";
        }
        createDBAndConn();
        ok("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
        // Le parent ferme : seuls les deux enfants tiennent la base.
        conn.reset();
        database.reset();
        ASSERT_EQ(pipe(openedPipe), 0);
    }

    void TearDown() override {
        close(openedPipe[0]);
        close(openedPipe[1]);
        EmptyDBTest::TearDown();
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    std::string reportPath(const std::string& who) const { return databasePath + "." + who + ".report"; }

    // Le premier écrivain : ouvre, valide la ligne 1, le dit par le tuyau, tient la base holdMs,
    // puis valide la ligne 2 et ferme ; ou meurt base ouverte si dieWhileHolding.
    pid_t startFirstWriter(int64_t holdMs, bool dieWhileHolding) {
        const auto pid = fork();
        if (pid != 0) {
            return pid;
        }
        int code = 0;
        try {
            auto config = *systemConfig;
            config.autoCheckpoint = false;
            Database first(databasePath, config);
            Connection connection(&first);
            connection.query("CALL force_checkpoint_on_close=false;");
            if (!connection.query("CREATE (:Item {id: 1, writer: 1});")->isSuccess()) {
                _exit(10);
            }
            [[maybe_unused]] auto written = write(openedPipe[1], "o", 1);
            std::this_thread::sleep_for(std::chrono::milliseconds(holdMs));
            if (dieWhileHolding) {
                kill(getpid(), SIGKILL);
            }
            if (!connection.query("CREATE (:Item {id: 2, writer: 1});")->isSuccess()) {
                code = 11;
            }
        } catch (...) {
            code = 12;
        }
        _exit(code);
    }

    // Le second écrivain : attend que le premier ait ouvert, puis ouvre avec la borne waitMs. Il
    // écrit dans son rapport : « opened <ms> <lignes vues à l'ouverture> » ou « failed <ms> <message> ».
    pid_t startSecondWriter(uint64_t waitMs) {
        const auto pid = fork();
        if (pid != 0) {
            return pid;
        }
        char byte = 0;
        if (read(openedPipe[0], &byte, 1) != 1) {
            _exit(20);
        }
        std::ofstream report(reportPath("second"));
        const auto start = std::chrono::steady_clock::now();
        const auto elapsed = [&] {
            return std::chrono::duration_cast<std::chrono::milliseconds>(
                std::chrono::steady_clock::now() - start)
                .count();
        };
        try {
            auto config = *systemConfig;
            setFileLockWait(config, waitMs);
            Database second(databasePath, config);
            Connection connection(&second);
            const auto seen = countItems(connection);
            report << "opened " << elapsed() << " " << seen << "\n";
            if (!connection.query("CREATE (:Item {id: 3, writer: 2});")->isSuccess()) {
                report << "write-failed\n";
            }
        } catch (const std::exception& e) {
            report << "failed " << elapsed() << " " << e.what() << "\n";
        }
        report.close();
        _exit(0);
    }

    static int waitFor(pid_t pid) {
        int status = 0;
        waitpid(pid, &status, 0);
        return status;
    }

    std::string readReport(const std::string& who) const {
        std::ifstream in(reportPath(who));
        std::stringstream text;
        text << in.rdbuf();
        return text.str();
    }

    int64_t itemsAfterBoth() {
        createDBAndConn();
        return countItems(*conn);
    }

    int openedPipe[2] = {-1, -1};
};

TEST_F(SecondWriterProcess, TheSecondWriterProcessWaitsThenWrites) {
    const auto first = startFirstWriter(1500, false);
    const auto second = startSecondWriter(10000);
    waitFor(first);
    waitFor(second);
    const auto report = readReport("second");
    std::istringstream in(report);
    std::string state;
    int64_t elapsedMs = -1;
    int64_t seen = -1;
    in >> state >> elapsedMs >> seen;
    EXPECT_EQ(state, "opened") << "[check: second-writer-opens] " << report;
    EXPECT_GE(elapsedMs, 1000) << "[check: second-writer-waited] " << report;
    EXPECT_EQ(itemsAfterBoth(), 3) << "[check: both-writers-rows] " << report;
}

TEST_F(SecondWriterProcess, TheSecondWriterProcessGivesUpAfterTheBoundWithANamedError) {
    const auto first = startFirstWriter(3000, false);
    const auto second = startSecondWriter(300);
    waitFor(second);
    waitFor(first);
    const auto report = readReport("second");
    std::istringstream in(report);
    std::string state;
    int64_t elapsedMs = -1;
    in >> state >> elapsedMs;
    EXPECT_EQ(state, "failed") << "[check: second-writer-gives-up] " << report;
    EXPECT_GE(elapsedMs, 300) << "[check: second-writer-waited-the-bound] " << report;
    EXPECT_LT(elapsedMs, 2500) << "[check: second-writer-bounded] " << report;
    EXPECT_NE(report.find(WAIT_EXCEEDED_FRAGMENT), std::string::npos)
        << "[check: second-writer-named-error] " << report;
    EXPECT_EQ(itemsAfterBoth(), 2) << "[check: first-writer-rows] " << report;
}

TEST_F(SecondWriterProcess, AfterTheFirstDiesTheSecondOpensOnItsReplay) {
    const auto first = startFirstWriter(500, true);
    const auto second = startSecondWriter(10000);
    const auto firstStatus = waitFor(first);
    waitFor(second);
    EXPECT_TRUE(WIFSIGNALED(firstStatus) && WTERMSIG(firstStatus) == SIGKILL)
        << "[check: first-writer-died] status " << firstStatus;
    const auto report = readReport("second");
    std::istringstream in(report);
    std::string state;
    int64_t elapsedMs = -1;
    int64_t seen = -1;
    in >> state >> elapsedMs >> seen;
    EXPECT_EQ(state, "opened") << "[check: second-writer-opens] " << report;
    EXPECT_EQ(seen, 1) << "[check: second-writer-sees-the-replay] " << report;
    EXPECT_EQ(itemsAfterBoth(), 2) << "[check: replayed-and-written-rows] " << report;
}

} // namespace
