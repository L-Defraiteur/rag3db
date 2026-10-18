// Un COMMIT dont le point de reprise échoue après la validation fait annuler une transaction déjà
// détruite (ticket
// 2026-10-18-un-commit-dont-le-point-de-reprise-echoue-annule-une-transaction-deja- partie).
// TransactionManager::commit retire la transaction de activeTransactions (et la détruit) avant
// d'écrire le point de reprise demandé ; s'il échoue, writeRequestedCheckpointIfLastNoLock lève une
// TransactionManagerException, et le contexte de la connexion annule sa transaction :
// TransactionManager::rollback sur l'objet libéré. Sous ASAN : heap-use-after-free ; en Release, un
// comportement indéfini qui peut passer inaperçu.
//
// - CommitAfterFailedCheckpoint.DoesNotRollBackADestroyedTransaction, dans un processus fils (un
//   rapport d'ASAN ou une assertion y est un rouge nommé). Point de reprise automatique en marche,
//   seuil de 4 Ko, deux connexions. A : BEGIN, une ligne. B : des écritures validées au-delà du
//   seuil pendant que A est ouverte (B pose la demande et rend la main). Puis le Checkpointer en
//   panne dans checkpointStorage, et A : COMMIT. A part la dernière et écrit le point de reprise
//   demandé, qui échoue. commit-whose-checkpoint-fails-does-not-touch-a-destroyed-transaction : le
//   fils sort normalement (code 0, sans signal ni rapport d'ASAN), et le COMMIT de A est refusé par
//   le message qui dit la validation faite et le point de reprise échoué. Le jouer sous ASAN : en
//   Release, le défaut peut ne rien montrer.

#ifndef __SINGLE_THREADED__

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>

#include "../flaky_checkpointer.h"
#include "bench_harness.h"
#include "common/exception/runtime.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/checkpointer.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

class CheckpointerFailingOnStorage final : public rag3db::storage::Checkpointer {
public:
    explicit CheckpointerFailingOnStorage(rag3db::main::ClientContext& context)
        : Checkpointer(context) {}

    bool checkpointStorage() override {
        throw RuntimeException("checkpoint failed (banc, CommitAfterFailedCheckpoint).");
    }
};

class CommitAfterFailedCheckpoint : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
    }
};

TEST_F(CommitAfterFailedCheckpoint, DoesNotRollBackADestroyedTransaction) {
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
            steps << who << " " << query.substr(0, 60) << " -> "
                  << (ok ? "ok" : "refus « " + result->getErrorMessage().substr(0, 220) + " »")
                  << std::endl;
            return ok;
        };
        {
            rag3db::main::Database database(databasePath, *systemConfig);
            rag3db::main::Connection a(&database);
            rag3db::main::Connection b(&database);
            say(a, "A", "CALL enable_multi_writes=true;");
            say(a, "A", "CALL force_checkpoint_on_close=false;");
            say(a, "A", "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, titre STRING);");
            say(a, "A", "CALL auto_checkpoint=true;");
            say(a, "A", "CALL checkpoint_threshold=4096;");
            say(a, "A", "BEGIN TRANSACTION;");
            say(a, "A", "CREATE (:Doc {id: 0, titre: 'a'});");
            for (auto i = 1; i <= 20; ++i) {
                b.query(
                    stringFormat("CREATE (:Doc {id: {}, titre: '{}'});", i, std::string(400, 'x')));
            }
            steps << "B 20 writes committed" << std::endl;
            const FlakyCheckpointer failing([](rag3db::main::ClientContext& context) {
                return std::make_unique<CheckpointerFailingOnStorage>(context);
            });
            failing.setCheckpointer(*a.getClientContext());
            const auto committed = say(a, "A", "COMMIT;");
            steps << "COMMIT " << (committed ? "accepted" : "refused") << std::endl;
            say(a, "A", "RETURN 1;");
        }
        steps << "CLOSED" << std::endl;
        _exit(0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    std::string steps;
    {
        std::ifstream in(stepsPath);
        for (std::string line; std::getline(in, line);) {
            steps += line + "\n";
        }
    }
    std::cerr << "  [COMMIT, point de reprise en panne] le fils (statut " << status << ") :\n"
              << steps;
    ASSERT_NE(steps.find("B 20 writes committed"), std::string::npos)
        << "[check: plan] the child died before A's COMMIT, status " << status;
    const bool clean = WIFEXITED(status) && WEXITSTATUS(status) == 0;
    const bool named = steps.find("This transaction is committed") != std::string::npos;
    EXPECT_TRUE(clean && named)
        << "[check: commit-whose-checkpoint-fails-does-not-touch-a-destroyed-transaction] the "
           "child "
        << (WIFSIGNALED(status) ? "died of signal " + std::to_string(WTERMSIG(status)) :
                                  "exited with " + std::to_string(WEXITSTATUS(status)))
        << " (an ASAN report exits with 1); A's COMMIT named the failed checkpoint: " << named;
}

} // namespace

#endif
