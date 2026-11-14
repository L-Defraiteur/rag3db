// L'annulation d'un COMMIT refusé ne défait que ses propres lignes (ticket
// docs/coeur-cpp/tickets/2026-11-14-sous-vingt-ecrivains-et-un-pool-plein-des-note-validees-
// manquent.md). Sous TwentyWriters.DdlEveryMinute et un pool de tampons plein, des Note validées
// manquent au compte. La piste : un COMMIT qui échoue après avoir versé ses lignes dans les groupes
// de nœuds reste actif. L'appelant l'annule plus tard, hors du verrou du gestionnaire, et
// NodeGroup::rollbackInsert ramène le groupe à la première ligne annulée : les lignes qu'un autre
// écrivain a validées entre-temps, à la suite, partiraient avec.
//
// Le refus se joue sans pool plein : le journal ne se synchronise pas une fois (CutFileSystem,
// failSyncOf), comme sur un disque plein. Le COMMIT de A est refusé après le versement de ses
// lignes ; B valide une ligne ; A fait ROLLBACK. Ce que B a validé doit rester.
//
// Les deux premiers cas sont séquentiels : l'annulation du COMMIT refusé se fait dans le même appel
// (runFuncInTransaction), et B n'y passe pas. Le troisième ouvre la fenêtre réelle : B lance son
// COMMIT pendant celui de A, lent (un gros lot), et attend le verrou du gestionnaire ; à l'échec de
// A, le verrou est rendu avant que l'annulation de A le reprenne, et B peut valider entre les deux.
#include <sys/wait.h>
#include <unistd.h>

#include <chrono>
#include <filesystem>
#include <fstream>
#include <memory>
#include <string>
#include <thread>

#include "common/file_system/virtual_file_system.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "test_helper/cut_file_system.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class RefusedCommitRollback : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        std::filesystem::create_directories(std::filesystem::path(databasePath).parent_path());
    }

    // Joue le cas dans un fils sur CutFileSystem ; rend ses pas, une ligne chacun.
    std::string play(bool blockForB) {
        const auto stepsPath = databasePath + ".steps";
        std::filesystem::remove(stepsPath);
        const auto pid = fork();
        if (pid == 0) {
            VirtualFileSystem::setDefaultFileSystemFactoryForTesting(
                [](const std::string& home) { return std::make_unique<CutFileSystem>(home); });
            std::ofstream steps(stepsPath);
            {
                Database database(databasePath, *systemConfig);
                Connection a(&database);
                Connection b(&database);
                auto say = [&](const std::string& what, Connection& c, const std::string& q) {
                    auto r = c.query(q);
                    steps << what << " "
                          << (r->isSuccess() ? "accepted" :
                                               "refused « " + r->getErrorMessage().substr(0, 160) +
                                                   " »")
                          << std::endl;
                    return r->isSuccess();
                };
                auto count = [&](Connection& c, const std::string& q) {
                    auto r = c.query(q);
                    return r->isSuccess() && r->hasNext() ? r->getNext()->toString() :
                                                            std::string("refus\n");
                };
                if (!say("SCHEMA", a, "CREATE NODE TABLE Note(id INT64 PRIMARY KEY, s STRING);") ||
                    !say("FILL", a,
                        "UNWIND range(0, 9) AS i CREATE (:Note {id: i, s: 'validée'});")) {
                    _exit(11);
                }
                say("A-BEGIN", a, "BEGIN TRANSACTION;");
                say("A-CREATE", a, "UNWIND range(100, 104) AS i CREATE (:Note {id: i, s: 'a'});");
                // Le disque est plein le temps d'une synchronisation du journal.
                auto failures = std::make_shared<int>(1);
                CutFileSystem::failSyncOf() = [failures](const std::string& path) {
                    return path.ends_with(".wal") && (*failures)-- > 0;
                };
                say("A-COMMIT", a, "COMMIT;");
                CutFileSystem::failSyncOf() = nullptr;
                if (blockForB) {
                    say("B-BEGIN", b, "BEGIN TRANSACTION;");
                    say("B-CREATE", b, "CREATE (:Note {id: 200, s: 'b'});");
                    say("B-COMMIT", b, "COMMIT;");
                } else {
                    say("B-CREATE", b, "CREATE (:Note {id: 200, s: 'b'});");
                }
                say("A-ROLLBACK", a, "ROLLBACK;");
                steps << "COUNT " << count(b, "MATCH (n:Note) RETURN count(n);");
                steps << "B-ROW " << count(b, "MATCH (n:Note {id: 200}) RETURN count(n);");
                steps << "B-SCAN " << count(b, "MATCH (n:Note) WHERE n.s = 'b' RETURN count(n);");
                steps << "A-ROWS " << count(b, "MATCH (n:Note) WHERE n.s = 'a' RETURN count(n);");
                steps.flush();
            }
            _exit(0);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        std::string steps;
        std::ifstream in(stepsPath);
        for (std::string line; std::getline(in, line);) {
            steps += line + "\n";
        }
        EXPECT_TRUE(WIFEXITED(status) && WEXITSTATUS(status) == 0)
            << "[check: setup] the child ended with status " << status << "\n"
            << steps;
        return steps;
    }

    static void expectBKept(const std::string& steps) {
        ASSERT_NE(steps.find("A-COMMIT refused"), std::string::npos)
            << "[check: setup] the failed sync did not refuse A's commit\n"
            << steps;
        ASSERT_NE(steps.find("B-CREATE accepted"), std::string::npos)
            << "[check: setup] B could not write after A's refused commit\n"
            << steps;
        EXPECT_NE(steps.find("B-ROW 1\n"), std::string::npos)
            << "[check: a-rollback-keeps-the-rows-of-another-writer] by key\n"
            << steps;
        EXPECT_NE(steps.find("B-SCAN 1\n"), std::string::npos)
            << "[check: a-rollback-keeps-the-rows-of-another-writer] by scan\n"
            << steps;
        EXPECT_NE(steps.find("COUNT 11\n"), std::string::npos)
            << "[check: a-rollback-keeps-the-rows-of-another-writer] 10 rows, then B's\n"
            << steps;
        EXPECT_NE(steps.find("A-ROWS 0\n"), std::string::npos)
            << "[check: a-rollback-undoes-its-own-rows]\n"
            << steps;
    }
};

