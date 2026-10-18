// Un ROLLBACK sous un tampon plein (ticket
// 2026-10-18-un-rollback-sous-un-tampon-plein-bloque-la-base-et-la-fermeture-tue-le-processus).
// Trouvé par le scénario des vingt écrivains, point de reprise automatique coupé : « Connection
// already has an active transaction » au BEGIN qui suit un ROLLBACK, le ROLLBACK ayant été refusé
// par le tampon plein.
//
// La recette, déterministe sur un fil et deux connexions, avec le tampon des tests (64 Mo) et le
// point de reprise automatique coupé :
// - A : BEGIN, puis A_ROWS lignes acceptées (un bloc sain) ;
// - B, en automatique : des lots validés par pas qui se divisent par deux, à partir de 5 000,
// jusqu'à
//   vingt refus. Les pages validées et pas encore reportées restent tenues, et le tampon est plein
//   au plus juste ;
// - A : ROLLBACK, puis ROLLBACK de nouveau.
// Joué le 18 octobre sur luciepc (62083d8be, sonde) : les deux ROLLBACK refusés (« Unable to
// allocate memory »), BEGIN, CHECKPOINT et lecture refusés aussi ; puis, à la fermeture,
// ~Connection appelle rollback, qui lève depuis le destructeur : std::terminate, SIGABRT.
//
// Avec un premier pas de 20 000, le tampon est presque plein, pas tout à fait : le ROLLBACK passe,
// mais un CHECKPOINT est refusé faute de mémoire, et ce refus empoisonne la base (« A checkpoint of
// this database failed, so it must be closed and reopened ») alors que rien n'a été écrit.
//
// Chaque cas se joue dans un processus fils : un abandon y est un rouge nommé, pas la fin de la
// passe.
// - FullBufferRollback.RollsBackAndClosesCleanly (premier pas 5 000) :
//   - rollback-under-a-full-buffer-succeeds : l'un des deux ROLLBACK de A passe ;
//   - closing-a-connection-does-not-abort : le fils sort sans signal en fermant ses connexions.
// - FullBufferRollback.RefusedCheckpointLeavesTheDatabaseOpen (premier pas 20 000) : après le
//   CHECKPOINT refusé par son nom, un BEGIN de A passe
//   (refused-checkpoint-leaves-the-database-open).

#ifndef __SINGLE_THREADED__

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <utility>

#include "bench_harness.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr int64_t A_ROWS = 20'000;
constexpr int64_t B_REFUSALS = 20;

class FullBufferRollback : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
    }

    // Joue la recette dans un fils ; rend son journal et son statut. withCheckpoint : après le
    // ROLLBACK, B lance CHECKPOINT et A tente un BEGIN.
    std::pair<std::string, int> play(int64_t bFirstStep, bool withCheckpoint) {
        namespace fs = std::filesystem;
        fs::create_directories(fs::path(databasePath).parent_path());
        const auto stepsPath = databasePath + ".steps";
        fs::remove(stepsPath);
        const auto pid = fork();
        if (pid == 0) {
            concurrency::disableCoreDumps();
            std::ofstream steps(stepsPath);
            auto say = [&](rag3db::main::Connection& connection, const std::string& who,
                           const std::string& query) {
                auto result = connection.query(query);
                const auto ok = result->isSuccess();
                steps << who << " " << query.substr(0, 40) << " -> "
                      << (ok ? "ok" : "refus « " + result->getErrorMessage().substr(0, 160) + " »")
                      << std::endl;
                return ok;
            };
            std::string vector;
            for (auto d = 0; d < 64; ++d) {
                vector += (d ? "," : "") + std::to_string(0.01 * (d % 50));
            }
            auto insert = [&](rag3db::main::Connection& connection, int64_t from, int64_t count) {
                return connection
                    .query(stringFormat("UNWIND range({}, {}) AS i CREATE (:Doc {id: i, titre: "
                                        "'titre', vec: [{}]});",
                        from, from + count - 1, vector))
                    ->isSuccess();
            };
            {
                rag3db::main::Database database(databasePath, *systemConfig);
                rag3db::main::Connection a(&database);
                rag3db::main::Connection b(&database);
                concurrency::applyBenchSettings(a);
                say(a, "A",
                    "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, titre STRING, vec FLOAT[64]);");
                say(a, "A", "BEGIN TRANSACTION;");
                for (int64_t at = 0; at < A_ROWS; at += 500) {
                    if (!insert(a, at, 500)) {
                        steps << "PLAN a-refused-at " << at << std::endl;
                        _exit(3);
                    }
                }
                int64_t at = 10'000'000;
                int64_t refusals = 0;
                for (auto step = bFirstStep; step >= 50 && refusals < B_REFUSALS;) {
                    if (insert(b, at, step)) {
                        at += step;
                    } else {
                        refusals++;
                        step /= 2;
                    }
                }
                steps << "B " << at - 10'000'000 << " rows committed before the buffer was full"
                      << std::endl;
                const auto first = say(a, "A", "ROLLBACK;");
                const auto second = first || say(a, "A", "ROLLBACK;");
                steps << "ROLLBACK " << (second ? "passed" : "refused") << std::endl;
                if (withCheckpoint) {
                    steps << "CHECKPOINT " << (say(b, "B", "CHECKPOINT;") ? "passed" : "refused")
                          << std::endl;
                    steps << "BEGIN " << (say(a, "A", "BEGIN TRANSACTION;") ? "passed" : "refused")
                          << std::endl;
                    say(a, "A", "ROLLBACK;");
                }
                // La fermeture : les connexions, puis la base, à la sortie de ce bloc.
            }
            steps << "CLOSED" << std::endl;
            _exit(0);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        std::string steps;
        std::ifstream in(stepsPath);
        for (std::string line; std::getline(in, line);) {
            steps += line + "\n";
        }
        std::cerr << "  [rollback sous tampon plein] le fils (premier pas " << bFirstStep << ") :\n"
                  << steps;
        return {steps, status};
    }
};

TEST_F(FullBufferRollback, RollsBackAndClosesCleanly) {
    const auto [steps, status] = play(5'000, false /* withCheckpoint */);
    ASSERT_EQ(steps.find("PLAN a-refused-at"), std::string::npos)
        << "[check: plan] A's block did not fit in the buffer: nothing is witnessed";
    ASSERT_NE(steps.find("ROLLBACK "), std::string::npos)
        << "[check: plan] the child died before its ROLLBACK, status " << status;
    EXPECT_NE(steps.find("ROLLBACK passed"), std::string::npos)
        << "[check: rollback-under-a-full-buffer-succeeds] both ROLLBACKs were refused";
    EXPECT_FALSE(WIFSIGNALED(status))
        << "[check: closing-a-connection-does-not-abort] the child died of signal "
        << WTERMSIG(status) << " while closing its connections";
}

TEST_F(FullBufferRollback, RefusedCheckpointLeavesTheDatabaseOpen) {
    const auto [steps, status] = play(20'000, true /* withCheckpoint */);
    ASSERT_EQ(steps.find("PLAN a-refused-at"), std::string::npos)
        << "[check: plan] A's block did not fit in the buffer: nothing is witnessed";
    ASSERT_NE(steps.find("CHECKPOINT refused"), std::string::npos)
        << "[check: plan] the CHECKPOINT was not refused by the full buffer: nothing is witnessed";
    EXPECT_NE(steps.find("BEGIN passed"), std::string::npos)
        << "[check: refused-checkpoint-leaves-the-database-open] a BEGIN after the refused "
           "CHECKPOINT was refused";
}

} // namespace

#endif
