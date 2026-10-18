// Un BEGIN refusé laisse la connexion en transaction explicite sans transaction (ticket
// 2026-10-18-un-begin-refuse-laisse-la-connexion-sans-transaction). Trouvé par le scénario des
// vingt écrivains (twenty_writers_scenario_test.cpp) : sous un tampon de 64 Mo plein, un BEGIN
// refusé par « Unable to allocate memory », puis la requête suivante du même écrivain plante dans
// CatalogSet::traverseVersionChainsForTransactionNoLock, transaction nulle (rdi = 0 sous gdb).
//
// TransactionContext::beginWriteTransaction pose le mode MANUAL avant d'appeler
// TransactionManager::beginTransaction ; si celle-ci lève (un second écrivain hors du mode
// multi-écrivains, un point de reprise échoué, le tampon plein), le mode reste MANUAL et la
// transaction nulle. ROLLBACK répond « No active transaction » sans rendre le mode, et la requête
// suivante se lie sans transaction.
//
// - FailedBegin.LeavesTheConnectionUsable : la recette déterministe, le mode multi-écrivains éteint
//   pour que le second BEGIN soit refusé. Le tout se joue dans un processus fils : un plantage y
//   est un rouge nommé, pas la fin de la passe. Attendu : le fils ne meurt pas, et le MERGE qui
//   suit le BEGIN refusé passe ou est refusé sous un nom.

#ifndef __SINGLE_THREADED__

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>

#include "bench_harness.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::testing;

namespace {

class FailedBegin : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
    }
};

TEST_F(FailedBegin, LeavesTheConnectionUsable) {
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
            steps << who << " " << query << " -> "
                  << (result->isSuccess() ? "ok" : "refus « " + result->getErrorMessage() + " »")
                  << std::endl;
            return result->isSuccess();
        };
        rag3db::main::Database database(databasePath, *systemConfig);
        rag3db::main::Connection a(&database);
        rag3db::main::Connection b(&database);
        say(a, "A", "CALL enable_multi_writes=false;");
        say(a, "A", "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, titre STRING);");
        say(a, "A", "BEGIN TRANSACTION;");
        say(a, "A", "CREATE (:Doc {id: 1, titre: 'a'});");
        // Le résultat d'abord : say écrit sa ligne pendant l'évaluation.
        const auto begun = say(b, "B", "BEGIN TRANSACTION;");
        steps << "PLAN " << (begun ? "begin-accepted" : "begin-refused") << std::endl;
        say(b, "B", "ROLLBACK;");
        const auto merged = say(b, "B", "MERGE (d:Doc {id: 2}) SET d.titre = 'b';");
        steps << "MERGE " << (merged ? "ok" : "refused") << std::endl;
        say(a, "A", "COMMIT;");
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
    std::cerr << "  [begin refusé] le fils :\n" << steps;
    ASSERT_EQ(steps.find("PLAN begin-accepted"), std::string::npos)
        << "[check: plan] the second BEGIN was accepted: nothing is witnessed";
    ASSERT_TRUE(WIFEXITED(status) || WIFSIGNALED(status));
    EXPECT_FALSE(WIFSIGNALED(status))
        << "[check: failed-begin-leaves-the-connection-usable] the child died of signal "
        << WTERMSIG(status) << " on the statement after the refused BEGIN";
    if (WIFEXITED(status)) {
        EXPECT_NE(steps.find("MERGE "), std::string::npos)
            << "[check: failed-begin-leaves-the-connection-usable] no MERGE outcome";
    }
}

} // namespace

#endif
