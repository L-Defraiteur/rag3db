// Les coupures de courant (ticket 2026-10-04-durabilite-sur-faute-d-entree-sortie-ou-coupure).
//
// Chaque cas se joue dans un processus fils qui ouvre la base sur CutFileSystem
// (test/include/test_helper/cut_file_system.h) et meurt par _exit à un point nommé : tout ce qui
// n'était pas synchronisé meurt avec lui. Le parent rouvre la base sur le système de fichiers
// ordinaire et juge ce qui reste. La création et la suppression de fichiers restent immédiates
// dans cette forme (voir la limite de CutFileSystem).
//
// - PowerCut.TheDoubleLosesWhatIsNotSynced : la doublure elle-même (un fichier écrit sans sync
//   est vide après la coupure, synchronisé il est plein) ; sans lui, un vert ne prouverait rien.
// - PowerCut.ACommitThatReturnedOkSurvivesACut : une validation rendue « ok » est là après la
//   coupure (le journal est synchronisé avant que la validation soit publiée).
// - PowerCut.ACutAfterTheCheckpointMarkLosesNothing : coupure juste après la marque CHECKPOINT
//   du journal, avant la recopie des pages fantômes (RAG3DB_TEST_DIE_AT=checkpoint-logged).
// - PowerCut.AJournalThatCannotBeSyncedPublishesNothing : la synchronisation du journal lève
//   pendant une validation (un disque plein) ; la validation échoue, et rien d'elle n'est visible,
//   ni dans le processus qui continue, ni après la coupure. Le ticket : la validation était
//   publiée en mémoire (undoBuffer->commit) avant l'écriture du journal.
// - PowerCut.ACutDuringRecoveryAfterTheShadowPagesLosesNothing et
//   PowerCut.ACutDuringRecoveryAfterTheJournalIsRemovedLosesNothing : une première mort laisse un
//   point de reprise à rejouer ; la reprise est coupée à ses deux points nommés.

#include <sys/wait.h>
#include <unistd.h>

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <string>

#include "common/file_system/virtual_file_system.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/wal/wal_replayer.h"
#include "test_helper/cut_file_system.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

constexpr int CUT = 77;

class PowerCut : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        std::filesystem::create_directories(std::filesystem::path(databasePath).parent_path());
    }

    // Joue `body` dans un fils sur CutFileSystem (ou sur le local si cut est faux) ; rend son
    // statut. Le corps finit par _exit : c'est la coupure.
    template<typename F>
    int inAChild(bool cut, F body) {
        const auto pid = fork();
        if (pid == 0) {
            if (cut) {
                VirtualFileSystem::setDefaultFileSystemFactoryForTesting(
                    [](const std::string& home) { return std::make_unique<CutFileSystem>(home); });
            }
            body();
            _exit(0);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        return WIFEXITED(status) ? WEXITSTATUS(status) : 128 + WTERMSIG(status);
    }

    static bool run(Connection& c, const std::string& q) { return c.query(q)->isSuccess(); }

    // La base de 2 000 lignes validées en quatre fois : sum(id) = 1 999 000.
    static bool fill(Connection& c) {
        if (!run(c, "CREATE NODE TABLE T(id INT64 PRIMARY KEY, s STRING);")) {
            return false;
        }
        for (auto from = 0; from < 2000; from += 500) {
            if (!run(c, "UNWIND range(" + std::to_string(from) + ", " +
                            std::to_string(from + 499) +
                            ") AS i CREATE (:T {id: i, s: 'ligne ' + CAST(i AS STRING)});")) {
                return false;
            }
        }
        return true;
    }

    std::string judge(const std::string& q) {
        createDBAndConn();
        auto result = conn->query(q);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        return result->hasNext() ? result->getNext()->toString() : "AUCUNE LIGNE";
    }
};

