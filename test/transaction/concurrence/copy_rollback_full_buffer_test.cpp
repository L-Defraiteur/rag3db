// L'annulation d'un COPY sous un tampon plein (ticket
// 2026-10-18-l-annulation-d-un-copy-sous-un-tampon-plein-peut-encore-allouer).
//
// La recette de FullBufferRollback.RollsBackAndClosesCleanly (full_buffer_rollback_test.cpp), avec
// un COPY de la connexion A à la place de ses CREATE : un COPY écrit ses lignes directement dans
// les groupes validés, et son annulation passe par les enregistrements INSERT_INFO
// (NodeTableVersionRecordHandler::rollbackInsert → NodeTable::rollbackPKIndexInsert), qui relit les
// clés des lignes annulées et les retire de l'index de clé primaire. Sous un tampon plein de pages
// validées non reportées (B), cette relecture ou ce retrait peuvent demander un cadre : le ROLLBACK
// lèverait au milieu du stockage partagé, la transaction retirée et la base condamnée.
//
// Le cas se joue dans un processus fils (un abandon y est un rouge nommé) :
// - rollback-of-a-copy-under-a-full-buffer-succeeds : l'un des deux ROLLBACK de A passe ;
// - database-stays-open-after-the-copy-rollback : un BEGIN de A passe ensuite, et la table compte
//   les seules lignes de B ;
// - closing-a-connection-does-not-abort : le fils sort sans signal.

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

class CopyRollbackUnderAFullBuffer : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
    }

    static std::string vectorText() {
        std::string vector;
        for (auto d = 0; d < 64; ++d) {
            vector += (d ? "," : "") + std::to_string(0.01 * (d % 50));
        }
        return vector;
    }

    // Les lignes du COPY de A, écrites avant le fils.
    std::string writeCopyFile() const {
        const auto path = databasePath + ".a.csv";
        std::ofstream out(path);
        const auto vector = vectorText();
        for (int64_t i = 0; i < A_ROWS; i++) {
            out << i << ",titre,\"[" << vector << "]\"\n";
        }
        return path;
    }

    std::pair<std::string, int> play(int64_t bFirstStep) {
        namespace fs = std::filesystem;
        fs::create_directories(fs::path(databasePath).parent_path());
        const auto copyFile = writeCopyFile();
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
            const auto vector = vectorText();
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
                if (!say(a, "A", "COPY Doc FROM '" + copyFile + "' (header=false);")) {
                    steps << "PLAN a-copy-refused" << std::endl;
                    _exit(3);
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
                const auto bRows = at - 10'000'000;
                steps << "B " << bRows << " rows committed before the buffer was full" << std::endl;
                const auto first = say(a, "A", "ROLLBACK;");
                const auto second = first || say(a, "A", "ROLLBACK;");
                steps << "ROLLBACK " << (second ? "passed" : "refused") << std::endl;
                const auto begun = say(a, "A", "BEGIN TRANSACTION;");
                steps << "BEGIN " << (begun ? "passed" : "refused") << std::endl;
                auto counted = a.query("MATCH (d:Doc) RETURN count(*);");
                if (counted->isSuccess() && counted->hasNext()) {
                    const auto rows = counted->getNext()->getValue(0)->getValue<int64_t>();
                    steps << "COUNT " << (rows == bRows ? "only-b" : "wrong") << " " << rows
                          << " for " << bRows << std::endl;
                } else {
                    steps << "COUNT refused" << std::endl;
                }
                say(a, "A", "ROLLBACK;");
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
        std::cerr << "  [annulation d'un COPY sous tampon plein] le fils (premier pas "
                  << bFirstStep << ") :\n"
                  << steps;
        return {steps, status};
    }
};

TEST_F(CopyRollbackUnderAFullBuffer, RollsBackAndKeepsTheDatabaseOpen) {
    const auto [steps, status] = play(5'000);
    ASSERT_EQ(steps.find("PLAN a-copy-refused"), std::string::npos)
        << "[check: plan] A's COPY did not fit in the buffer: nothing is witnessed";
    ASSERT_NE(steps.find("ROLLBACK "), std::string::npos)
        << "[check: plan] the child died before its ROLLBACK, status " << status;
    EXPECT_NE(steps.find("ROLLBACK passed"), std::string::npos)
        << "[check: rollback-of-a-copy-under-a-full-buffer-succeeds] both ROLLBACKs were refused";
    EXPECT_NE(steps.find("BEGIN passed"), std::string::npos)
        << "[check: database-stays-open-after-the-copy-rollback] a BEGIN after the ROLLBACK was "
           "refused";
    EXPECT_NE(steps.find("COUNT only-b"), std::string::npos)
        << "[check: database-stays-open-after-the-copy-rollback] the table does not count B's "
           "rows alone";
    EXPECT_FALSE(WIFSIGNALED(status))
        << "[check: closing-a-connection-does-not-abort] the child died of signal "
        << WTERMSIG(status);
}

} // namespace

#endif
