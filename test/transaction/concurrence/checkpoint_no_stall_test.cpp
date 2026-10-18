// Le point de reprise qui ne fige personne (ticket
// 2026-10-18-un-point-de-reprise-fige-toutes-les-transactions-le-temps-de-s-ecrire, voie 2, la
// forme de PostgreSQL) : un point de reprise n'arrête ni les BEGIN ni les lectures, et le tampon
// écrit sur disque des pages validées et pas encore reportées quand il en manque (l'écriture à la
// demande, « steal »), le journal permettant de les rejouer. Aujourd'hui, un point de reprise
// retient tous les BEGIN, lecteurs compris, et le tampon ne rend pas une page validée tant qu'elle
// n'est pas reportée.
//
// - CheckpointNoStall.BeginDuringACheckpointDoesNotWait : une table de vecteurs réécrits en entier
//   (des milliers de pages à reporter) ; une connexion lance CHECKPOINT, une autre ouvre une
//   transaction 30 ms plus tard. begin-during-a-checkpoint-does-not-wait : le BEGIN rend la main en
//   moins de 100 ms, quelle que soit la durée du point de reprise.
// - CheckpointNoStall.ReadDuringACheckpointDoesNotWait : la même chose pour une lecture par clé en
//   automatique (read-during-a-checkpoint-does-not-wait).
// - CheckpointNoStall.CommittedPagesAreWrittenOutWhenTheBufferIsFull : le tampon des tests (64 Mo),
//   le point de reprise automatique coupé ; un écrivain valide des lots de vecteurs jusqu'à TARGET
//   lignes, plus que le tampon n'en tient. committed-pages-are-written-out-when-the-buffer-is-full
//   : aucun lot n'est refusé par le tampon, et chaque ligne validée se relit. C'est le tableau des
//   seuils (ticket du point de reprise, « La première voie, mesurée ») retourné en témoin ; le
//   scénario des vingt écrivains le joue à plusieurs par SCENARIO_NO_AUTO_CHECKPOINT=1.
// - CheckpointNoStall.CrashAfterWrittenOutPagesReplaysExactly : la même écriture dans un processus
//   fils, tué par SIGKILL base ouverte, après TARGET lignes validées ; le père rouvre.
//   crash-after-written-out-pages-replays-exactly : le fils a validé TARGET lignes (des pages ont
//   donc été rendues), et la base rouverte en porte exactement le nombre que son journal dit
//   validé.

#ifndef __SINGLE_THREADED__

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <chrono>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <random>
#include <string>
#include <thread>

#include "bench_harness.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

using Clock = std::chrono::steady_clock;
constexpr int DIM = 64;
// Les lignes du point de reprise : COPY, puis chaque vecteur réécrit, pour des milliers de pages à
// reporter.
constexpr int64_t CHECKPOINT_ROWS = 100'000;
constexpr double MAX_WAIT_MS = 100;
constexpr double MIN_CHECKPOINT_MS = 500;
// Le tampon des tests (64 Mo) tient environ 170 000 lignes de ce format (FullBufferRollback, 18
// octobre) ; TARGET en demande plus du double.
constexpr int64_t TARGET = 400'000;
constexpr int64_t BATCH = 5'000;

double msSince(Clock::time_point start) {
    return std::chrono::duration<double, std::milli>(Clock::now() - start).count();
}

std::string vectorLiteral(int64_t seed) {
    std::string out = "[";
    for (auto d = 0; d < DIM; ++d) {
        out += (d ? "," : "") + std::to_string(0.001 * static_cast<double>((seed * 31 + d) % 997));
    }
    return out + "]";
}