TEST_F(PowerCut, TheDoubleLosesWhatIsNotSynced) {
    const auto unsynced = databasePath + ".unsynced";
    const auto synced = databasePath + ".synced";
    const auto status = inAChild(false, [&] {
        CutFileSystem fs{""};
        const auto flags = FileOpenFlags(FileFlags::WRITE | FileFlags::READ_ONLY |
                                         FileFlags::CREATE_IF_NOT_EXISTS);
        const uint8_t bytes[4] = {'a', 'b', 'c', 'd'};
        auto a = fs.openFile(unsynced, flags);
        a->writeFile(bytes, 4, 0);
        uint8_t back[4] = {};
        a->readFromFile(back, 4, 0);
        if (std::memcmp(back, bytes, 4) != 0 || a->getFileSize() != 4) {
            _exit(10); // la doublure ne rend pas ce qu'elle a reçu
        }
        auto b = fs.openFile(synced, flags);
        b->writeFile(bytes, 4, 0);
        b->syncFile();
        _exit(CUT);
    });
    ASSERT_EQ(status, CUT) << "[check: setup] the child did not reach the cut";
    EXPECT_EQ(std::filesystem::file_size(unsynced), 0u)
        << "[check: unsynced-write-is-lost-by-the-cut]";
    EXPECT_EQ(std::filesystem::file_size(synced), 4u) << "[check: synced-write-survives-the-cut]";
}

TEST_F(PowerCut, ACommitThatReturnedOkSurvivesACut) {
    const auto status = inAChild(true, [&] {
        Database database(databasePath, *systemConfig);
        Connection connection(&database);
        if (!fill(connection)) {
            _exit(11);
        }
        _exit(CUT); // les validations ont rendu « ok » : coupure
    });
    ASSERT_EQ(status, CUT) << "[check: setup] the child did not reach the cut";
    EXPECT_EQ(judge("MATCH (t:T) RETURN count(*), sum(t.id);"), "2000|1999000\n")
        << "[check: committed-rows-survive-a-cut]";
}

TEST_F(PowerCut, ACutAfterTheCheckpointMarkLosesNothing) {
    const auto status = inAChild(true, [&] {
        setenv("RAG3DB_TEST_HOOKS", "1", 1);
        setenv("RAG3DB_TEST_DIE_AT", "checkpoint-logged", 1);
        Database database(databasePath, *systemConfig);
        Connection connection(&database);
        if (!fill(connection)) {
            _exit(11);
        }
        connection.query("CHECKPOINT;"); // meurt à checkpoint-logged (_Exit 137)
        _exit(12);
    });
    ASSERT_EQ(status, 137) << "[check: setup] the child did not die at checkpoint-logged";
    EXPECT_EQ(judge("MATCH (t:T) RETURN count(*), sum(t.id);"), "2000|1999000\n")
        << "[check: a-cut-after-the-checkpoint-mark-loses-nothing]";
    EXPECT_EQ(judge("MATCH (t:T {id: 1234}) RETURN t.s;"), "ligne 1234\n")
        << "[check: a-cut-after-the-checkpoint-mark-loses-nothing]";
}