// Le troisième cas, joué dans un fils : rend le nombre de tours où une ligne de B a disparu.
static int playConcurrent(const std::string& databasePath, rag3db::main::SystemConfig& config,
    std::ofstream& steps) {
    Database database(databasePath, config);
    Connection a(&database);
    Connection b(&database);
    if (!a.query("CREATE NODE TABLE Note(id INT64 PRIMARY KEY, s STRING);")->isSuccess() ||
        !a.query("UNWIND range(0, 9) AS i CREATE (:Note {id: i, s: 'validée'});")->isSuccess()) {
        _exit(11);
    }
    int lost = 0;
    int refused = 0;
    for (auto round = 0; round < 20; round++) {
        const auto from = 1'000'000 + round * 100'000;
        a.query("BEGIN TRANSACTION;");
        a.query("UNWIND range(" + std::to_string(from) + ", " + std::to_string(from + 59'999) +
                ") AS i CREATE (:Note {id: i, s: 'a'});");
        b.query("BEGIN TRANSACTION;");
        b.query("CREATE (:Note {id: " + std::to_string(round) + "00, s: 'b'});");
        auto failures = std::make_shared<int>(1);
        CutFileSystem::failSyncOf() = [failures](const std::string& path) {
            return path.ends_with(".wal") && (*failures)-- > 0;
        };
        bool aRefused = false;
        std::thread ta([&] { aRefused = !a.query("COMMIT;")->isSuccess(); });
        // B demande son COMMIT pendant celui de A : il attend le verrou du gestionnaire.
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
        const bool bCommitted = b.query("COMMIT;")->isSuccess();
        ta.join();
        CutFileSystem::failSyncOf() = nullptr;
        refused += aRefused ? 1 : 0;
        auto r = b.query("MATCH (n:Note) WHERE n.s = 'b' RETURN count(n);");
        const auto seen = r->isSuccess() && r->hasNext() ?
                              r->getNext()->getValue(0)->getValue<int64_t>() :
                              -1;
        auto key = b.query("MATCH (n:Note {id: " + std::to_string(round) + "00}) RETURN count(n);");
        const auto byKey = key->isSuccess() && key->hasNext() ?
                               key->getNext()->getValue(0)->getValue<int64_t>() :
                               -1;
        const auto expected = round + 1 - lost;
        if (bCommitted && (seen != expected || byKey != 1)) {
            steps << "ROUND " << round << " A " << (aRefused ? "refused" : "accepted")
                  << " B committed, B rows by scan " << seen << " (expected " << expected
                  << "), by key " << byKey << std::endl;
            lost++;
        }
        if (!bCommitted) {
            steps << "ROUND " << round << " B refused" << std::endl;
        }
    }
    steps << "REFUSED-A " << refused << std::endl;
    steps << "LOST " << lost << std::endl;
    return lost;
}

TEST_F(RefusedCommitRollback, AWriterCommittingWhileARefusedCommitUnwindsKeepsItsRow) {
    const auto stepsPath = databasePath + ".steps";
    std::filesystem::remove(stepsPath);
    const auto pid = fork();
    if (pid == 0) {
        VirtualFileSystem::setDefaultFileSystemFactoryForTesting(
            [](const std::string& home) { return std::make_unique<CutFileSystem>(home); });
        std::ofstream steps(stepsPath);
        playConcurrent(databasePath, *systemConfig, steps);
        steps.flush();
        _exit(0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    std::string steps;
    std::ifstream in(stepsPath);
    for (std::string line; std::getline(in, line);) {
        steps += line + "\n";
    }
    ASSERT_TRUE(WIFEXITED(status) && WEXITSTATUS(status) == 0)
        << "[check: setup] the child ended with status " << status << "\n"
        << steps;
    ASSERT_EQ(steps.find("REFUSED-A 0\n"), std::string::npos)
        << "[check: setup] A's commit was never refused\n"
        << steps;
    EXPECT_NE(steps.find("LOST 0\n"), std::string::npos)
        << "[check: a-rollback-keeps-the-rows-of-another-writer] concurrent\n"
        << steps;
}

TEST_F(RefusedCommitRollback, AnotherWriterCommittingBeforeTheRollbackKeepsItsRow) {
    expectBKept(play(false));
}

TEST_F(RefusedCommitRollback, AnotherWriterCommittingABlockBeforeTheRollbackKeepsItsRow) {
    expectBKept(play(true));
}

} // namespace