class CheckpointNoStall : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
    }

    static void ok(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess())
            << "[check: query] " << query.substr(0, 160) << " : " << result->getErrorMessage();
    }

    // Une base de CHECKPOINT_ROWS lignes dont chaque vecteur a été réécrit après le dernier point
    // de reprise : le prochain CHECKPOINT a des milliers de pages à reporter.
    void prepareACostlyCheckpoint() {
        systemConfig->bufferPoolSize = 2ull * 1024 * 1024 * 1024;
        createDBAndConn();
        concurrency::applyBenchSettings(*conn);
        ok(*conn, stringFormat("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[{}]);", DIM));
        const auto path = databasePath + ".docs.csv";
        {
            std::ofstream out(path);
            for (int64_t id = 0; id < CHECKPOINT_ROWS; ++id) {
                out << id << ",\"" << vectorLiteral(id) << "\"\n";
            }
        }
        ok(*conn, "COPY Doc FROM '" + path + "' (header=false);");
        std::filesystem::remove(path);
        ok(*conn, "CHECKPOINT;");
        ok(*conn, stringFormat("MATCH (d:Doc) SET d.vec = {};", vectorLiteral(7)));
    }

    // CHECKPOINT sur la connexion principale, et other 30 ms après son départ ; rend (durée du
    // point de reprise, durée de other, erreur de other).
    struct Overlap {
        double checkpointMs;
        double otherMs;
        std::string otherError;
    };
    Overlap duringACheckpoint(rag3db::main::Connection& other, const std::string& otherQuery) {
        Overlap out{};
        const auto start = Clock::now();
        std::thread checkpoint([&] {
            auto result = conn->query("CHECKPOINT;");
            out.checkpointMs = msSince(start);
            if (!result->isSuccess()) {
                out.checkpointMs = -1;
            }
        });
        std::this_thread::sleep_for(std::chrono::milliseconds(30));
        const auto otherStart = Clock::now();
        auto result = other.query(otherQuery);
        out.otherMs = msSince(otherStart);
        out.otherError = result->isSuccess() ? "" : result->getErrorMessage();
        checkpoint.join();
        return out;
    }

    // Des lots de BATCH lignes validés jusqu'à TARGET ; rend le nombre de lignes validées et le
    // premier refus. log, s'il est donné, reçoit le compte après chaque validation.
    static std::pair<int64_t, std::string> fill(rag3db::main::Connection& connection,
        std::FILE* log = nullptr) {
        int64_t committed = 0;
        while (committed < TARGET) {
            auto result =
                connection.query(stringFormat("UNWIND range({}, {}) AS i CREATE (:Doc {id: "
                                              "i, vec: {}});",
                    committed, committed + BATCH - 1, vectorLiteral(committed)));
            if (!result->isSuccess()) {
                return {committed, result->getErrorMessage()};
            }
            committed += BATCH;
            if (log) {
                std::fprintf(log, "%ld\n", static_cast<long>(committed));
                std::fflush(log);
            }
        }
        return {committed, ""};
    }
};

TEST_F(CheckpointNoStall, BeginDuringACheckpointDoesNotWait) {
    prepareACostlyCheckpoint();
    rag3db::main::Connection other(database.get());
    const auto r = duringACheckpoint(other, "BEGIN TRANSACTION;");
    other.query("ROLLBACK;");
    std::cerr << "  [point de reprise] CHECKPOINT " << r.checkpointMs
              << " ms ; BEGIN pendant : " << r.otherMs << " ms "
              << (r.otherError.empty() ? "" : "(" + r.otherError + ")") << "\n";
    ASSERT_GE(r.checkpointMs, MIN_CHECKPOINT_MS)
        << "[check: plan] the checkpoint took " << r.checkpointMs << " ms: nothing is witnessed";
    EXPECT_TRUE(r.otherError.empty() && r.otherMs <= MAX_WAIT_MS)
        << "[check: begin-during-a-checkpoint-does-not-wait] BEGIN took " << r.otherMs
        << " ms during a " << r.checkpointMs << " ms checkpoint " << r.otherError;
}

