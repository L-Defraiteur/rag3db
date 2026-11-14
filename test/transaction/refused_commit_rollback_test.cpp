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
#include <sys/wait.h>
#include <unistd.h>

#include <filesystem>
#include <fstream>
#include <memory>
#include <string>

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

TEST_F(RefusedCommitRollback, AnotherWriterCommittingBeforeTheRollbackKeepsItsRow) {
    expectBKept(play(false));
}

TEST_F(RefusedCommitRollback, AnotherWriterCommittingABlockBeforeTheRollbackKeepsItsRow) {
    expectBKept(play(true));
}

} // namespace