TEST_F(PowerCut, AJournalThatCannotBeSyncedPublishesNothing) {
    const auto stepsPath = databasePath + ".steps";
    std::filesystem::remove(stepsPath);
    const auto status = inAChild(true, [&] {
        std::ofstream steps(stepsPath);
        Database database(databasePath, *systemConfig);
        Connection connection(&database);
        if (!fill(connection)) {
            _exit(11);
        }
        auto count = [&](const std::string& q) {
            auto result = connection.query(q);
            return result->isSuccess() && result->hasNext() ? result->getNext()->toString() :
                                                              std::string("refus");
        };
        // Le disque est plein : le journal ne se synchronise plus.
        CutFileSystem::failSyncOf() = [](const std::string& path) {
            return path.ends_with(".wal");
        };
        auto refused = connection.query("CREATE (:T {id: 5000, s: 'perdue'});");
        steps << "COMMIT " << (refused->isSuccess() ? "accepted" : "refused") << " « "
              << (refused->isSuccess() ? "" : refused->getErrorMessage().substr(0, 200)) << " »"
              << std::endl;
        CutFileSystem::failSyncOf() = nullptr;
        steps << "AFTER " << count("MATCH (t:T) RETURN count(*);");
        steps << "LOST " << count("MATCH (t:T {id: 5000}) RETURN count(*);");
        // La clé est libre : la même ligne se valide maintenant.
        auto again = connection.query("CREATE (:T {id: 5000, s: 'gardée'});");
        steps << "AGAIN " << (again->isSuccess() ? "accepted" : "refused") << " « "
              << (again->isSuccess() ? "" : again->getErrorMessage().substr(0, 200)) << " »"
              << std::endl;
        auto read = connection.query("MATCH (t:T) RETURN count(*);");
        steps << "READ " << (read->isSuccess() ? "accepted" : "refused « " + read->getErrorMessage().substr(0, 200) + " »") << std::endl;
        steps.flush();
        _exit(CUT);
    });
    std::string steps;
    std::ifstream in(stepsPath);
    for (std::string line; std::getline(in, line);) {
        steps += line + "\n";
    }
    ASSERT_EQ(status, CUT) << "[check: setup] the child did not reach the cut\n" << steps;
    ASSERT_NE(steps.find("COMMIT refused"), std::string::npos)
        << "[check: setup] the failed sync did not fail the commit\n" << steps;
    EXPECT_NE(steps.find("AFTER 2000\n"), std::string::npos)
        << "[check: a-commit-whose-journal-failed-is-not-visible]\n" << steps;
    EXPECT_NE(steps.find("LOST 0\n"), std::string::npos)
        << "[check: a-commit-whose-journal-failed-is-not-visible]\n" << steps;
    EXPECT_NE(steps.find("AGAIN accepted"), std::string::npos)
        << "[check: the-key-of-the-failed-commit-is-free]\n" << steps;
    EXPECT_EQ(judge("MATCH (t:T) RETURN count(*), sum(t.id);"), "2001|2004000\n")
        << "[check: after-the-cut-only-the-synced-commit-is-there]";
    EXPECT_EQ(judge("MATCH (t:T {id: 5000}) RETURN t.s;"), "gardée\n")
        << "[check: after-the-cut-only-the-synced-commit-is-there]";
}

class PowerCutDuringRecovery : public PowerCut {
protected:
    // Une première mort, sans coupure, à checkpoint-logged : le journal finit par CHECKPOINT, le
    // fichier fantôme est plein, la reprise suivante rejoue le point de reprise.
    void leaveACheckpointToReplay() {
        const auto status = inAChild(false, [&] {
            setenv("RAG3DB_TEST_HOOKS", "1", 1);
            setenv("RAG3DB_TEST_DIE_AT", "checkpoint-logged", 1);
            Database database(databasePath, *systemConfig);
            Connection connection(&database);
            if (!fill(connection)) {
                _exit(11);
            }
            connection.query("CHECKPOINT;");
            _exit(12);
        });
        ASSERT_EQ(status, 137) << "[check: setup] the first death did not happen";
    }
    // La reprise, sur CutFileSystem, coupée au point nommé.
    void cutTheRecoveryAt(rag3db::storage::RecoveryPoint point) {
        const auto status = inAChild(true, [&] {
            rag3db::storage::WALReplayer::setRecoveryHookForTesting(
                [point](rag3db::storage::RecoveryPoint reached) {
                    if (reached == point) {
                        _exit(CUT);
                    }
                });
            Database database(databasePath, *systemConfig);
            _exit(13); // la reprise est passée sans atteindre le point
        });
        ASSERT_EQ(status, CUT) << "[check: setup] the recovery did not reach its point";
    }
};

TEST_F(PowerCutDuringRecovery, ACutAfterTheShadowPagesAreReplayedLosesNothing) {
    leaveACheckpointToReplay();
    cutTheRecoveryAt(rag3db::storage::RecoveryPoint::SHADOW_PAGES_REPLAYED);
    EXPECT_EQ(judge("MATCH (t:T) RETURN count(*), sum(t.id);"), "2000|1999000\n")
        << "[check: a-cut-after-the-shadow-pages-loses-nothing]";
}

TEST_F(PowerCutDuringRecovery, ACutAfterTheJournalIsRemovedLosesNothing) {
    leaveACheckpointToReplay();
    cutTheRecoveryAt(rag3db::storage::RecoveryPoint::JOURNAL_REMOVED);
    EXPECT_EQ(judge("MATCH (t:T) RETURN count(*), sum(t.id);"), "2000|1999000\n")
        << "[check: a-cut-after-the-journal-is-removed-loses-nothing]";
    EXPECT_EQ(judge("MATCH (t:T {id: 1234}) RETURN t.s;"), "ligne 1234\n")
        << "[check: a-cut-after-the-journal-is-removed-loses-nothing]";
}

} // namespace