TEST_F(CheckpointNoStall, ReadDuringACheckpointDoesNotWait) {
    prepareACostlyCheckpoint();
    rag3db::main::Connection other(database.get());
    const auto r = duringACheckpoint(other, "MATCH (d:Doc {id: 4242}) RETURN d.id;");
    std::cerr << "  [point de reprise] CHECKPOINT " << r.checkpointMs
              << " ms ; lecture pendant : " << r.otherMs << " ms "
              << (r.otherError.empty() ? "" : "(" + r.otherError + ")") << "\n";
    ASSERT_GE(r.checkpointMs, MIN_CHECKPOINT_MS)
        << "[check: plan] the checkpoint took " << r.checkpointMs << " ms: nothing is witnessed";
    EXPECT_TRUE(r.otherError.empty() && r.otherMs <= MAX_WAIT_MS)
        << "[check: read-during-a-checkpoint-does-not-wait] the read took " << r.otherMs
        << " ms during a " << r.checkpointMs << " ms checkpoint " << r.otherError;
}

TEST_F(CheckpointNoStall, CommittedPagesAreWrittenOutWhenTheBufferIsFull) {
    createDBAndConn();
    concurrency::applyBenchSettings(*conn);
    ok(*conn, stringFormat("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[{}]);", DIM));
    const auto [committed, refusal] = fill(*conn);
    auto count = conn->query("MATCH (d:Doc) RETURN count(d);");
    const auto read = count->isSuccess() && count->hasNext() ?
                          count->getNext()->getValue(0)->getValue<int64_t>() :
                          -1;
    std::cerr << "  [point de reprise] tampon de 64 Mo, sans point de reprise : " << committed
              << " lignes validées sur " << TARGET << ", relues " << read
              << (refusal.empty() ? "" : ", refus : " + refusal.substr(0, 120)) << "\n";
    EXPECT_TRUE(refusal.empty() && committed == TARGET && read == TARGET)
        << "[check: committed-pages-are-written-out-when-the-buffer-is-full] " << committed
        << " rows committed of " << TARGET << ", " << read << " read back ; " << refusal;
}

TEST_F(CheckpointNoStall, CrashAfterWrittenOutPagesReplaysExactly) {
    namespace fs = std::filesystem;
    fs::create_directories(fs::path(databasePath).parent_path());
    const auto logPath = databasePath + ".committed";
    fs::remove(logPath);
    const auto pid = fork();
    if (pid == 0) {
        concurrency::disableCoreDumps();
        rag3db::main::Database child(databasePath, *systemConfig);
        rag3db::main::Connection connection(&child);
        concurrency::applyBenchSettings(connection);
        connection.query(
            stringFormat("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[{}]);", DIM));
        auto* log = std::fopen(logPath.c_str(), "w");
        fill(connection, log);
        // Mourir base ouverte : sortir de la portée fermerait la base proprement.
        kill(getpid(), SIGKILL);
        _exit(9);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
        << "[check: plan] the child was not killed, status " << status;
    int64_t logged = 0;
    {
        std::ifstream in(logPath);
        for (int64_t value; in >> value;) {
            logged = value;
        }
    }
    createDBAndConn();
    concurrency::applyBenchSettings(*conn);
    auto count = conn->query("MATCH (d:Doc) RETURN count(d);");
    ASSERT_TRUE(count->isSuccess()) << "[check: query] " << count->getErrorMessage();
    const auto reopened = count->getNext()->getValue(0)->getValue<int64_t>();
    std::cerr << "  [point de reprise] arrêt brutal : le fils a validé " << logged << " lignes sur "
              << TARGET << ", la base rouverte en porte " << reopened << "\n";
    EXPECT_TRUE(logged == TARGET && reopened == logged)
        << "[check: crash-after-written-out-pages-replays-exactly] the child committed " << logged
        << " rows of " << TARGET << " (fewer means the buffer refused instead of writing pages "
        << "out), the reopened database holds " << reopened;
}

} // namespace

#endif
