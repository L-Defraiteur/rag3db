// Le banc de concurrence d'écriture (marche A1). Spécification :
// docs/2-octobre-2026-00h36/01-specification-du-banc-de-concurrence.md.
//
// Ce banc prouve des corruptions ; il ne corrige rien. Un cas rouge ici est un
// résultat, pas une panne du banc : la liste des rouges attendus est dans
// known_red.txt, et la passe la compare au résultat (concurrence_test.known_red).
//
// Réglages communs, et pourquoi :
// - debug_enable_multi_writes=true : sans lui le second écrivain est refusé et il n'y
//   a pas de concurrence à mesurer.
// - auto_checkpoint=false partout sauf C8 : un point de reprise déclenché par un
//   commit attend que toutes les autres transactions soient sorties et expire au bout
//   de 5 s ; il mesurerait le point de reprise, pas les conflits. C8 est le cas du
//   point de reprise, explicite et automatique.
//
// Les cas déterministes tiennent la fenêtre de course ouverte par des transactions
// explicites et une barrière : BEGIN, écriture, barrière, puis les commits l'un
// après l'autre dans un ordre fixé. Aucun crochet dans le moteur.
//
// Chaque cas tourne sous plusieurs paramètres : le lanceur (fils ou processus) et le
// moment où l'on vérifie — à chaud (Hot), après CHECKPOINT, fermeture et réouverture
// (Reopen), après arrêt brutal et rejeu du journal (Crash). Les deux derniers
// comparent aussi toutes les réponses à celles d'avant (vidage canonique).

#ifndef __SINGLE_THREADED__

#include <signal.h>

#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <map>
#include <memory>
#include <random>
#include <set>

#include "bench_harness.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "integrity/integrity_checker.h"
#include "storage/storage_utils.h"

using namespace rag3db::common;
using namespace rag3db::testing;
using namespace rag3db::testing::concurrency;

namespace {

enum class Check { Hot, Reopen, Crash };

std::string checkName(Check check) {
    switch (check) {
    case Check::Hot:
        return "Hot";
    case Check::Reopen:
        return "Reopen";
    case Check::Crash:
        return "Crash";
    }
    return "?";
}

struct BenchParam {
    LaunchMode launcher;
    Check check;
};

std::string paramName(const ::testing::TestParamInfo<BenchParam>& info) {
    return launchModeName(info.param.launcher) + "_" + checkName(info.param.check);
}

// Graine des cas aléatoires, imprimée en tête de chaque cas, rejouable par
// CONCURRENCE_GRAINE. Par défaut fixe, pour que la passe soit comparable d'une fois à
// l'autre ; l'ordonnancement des fils, lui, ne l'est pas.
uint64_t benchSeed() {
    const auto* value = std::getenv("CONCURRENCE_GRAINE");
    return value != nullptr && *value != '\0' ? std::stoull(value) : 20261002;
}

uint32_t benchIterations(uint32_t defaultValue) {
    const auto* value = std::getenv("CONCURRENCE_ITERATIONS");
    return value != nullptr && *value != '\0' ? std::stoul(value) : defaultValue;
}

std::vector<uint32_t> ascending(uint32_t numWorkers) {
    std::vector<uint32_t> order;
    for (auto i = 0u; i < numWorkers; ++i) {
        order.push_back(i);
    }
    return order;
}

} // namespace

class ConcurrencyBench;

// Un cas du banc : la préparation (sur la connexion du test, avant les écrivains), le
// scénario d'un écrivain, et ce qu'on attend une fois les écrivains finis — évalué sur
// la base telle qu'elle est au moment de la vérification (à chaud, rouverte, rejouée).
struct BenchCase {
    uint32_t numWorkers = 2;
    std::function<void(ConcurrencyBench&)> setup;
    Scenario scenario;
    std::function<void(ConcurrencyBench&, const SharedArea&)> expect;
    // Un cas de conflit entre écrivains n'a pas de sens à plusieurs processus tant que
    // B n'existe pas : le second processus écrivain est refusé à l'ouverture (C9).
    bool runsInProcesses = false;
    bool onlyInProcesses = false;
    bool autoCheckpoint = false;
    // Au-delà, le moteur est tenu pour bloqué et le cas est rouge (voir launch).
    std::chrono::milliseconds guard = DEFAULT_GUARD;
    // La table porte un index HNSW : l'extension vector est chargée à chaque ouverture.
    bool vectorExtension = false;
    // Le scénario tourne dans un processus fils qui vérifie, puis ferme la base ; le père
    // rouvre et revérifie. Un plantage du moteur devient un rouge étiqueté
    // (writer-process-crashed) au lieu de tuer toute la passe. Pour les cas dont le
    // moteur d'aujourd'hui plante à coup sûr (H1, défaut 5 de l'étude HNSW).
    bool isolated = false;
};

class ConcurrencyBench : public EmptyDBTest, public ::testing::WithParamInterface<BenchParam> {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "the bench needs an on-disk database (checkpoint, reopen)";
        }
        systemConfig->maxDBSize = 1024ull * 1024 * 1024 * 1024;
        systemConfig->bufferPoolSize = 1024 * 1024 * 1024;
        createDBAndConn();
        applyBenchSettings(*conn);
    }

    // Le nom d'un test paramétré contient des « / » : le chemin de la base passe par des
    // répertoires intermédiaires (Launchers/, Launchers/ConcurrencyBench.<cas>/) que
    // tous les processus de test partagent. On ne les supprime pas : le 3 octobre, les
    // supprimer quand ils étaient vides a probablement fait échouer une passe parallèle
    // (un autre processus créait sa base dedans au même instant). Ils restent, vides et
    // stables ; seul le dossier propre à la base, au nom horodaté, est retiré.
    void TearDown() override { EmptyDBTest::TearDown(); }

    void mustRun(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << "\n" << result->getErrorMessage();
    }

    int64_t queryInt(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << "[check: query] " << query << "\n"
                                         << result->getErrorMessage();
        if (!result->isSuccess() || !result->hasNext()) {
            return -1;
        }
        return result->getNext()->getValue(0)->getValue<int64_t>();
    }

    // Fait tourner un cas sous le paramètre du test, écrit le compte rendu brut, puis
    // vérifie. Le vérificateur lit toujours après la fin des écrivains, jamais depuis un
    // lecteur concurrent : la course du lecteur d'un autre processus n'est pas corrigée
    // sur master (journal des chantiers §6).
    void runCase(const BenchCase& benchCase) {
        const auto [launcher, check] = GetParam();
        if (benchCase.onlyInProcesses && (launcher != LaunchMode::Process || check != Check::Hot)) {
            GTEST_SKIP() << "an inter-process case, checked hot with the Process launcher";
        }
        if (launcher == LaunchMode::Process && !benchCase.runsInProcesses) {
            GTEST_SKIP() << "B not built: a second writer process is refused when it opens "
                            "the database (case C9)";
        }
        if (benchCase.isolated && check == Check::Reopen) {
            GTEST_SKIP() << "an isolated case: its Hot check already closes, reopens and "
                            "checks again";
        }
        std::cerr << "[bench] " << ::testing::UnitTest::GetInstance()->current_test_info()->name()
                  << " (seed " << benchSeed() << ")\n";
        currentCase = &benchCase;
        if (benchCase.vectorExtension) {
            loadVectorExtension(*conn);
        }
        if (benchCase.autoCheckpoint) {
            mustRun("CALL auto_checkpoint=true;");
        }
        benchCase.setup(*this);

        SharedMapping mapping(benchCase.numWorkers);
        auto& area = mapping.get();
        std::vector<integrity::Violation> violations;
        switch (check) {
        case Check::Hot:
            if (benchCase.isolated) {
                violations = runIsolated(benchCase, area);
                break;
            }
            launchHere(launcher, benchCase, area);
            violations = verify("hot");
            break;
        case Check::Reopen:
            violations = runThenReopen(benchCase, area);
            break;
        case Check::Crash:
            violations = runThenCrash(benchCase, area);
            break;
        }
        // Une étiquette par invariant violé : known_red.txt épingle cet ensemble.
        std::set<std::string> violated;
        for (const auto& violation : violations) {
            violated.insert(violation.invariant);
        }
        std::string checks;
        for (const auto& invariant : violated) {
            checks += "[check: " + invariant + "] ";
        }
        EXPECT_TRUE(violations.empty()) << checks << "integrity violations, see the raw report";
        if (databaseLost) {
            return;
        }
        EXPECT_EQ(totalRefusals(area, Refusal::Unexpected), 0u)
            << "[check: no-unexpected-error] " << "an unexpected error";
        EXPECT_EQ(area.barrierTimedOut.load(), 0u)
            << "[check: barrier] " << "a writer never reached the barrier";
        EXPECT_EQ(area.guardExpired.load(), 0u)
            << "[check: guard] " << "the engine blocked a writer past the guard";
        benchCase.expect(*this, area);
    }

    // Rouvre la base du test et recharge ce que le cas demande (l'extension vector).
    // Une base qui ne se rouvre plus (le rejeu du journal refuse, par exemple, une clé
    // en double que des écrivains concurrents ont validée : H4, 3 octobre) est un rouge
    // étiqueté, pas une exception qui sort du test ; la suite du cas est alors sautée.
    bool reopen() {
        try {
            createDBAndConn();
            if (currentCase != nullptr && currentCase->vectorExtension) {
                loadVectorExtension(*conn);
            }
            return true;
        } catch (const std::exception& e) {
            ADD_FAILURE() << "[check: database-reopens] the database could not be reopened: "
                          << e.what();
            conn.reset();
            database.reset();
            databaseLost = true;
            return false;
        }
    }

    // Après la mort d'un processus écrivain : ouvrir la base dans un processus neuf, comme un
    // service qui redémarre. Un plantage à l'ouverture est le rouge
    // database-opens-without-crash, et la suite du cas est sautée.
    bool probeAfterDeath() {
        const auto probe = probeOpenInFreshProcess(databasePath,
            currentCase != nullptr && currentCase->vectorExtension);
        if (probe.empty()) {
            return true;
        }
        ADD_FAILURE() << "[check: database-opens-without-crash] " << probe;
        databaseLost = true;
        return false;
    }

private:
    const BenchCase* currentCase = nullptr;
    bool databaseLost = false;

    // Le scénario dans un processus fils : il ouvre la base, lance les écrivains en fils,
    // vérifie (les violations passent par un fichier), puis ferme la base normalement.
    // Le père lit ce compte rendu, constate comment le fils est sorti, rouvre et
    // revérifie.
    std::vector<integrity::Violation> runIsolated(const BenchCase& benchCase, SharedArea& area) {
        const auto reportPath = databasePath + ".isolated-report";
        conn.reset();
        database.reset();
        const auto pid = fork();
        if (pid == 0) {
            disableCoreDumps();
            std::ofstream out(reportPath);
            try {
                auto childDatabase =
                    std::make_unique<rag3db::main::Database>(databasePath, *systemConfig);
                auto childConnection =
                    std::make_unique<rag3db::main::Connection>(childDatabase.get());
                applyBenchSettings(*childConnection);
                if (benchCase.vectorExtension) {
                    loadVectorExtension(*childConnection);
                }
                launch(LaunchMode::Thread,
                    Opener{childDatabase.get(), databasePath, *systemConfig,
                        benchCase.vectorExtension},
                    area, benchCase.scenario, benchCase.guard);
                auto violations = integrity::checkLevel1(*childConnection);
                const auto level2 = integrity::checkLevel2(*childConnection);
                violations.insert(violations.end(), level2.begin(), level2.end());
                for (const auto& violation : violations) {
                    out << violation.invariant << "\t" << violation.detail << "\n";
                }
                out.close();
                childConnection.reset();
                childDatabase.reset();
            } catch (const std::exception& e) {
                out << "child-failed\t" << e.what() << "\n";
                out.close();
            }
            _exit(0);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        const bool exitedNormally = WIFEXITED(status) && WEXITSTATUS(status) == 0;
        EXPECT_TRUE(exitedNormally)
            << "[check: writer-process-crashed] the isolated writer process did not exit "
               "normally — "
            << (WIFSIGNALED(status)  ? "signal " + std::to_string(WTERMSIG(status)) :
                   WIFEXITED(status) ? "exit code " + std::to_string(WEXITSTATUS(status)) :
                                       std::string("unknown status"));
        std::cerr << describe(area);
        std::vector<integrity::Violation> violations;
        {
            std::ifstream in(reportPath);
            for (std::string line; std::getline(in, line);) {
                const auto tab = line.find('\t');
                violations.push_back(
                    {line.substr(0, tab), tab == std::string::npos ? "" : line.substr(tab + 1)});
            }
        }
        std::filesystem::remove(reportPath);
        std::cerr << "  -- integrity in the writer process:\n" << integrity::describe(violations);
        if (!exitedNormally && !probeAfterDeath()) {
            return violations;
        }
        if (!reopen()) {
            return violations;
        }
        const auto after = verify("after close and reopen");
        violations.insert(violations.end(), after.begin(), after.end());
        return violations;
    }

    void launchHere(LaunchMode launcher, const BenchCase& benchCase, SharedArea& area) {
        if (launcher == LaunchMode::Thread) {
            launch(launcher,
                Opener{database.get(), databasePath, *systemConfig, benchCase.vectorExtension},
                area, benchCase.scenario, benchCase.guard);
        } else {
            // Les écrivains ouvrent eux-mêmes la base : le test doit l'avoir fermée.
            conn.reset();
            database.reset();
            EXPECT_TRUE(launch(launcher,
                Opener{nullptr, databasePath, *systemConfig, benchCase.vectorExtension}, area,
                benchCase.scenario, benchCase.guard))
                << "[check: process-exit] "
                << "a writer process did not exit normally";
            if (!reopen()) {
                return;
            }
        }
        std::cerr << describe(area);
    }

    std::vector<integrity::Violation> verify(const std::string& moment) {
        auto violations = integrity::checkLevel1(*conn);
        const auto level2 = integrity::checkLevel2(*conn);
        violations.insert(violations.end(), level2.begin(), level2.end());
        std::cerr << "  -- integrity " << moment << ":\n" << integrity::describe(violations);
        return violations;
    }

    std::vector<integrity::Violation> runThenReopen(const BenchCase& benchCase, SharedArea& area) {
        launchHere(LaunchMode::Thread, benchCase, area);
        verify("hot");
        const auto before = integrity::canonicalDump(*conn);
        {
            // Un résultat ne doit pas survivre à sa base : il est détruit avant la réouverture.
            auto checkpoint = conn->query("CHECKPOINT;");
            std::cerr << "  -- CHECKPOINT: "
                      << (checkpoint->isSuccess() ? "ok" : checkpoint->getErrorMessage()) << "\n";
            EXPECT_TRUE(checkpoint->isSuccess())
                << "[check: checkpoint] " << checkpoint->getErrorMessage();
        }
        if (!reopen()) {
            return {};
        }
        auto violations = verify("after checkpoint and reopen");
        const auto sameAnswers = integrity::compareDumps(before, integrity::canonicalDump(*conn),
            "before the checkpoint", "after the reopen");
        std::cerr << "  -- answers before / after:\n" << integrity::describe(sameAnswers);
        violations.insert(violations.end(), sameAnswers.begin(), sameAnswers.end());
        return violations;
    }

    // Les écrivains tournent dans un processus fils qui ouvre la base, fait son
    // scénario, note ses réponses, puis se tue par SIGKILL : tous ses commits ont été
    // acquittés, la base n'a pas été fermée. Le père rouvre, ce qui rejoue le journal,
    // et compare.
    std::vector<integrity::Violation> runThenCrash(const BenchCase& benchCase, SharedArea& area) {
        const auto hotDumpPath = databasePath + ".hot-dump";
        conn.reset();
        database.reset();
        const auto pid = fork();
        if (pid == 0) {
            disableCoreDumps();
            std::ofstream out(hotDumpPath);
            try {
                rag3db::main::Database childDatabase(databasePath, *systemConfig);
                rag3db::main::Connection childConnection(&childDatabase);
                applyBenchSettings(childConnection);
                if (benchCase.vectorExtension) {
                    loadVectorExtension(childConnection);
                }
                if (benchCase.autoCheckpoint) {
                    childConnection.query("CALL auto_checkpoint=true;");
                }
                launch(LaunchMode::Thread,
                    Opener{&childDatabase, databasePath, *systemConfig, benchCase.vectorExtension},
                    area, benchCase.scenario, benchCase.guard);
                for (const auto& line : integrity::canonicalDump(childConnection)) {
                    out << line << "\n";
                }
                out.close();
                // Mourir ici, base ouverte. Jusqu'au 3 octobre au soir, le SIGKILL venait
                // après la fin de ce bloc : la base était détruite, donc fermée proprement
                // (point de reprise final), et la variante Crash ne rejouait aucun journal.
                kill(getpid(), SIGKILL);
            } catch (const std::exception& e) {
                out << "CHILD FAILED: " << e.what() << "\n";
            }
            out.close();
            kill(getpid(), SIGKILL);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        EXPECT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
            << "[check: killed] "
            << "the writer process was expected to die by SIGKILL" << " — "
            << (WIFSIGNALED(status)  ? "signal " + std::to_string(WTERMSIG(status)) :
                   WIFEXITED(status) ? "exit code " + std::to_string(WEXITSTATUS(status)) :
                                       std::string("unknown status"));
        std::cerr << describe(area);

        std::vector<std::string> before;
        {
            std::ifstream in(hotDumpPath);
            for (std::string line; std::getline(in, line);) {
                before.push_back(line);
            }
        }
        std::filesystem::remove(hotDumpPath);
        if (!probeAfterDeath()) {
            return {};
        }
        // Le témoin que l'arrêt est brutal : un journal non vide attend d'être rejoué.
        const auto walPath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
        const auto walSize =
            std::filesystem::exists(walPath) ? std::filesystem::file_size(walPath) : 0;
        std::cerr << "  -- journal to replay: " << walSize << " bytes\n";
        EXPECT_GT(walSize, 0u) << "[check: journal-to-replay] the writer process closed its "
                                  "database before dying: nothing is replayed";
        if (!reopen()) {
            return {};
        }
        reportSetAsideJournals();
        auto violations = verify("after kill and replay");
        const auto sameAnswers = integrity::compareDumps(before, integrity::canonicalDump(*conn),
            "before the kill", "after the replay");
        std::cerr << "  -- answers before / after:\n" << integrity::describe(sameAnswers);
        violations.insert(violations.end(), sameAnswers.begin(), sameAnswers.end());
        return violations;
    }

    // Depuis l'étape E du journal, une fin coupée est copiée dans <wal>.ecarte-<ms>. Un
    // arrêt après des commits acquittés ne devrait rien écarter : on le dit, et on
    // nettoie pour le cas suivant.
    void reportSetAsideJournals() {
        const auto walPath =
            std::filesystem::path(rag3db::storage::StorageUtils::getWALFilePath(databasePath));
        const auto prefix = walPath.filename().string() + ".ecarte-";
        for (const auto& entry : std::filesystem::directory_iterator(walPath.parent_path())) {
            if (entry.path().filename().string().starts_with(prefix)) {
                std::cerr << "  -- set-aside journal: " << entry.path().filename() << " ("
                          << entry.file_size() << " bytes)\n";
                ADD_FAILURE() << "[check: no-set-aside] "
                              << "the replay set aside part of the journal after acknowledged "
                                 "commits: "
                              << entry.path();
                std::filesystem::remove(entry.path());
            }
        }
    }
};

// C0 — témoin. Clés disjointes : chaque écrivain crée ses propres nœuds, chacun dans
// sa transaction, puis des relations entre ses nœuds déjà validés. Doit être vert :
// il prouve que le banc et le vérificateur ne crient pas sans raison.
TEST_P(ConcurrencyBench, C0_DisjointKeysWitness) {
    static constexpr uint32_t numWorkers = 4;
    static constexpr int64_t numNodes = 50;
    runCase({.numWorkers = numWorkers,
        .setup =
            [](ConcurrencyBench& bench) {
                bench.mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
                bench.mustRun(
                    "CREATE REL TABLE Link(FROM Item TO Item, src_id INT64, dst_id INT64);");
            },
        .scenario =
            [](Worker& worker) {
                const int64_t base = worker.index() * 1000;
                for (auto i = 0; i < numNodes; ++i) {
                    worker.run(stringFormat("CREATE (:Item {id: {}, writer: {}});", base + i,
                        worker.index()));
                }
                worker.sync();
                for (auto i = 0; i + 1 < numNodes; ++i) {
                    worker.run(stringFormat("MATCH (a:Item {id: {}}), (b:Item {id: {}}) "
                                            "CREATE (a)-[:Link {src_id: {}, dst_id: {}}]->(b);",
                        base + i, base + i + 1, base + i, base + i + 1));
                }
            },
        .expect =
            [](ConcurrencyBench& bench, const SharedArea&) {
                EXPECT_EQ(bench.queryInt("MATCH (n:Item) RETURN count(n);"), numWorkers * numNodes)
                    << "[check: node-count] ";
                EXPECT_EQ(bench.queryInt("MATCH ()-[r:Link]->() RETURN count(r);"),
                    numWorkers * (numNodes - 1))
                    << "[check: rel-count] ";
            }});
}

// C1 — même clé primaire. Chaque écrivain ouvre une transaction et crée le nœud de clé
// 7 ; puis tous valident l'un après l'autre (commitInOrderByEvents : les écritures
// sont toutes finies avant le premier commit, sans barrière). Invariant : un seul
// commit réussit, une seule ligne de clé 7. Rouge à l'étape 1 : la clé non validée de
// l'autre transaction est invisible au contrôle d'unicité (node_table.cpp,
// validatePkNotExists).
static BenchCase samePrimaryKey(uint32_t numWorkers) {
    return {.numWorkers = numWorkers,
        .setup =
            [](ConcurrencyBench& bench) {
                bench.mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
            },
        .scenario =
            [](Worker& worker) {
                worker.begin();
                worker.writeInTurn(
                    stringFormat("CREATE (:Item {id: 7, writer: {}});", worker.index()));
                worker.commitInOrderByEvents(ascending(worker.numWorkers()));
            },
        .expect =
            [](ConcurrencyBench& bench, const SharedArea& area) {
                EXPECT_EQ(totalCommits(area), 1u)
                    << "[check: one-commit] " << "exactly one writer of key 7 may commit";
                EXPECT_EQ(bench.queryInt("MATCH (n:Item) WHERE n.id = 7 RETURN count(n);"), 1)
                    << "[check: one-row-per-key] ";
            }};
}

TEST_P(ConcurrencyBench, C1_SamePrimaryKeyTwoWriters) {
    runCase(samePrimaryKey(2));
}

TEST_P(ConcurrencyBench, C1_SamePrimaryKeyThreeWriters) {
    runCase(samePrimaryKey(3));
}

// C1 — la même clé SANS chevauchement des écritures (relecture du cœur C++, 3 octobre).
// L'écrivain 1 ouvre sa transaction et lit : son instantané est pris. Puis l'écrivain 0
// ouvre, crée la clé 7 et valide. Enfin l'écrivain 1 crée la clé 7 et valide. Aucune
// écriture ne se chevauche : un verrou par clé ne voit rien. Seul un contrôle
// d'unicité fait contre le dernier état validé — et non contre l'instantané de
// l'écrivain 1, qui précède le commit de l'écrivain 0 — refuse le second. Invariant :
// un seul commit, une seule ligne de clé 7.
TEST_P(ConcurrencyBench, C1_SnapshotPredatesCommit) {
    runCase({.numWorkers = 2,
        .setup =
            [](ConcurrencyBench& bench) {
                bench.mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
            },
        .scenario =
            [](Worker& worker) {
                if (worker.index() == 1) {
                    worker.begin();
                    worker.run("MATCH (n:Item) RETURN count(n);");
                }
                worker.sync();
                if (worker.index() == 0) {
                    worker.begin();
                    worker.run("CREATE (:Item {id: 7, writer: 0});");
                    worker.commit();
                }
                worker.sync();
                if (worker.index() == 1) {
                    worker.run("CREATE (:Item {id: 7, writer: 1});");
                    worker.commit();
                }
            },
        .expect =
            [](ConcurrencyBench& bench, const SharedArea& area) {
                EXPECT_EQ(totalCommits(area), 1u)
                    << "[check: one-commit] " << "exactly one writer of key 7 may commit";
                EXPECT_EQ(bench.queryInt("MATCH (n:Item) WHERE n.id = 7 RETURN count(n);"), 1)
                    << "[check: one-row-per-key] ";
            }});
}

// L'attente sous les verrous (A4′), prouvée par l'ordre des événements : la fin d'écriture de
// l'écrivain 1 vient après la validation de l'écrivain 0.
static void expectSecondWriterWaited(const SharedArea& area) {
    const auto firstCommit = eventIndex(area, 0, "commit:done");
    auto secondWrite = eventIndex(area, 1, "write:done");
    if (secondWrite < 0) {
        secondWrite = eventIndex(area, 1, "write:failed");
    }
    EXPECT_GT(secondWrite, firstCommit)
        << "[check: waited] the second writer's write ended before the first writer committed";
}

// C2 — relation contre suppression d'une de ses extrémités. Les nœuds 1 et 2 sont
// validés avant. L'écrivain 0 supprime un nœud (la source 1, la destination 2, ou la
// source par DETACH DELETE) ; l'écrivain 1 crée une relation 1 -> 2. Puis les deux
// commits dans l'ordre donné. Invariant : jamais les deux à la fois. Rouge à l'étape 1
// pour la source, dans les deux ordres : chacun ne voit que son instantané
// (delete_executor.cpp, throwIfNodeHasRels). Les variantes destination et DETACH
// DELETE viennent de la relecture du cœur C++ (3 octobre) : l'autre direction de la
// CSR, et une suppression qui détache ce qu'elle voit — pas la relation encore non
// validée de l'autre.
// Sous les verrous (A4′), celui qui doit valider le premier écrit aussi le premier : l'écrivain
// 0 écrit, l'écrivain 1 attend dans son écriture (writeInTurn), et les validations suivent
// l'ordre {0, 1}. relationFirst : l'écrivain 0 crée la relation et l'écrivain 1 supprime ; sinon
// l'inverse. Attendus (page 07 du cœur C++, §0) :
// - la suppression d'abord : la relation voit une extrémité supprimée par une validation
//   postérieure à son instantané, erreur de sérialisation (comme PostgreSQL) ;
// - la relation d'abord, DELETE sans DETACH : refus « has connected edges » d'après le dernier
//   état validé ;
// - la relation d'abord, DETACH DELETE : le nœud tenu, le détachement voit le dernier état
//   validé et détache la relation de l'autre (comme Neo4j) — les deux valident, et il ne reste
//   ni nœud ni relation.
// Dans tous les cas, l'écrivain 1 a attendu : sa fin d'écriture vient après la validation de
// l'écrivain 0.
static BenchCase deleteVersusNewRelation(std::string deletion, bool relationFirst) {
    return {.numWorkers = 2,
        .setup =
            [](ConcurrencyBench& bench) {
                bench.mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
                bench.mustRun(
                    "CREATE REL TABLE Link(FROM Item TO Item, src_id INT64, dst_id INT64);");
                bench.mustRun("CREATE (:Item {id: 1, writer: -1}), (:Item {id: 2, writer: -1});");
            },
        .scenario =
            [deletion, relationFirst](Worker& worker) {
                worker.begin();
                if ((worker.index() == 0) == relationFirst) {
                    worker.writeInTurn("MATCH (a:Item {id: 1}), (b:Item {id: 2}) "
                                       "CREATE (a)-[:Link {src_id: 1, dst_id: 2}]->(b);");
                } else {
                    worker.writeInTurn(deletion);
                }
                worker.commitInOrderByEvents({0, 1});
            },
        .expect =
            [deletion, relationFirst](ConcurrencyBench& bench, const SharedArea& area) {
                expectSecondWriterWaited(area);
                const bool detaches = deletion.find("DETACH") != std::string::npos;
                if (relationFirst && detaches) {
                    EXPECT_EQ(totalCommits(area), 2u)
                        << "[check: both-commit] the DETACH DELETE detaches the relation "
                           "committed before it";
                    EXPECT_EQ(bench.queryInt("MATCH (a:Item {id: 1}) RETURN count(a);"), 0)
                        << "[check: node-deleted] ";
                    EXPECT_EQ(bench.queryInt("MATCH ()-[r:Link]->() RETURN count(r);"), 0)
                        << "[check: no-dangling-relation] ";
                    return;
                }
                EXPECT_EQ(totalCommits(area), 1u)
                    << "[check: one-commit] "
                    << "the delete and the new relation cannot both commit";
                const auto refusal =
                    relationFirst ? Refusal::ConnectedEdges : Refusal::SerializationFailure;
                EXPECT_EQ(totalRefusals(area, refusal), 1u)
                    << "[check: named-refusal] the second writer must get "
                    << (relationFirst ? "« has connected edges »" : "the serialization error");
            }};
}

static constexpr const char* DELETE_SOURCE = "MATCH (a:Item {id: 1}) DELETE a;";
static constexpr const char* DELETE_DESTINATION = "MATCH (b:Item {id: 2}) DELETE b;";
static constexpr const char* DETACH_DELETE_SOURCE = "MATCH (a:Item {id: 1}) DETACH DELETE a;";

TEST_P(ConcurrencyBench, C2_DeleteCommitsFirst) {
    runCase(deleteVersusNewRelation(DELETE_SOURCE, false));
}

TEST_P(ConcurrencyBench, C2_RelationCommitsFirst) {
    runCase(deleteVersusNewRelation(DELETE_SOURCE, true));
}

TEST_P(ConcurrencyBench, C2_DestinationDeleteCommitsFirst) {
    runCase(deleteVersusNewRelation(DELETE_DESTINATION, false));
}

TEST_P(ConcurrencyBench, C2_DestinationRelationCommitsFirst) {
    runCase(deleteVersusNewRelation(DELETE_DESTINATION, true));
}

TEST_P(ConcurrencyBench, C2_DetachDeleteCommitsFirst) {
    runCase(deleteVersusNewRelation(DETACH_DELETE_SOURCE, false));
}

TEST_P(ConcurrencyBench, C2_DetachRelationCommitsFirst) {
    runCase(deleteVersusNewRelation(DETACH_DELETE_SOURCE, true));
}

// C3 — offsets locaux qui se chevauchent (marche A2). Chaque écrivain, dans une seule
// transaction, crée ses nœuds puis une chaîne de relations entre eux ; chaque relation
// porte les clés de ses extrémités. Les deux transactions numérotent leurs nœuds à
// partir de la même taille de table. Invariant : chaque relation relie les nœuds dont
// elle porte les clés. Rouge à l'étape 1, sans erreur : les relations du second à
// valider pointent vers les nœuds du premier.
TEST_P(ConcurrencyBench, C3_OverlappingLocalOffsets) {
    static constexpr uint32_t numWorkers = 2;
    static constexpr int64_t numNodes = 8;
    runCase({.numWorkers = numWorkers,
        .setup =
            [](ConcurrencyBench& bench) {
                bench.mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
                bench.mustRun(
                    "CREATE REL TABLE Link(FROM Item TO Item, src_id INT64, dst_id INT64);");
            },
        .scenario =
            [](Worker& worker) {
                const int64_t base = (worker.index() + 1) * 100;
                worker.begin();
                for (auto i = 0; i < numNodes; ++i) {
                    worker.run(stringFormat("CREATE (:Item {id: {}, writer: {}});", base + i,
                        worker.index()));
                }
                for (auto i = 0; i + 1 < numNodes; ++i) {
                    worker.run(stringFormat("MATCH (a:Item {id: {}}), (b:Item {id: {}}) "
                                            "CREATE (a)-[:Link {src_id: {}, dst_id: {}}]->(b);",
                        base + i, base + i + 1, base + i, base + i + 1));
                }
                worker.sync();
                worker.commitInOrder(ascending(worker.numWorkers()));
            },
        .expect =
            [](ConcurrencyBench& bench, const SharedArea& area) {
                EXPECT_EQ(totalCommits(area), numWorkers) << "[check: all-commit] ";
                EXPECT_EQ(bench.queryInt("MATCH (n:Item) RETURN count(n);"), numWorkers * numNodes)
                    << "[check: node-count] ";
                EXPECT_EQ(bench.queryInt("MATCH ()-[r:Link]->() RETURN count(r);"),
                    numWorkers * (numNodes - 1))
                    << "[check: rel-count] ";
            }});
}

// C4 — virements. Huit comptes à 100 ; chaque écrivain fait des virements aléatoires,
// chacun dans sa transaction (débit puis crédit). Un conflit d'écriture annule le
// virement entier. Invariant : la somme ne bouge pas — une mise à jour perdue la
// changerait. Attendu vert : le conflit de mise à jour d'une même ligne existe
// (update_info.cpp).
TEST_P(ConcurrencyBench, C4_Transfers) {
    static constexpr int64_t numAccounts = 8;
    const auto iterations = benchIterations(100);
    runCase({.numWorkers = 4,
        .setup =
            [](ConcurrencyBench& bench) {
                bench.mustRun("CREATE NODE TABLE Account(id INT64 PRIMARY KEY, balance INT64);");
                for (auto i = 0; i < numAccounts; ++i) {
                    bench.mustRun(stringFormat("CREATE (:Account {id: {}, balance: 100});", i));
                }
            },
        .scenario =
            [iterations](Worker& worker) {
                std::mt19937_64 random(benchSeed() + worker.index());
                std::uniform_int_distribution<int64_t> account(0, numAccounts - 1);
                std::uniform_int_distribution<int64_t> amount(1, 10);
                for (auto i = 0u; i < iterations; ++i) {
                    const auto from = account(random);
                    auto to = account(random);
                    if (to == from) {
                        to = (to + 1) % numAccounts;
                    }
                    const auto value = amount(random);
                    worker.begin();
                    worker.run(stringFormat(
                        "MATCH (a:Account {id: {}}) SET a.balance = a.balance - {};", from, value));
                    worker.run(stringFormat(
                        "MATCH (a:Account {id: {}}) SET a.balance = a.balance + {};", to, value));
                    worker.commit();
                }
            },
        .expect =
            [](ConcurrencyBench& bench, const SharedArea& area) {
                EXPECT_EQ(bench.queryInt("MATCH (a:Account) RETURN count(a);"), numAccounts)
                    << "[check: account-count] ";
                EXPECT_EQ(bench.queryInt("MATCH (a:Account) RETURN sum(a.balance);"),
                    numAccounts * 100)
                    << "[check: sum-conserved] ";
                EXPECT_GT(totalCommits(area), 0u)
                    << "[check: some-commit] " << "no transfer committed at all";
            }});
}

// C5 — double suppression de la même ligne. Invariant : une seule suppression valide,
// l'autre est refusée par « Write-write conflict ». Les deux DELETE partent en même
// temps. Mesuré le 3 octobre sur 200 répétitions : rouge 2/200 à chaud, 2/200 après
// réouverture, 52/200 après arrêt brutal — les deux valident. C'est la course du chemin
// de suppression sans verrou (marche A5) ; le cas est donc probabiliste
// (probabilistic.txt), et son témoin sous ThreadSanitizer sort 9 passes sur 10.
// Sous les verrous (A4′) : l'écrivain 0 supprime le premier, l'écrivain 1 attend le verrou de
// la ligne, puis reçoit l'erreur de sérialisation — la ligne a été supprimée et validée après son
// instantané (option A, comme PostgreSQL). Plus « en même temps » : l'ordre de prise du verrou
// déciderait de l'ordre des validations, et {0, 1} attendrait jusqu'au délai.
TEST_P(ConcurrencyBench, C5_DoubleDelete) {
    runCase({.numWorkers = 2,
        .setup =
            [](ConcurrencyBench& bench) {
                bench.mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
                bench.mustRun("CREATE (:Item {id: 1, writer: -1}), (:Item {id: 2, writer: -1});");
            },
        .scenario =
            [](Worker& worker) {
                worker.begin();
                worker.writeInTurn("MATCH (n:Item {id: 1}) DELETE n;");
                worker.commitInOrderByEvents({0, 1});
            },
        .expect =
            [](ConcurrencyBench& bench, const SharedArea& area) {
                EXPECT_EQ(totalCommits(area), 1u)
                    << "[check: one-commit] " << "exactly one delete may commit";
                expectSecondWriterWaited(area);
                EXPECT_EQ(totalRefusals(area, Refusal::SerializationFailure), 1u)
                    << "[check: serialization-refusal] ";
                EXPECT_EQ(bench.queryInt("MATCH (n:Item) RETURN count(n);"), 1)
                    << "[check: row-count] ";
            }});
}

// C6 — suppression contre mise à jour de la même ligne. L'écrivain 0 supprime le nœud
// 1, l'écrivain 1 change sa valeur ; puis les commits dans l'ordre donné. Aujourd'hui
// les deux valident, la ligne finit supprimée et la mise à jour est perdue sans erreur
// (reproduit dans un seul fil : MinimalReproduction.C6_*).
// L'attendu dépend de l'écart n° 2 de la note de conception sur les verrous
// (docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md), pas encore tranché
// par Lucie :
// - option A, recommandée — un instantané par transaction (le comportement Repeatable
//   Read de PostgreSQL) : JAMAIS LES DEUX, l'invariant écrit ici. Le second écrivain
//   attend ; si le premier valide, le second reçoit une erreur nommée et transitoire ;
//   si le premier annule, le second passe ;
// - option B — un instantané par instruction (Read Committed) : les deux valident, et
//   la mise à jour touche zéro ligne ; l'état final d'aujourd'hui serait alors
//   admissible.
// Sous l'une comme sous l'autre, ce qui est un défaut aujourd'hui : il n'y a ni attente
// ni détection croisée (la mise à jour ne regarde que les mises à jour, update_info.cpp ;
// la suppression que les suppressions, version_info.cpp), et ThreadSanitizer voit une
// course sur isDeleted. Lucie a retenu l'option A ; A4′ la met en œuvre (attente, puis erreur
// de sérialisation).
// Sous les verrous (A4′), celui qui doit valider le premier écrit le premier : l'écrivain 0
// supprime (ou met à jour, updateFirst), l'écrivain 1 attend le verrou de la ligne, puis reçoit
// l'erreur de sérialisation (option A) ; les validations suivent l'ordre {0, 1}.
static BenchCase deleteVersusUpdate(bool updateFirst) {
    return {.numWorkers = 2,
        .setup =
            [](ConcurrencyBench& bench) {
                bench.mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
                bench.mustRun("CREATE (:Item {id: 1, writer: -1});");
            },
        .scenario =
            [updateFirst](Worker& worker) {
                worker.begin();
                if ((worker.index() == 0) == updateFirst) {
                    worker.writeInTurn("MATCH (n:Item {id: 1}) SET n.writer = 1;");
                } else {
                    worker.writeInTurn("MATCH (n:Item {id: 1}) DELETE n;");
                }
                worker.commitInOrderByEvents({0, 1});
            },
        .expect =
            [](ConcurrencyBench& bench, const SharedArea& area) {
                expectSecondWriterWaited(area);
                EXPECT_EQ(totalRefusals(area, Refusal::SerializationFailure), 1u)
                    << "[check: serialization-refusal] ";
                EXPECT_EQ(totalCommits(area), 1u)
                    << "[check: one-commit] "
                    << "the delete and the update of the same row cannot both commit";
                const auto remaining = bench.queryInt("MATCH (n:Item) RETURN count(n);");
                const auto updated =
                    bench.queryInt("MATCH (n:Item) WHERE n.writer = 1 RETURN count(n);");
                EXPECT_TRUE((remaining == 0) || (remaining == 1 && updated == 1))
                    << "[check: final-state] " << "remaining rows: " << remaining
                    << ", updated rows: " << updated;
            }};
}

TEST_P(ConcurrencyBench, C6_DeleteCommitsFirst) {
    runCase(deleteVersusUpdate(false));
}

TEST_P(ConcurrencyBench, C6_UpdateCommitsFirst) {
    runCase(deleteVersusUpdate(true));
}

// C7 — mélange aléatoire sur un petit domaine de clés : créations (doublons
// possibles), suppressions DETACH, relations, mises à jour, une transaction sur huit
// annulée volontairement. Invariant : tout le vérificateur. Le filet des marches
// suivantes.
// Pourquoi cinq manches : à l'étape 2, une seule manche a été vue verte puis rouge sur
// la même graine — l'ordonnancement décide. Un test qui rougit une fois sur deux ne
// doit jamais se lire comme un vert. Le cas joue donc cinq manches, chacune sur ses
// propres tables (Item0/Link0 … Item4/Link4), et il est rouge si une seule viole un
// invariant. Mesuré le 3 octobre sur vingt passes : rouge 18/20 à chaud, 19/20 après
// réouverture, 15/20 après arrêt brutal. Pas unanime : il porte le label
// concurrence-probabiliste (probabilistic.txt) et n'est pas comparé ; son rouge est un
// vrai rouge, son vert ne prouve rien.
TEST_P(ConcurrencyBench, C7_RandomMix) {
    static constexpr int64_t numKeys = 12;
    static constexpr uint32_t numRounds = 5;
    const auto iterations = benchIterations(150);
    runCase({.numWorkers = 4,
        .setup =
            [](ConcurrencyBench& bench) {
                for (auto round = 0u; round < numRounds; ++round) {
                    bench.mustRun(stringFormat(
                        "CREATE NODE TABLE Item{}(id INT64 PRIMARY KEY, writer INT64);", round));
                    bench.mustRun(stringFormat("CREATE REL TABLE Link{}(FROM Item{} TO Item{}, "
                                               "src_id INT64, dst_id INT64);",
                        round, round, round));
                }
            },
        .scenario =
            [iterations](Worker& worker) {
                std::mt19937_64 random(benchSeed() + 100 + worker.index());
                std::uniform_int_distribution<int64_t> key(0, numKeys - 1);
                std::uniform_int_distribution<int> operation(0, 3);
                std::uniform_int_distribution<int> numOperations(1, 3);
                for (auto round = 0u; round < numRounds; ++round) {
                    for (auto i = 0u; i < iterations; ++i) {
                        worker.begin();
                        for (auto n = numOperations(random); n > 0; --n) {
                            const auto k = key(random);
                            switch (operation(random)) {
                            case 0:
                                worker.run(stringFormat("CREATE (:Item{} {id: {}, writer: {}});",
                                    round, k, worker.index()));
                                break;
                            case 1:
                                worker.run(stringFormat(
                                    "MATCH (n:Item{} {id: {}}) DETACH DELETE n;", round, k));
                                break;
                            case 2: {
                                const auto other = key(random);
                                worker.run(stringFormat(
                                    "MATCH (a:Item{} {id: {}}), (b:Item{} {id: {}}) "
                                    "CREATE (a)-[:Link{} {src_id: {}, dst_id: {}}]->(b);",
                                    round, k, round, other, round, k, other));
                            } break;
                            default:
                                worker.run(
                                    stringFormat("MATCH (n:Item{} {id: {}}) SET n.writer = {};",
                                        round, k, worker.index()));
                                break;
                            }
                        }
                        if (random() % 8 == 0) {
                            worker.rollback();
                        } else {
                            worker.commit();
                        }
                    }
                    worker.sync();
                }
            },
        .expect = [](ConcurrencyBench&, const SharedArea&) {}});
}

// C8 — point de reprise sous écrivains. Trois écrivains insèrent des clés disjointes,
// chacune dans sa transaction, avec le point de reprise automatique allumé et un seuil
// bas ; le quatrième lance au plus trois CHECKPOINT pendant qu'ils écrivent.
// Invariant : chaque insertion acquittée est là, une fois et une seule ; les refus de
// point de reprise sont lus (délai d'attente). Attendu vert pour l'intégrité.
// Mesuré à l'étape 2, avec CHECKPOINT en boucle et 300 insertions par écrivain : 17
// points de reprise sur 24 expirent au délai du moteur (5 s), et chaque attente gèle les
// écrivains (94 à 189 s pour ce cas). Le délai se règle par TransactionManager::
// setCheckPointWaitTimeoutForTransactionsToLeaveInMicros, privé : décision de
// l'orchestration (3 octobre), le cas est borné à trois CHECKPOINT et 100 insertions,
// et l'accès au délai sera demandé quand la marche A7, dont ce gel est le sujet,
// s'ouvrira.
TEST_P(ConcurrencyBench, C8_CheckpointUnderWriters) {
    static constexpr uint32_t numWriters = 3;
    static constexpr uint32_t maxCheckpoints = 3;
    const auto iterations = benchIterations(100);
    runCase({.numWorkers = numWriters + 1,
        .setup =
            [](ConcurrencyBench& bench) {
                bench.mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
                bench.mustRun("CALL checkpoint_threshold=65536;");
            },
        .scenario =
            [iterations](Worker& worker) {
                if (worker.index() == numWriters) {
                    for (auto attempt = 0u;
                        attempt < maxCheckpoints && worker.shared().flag.load() < numWriters;
                        ++attempt) {
                        worker.run("CHECKPOINT;");
                        std::this_thread::sleep_for(std::chrono::milliseconds(2));
                    }
                    return;
                }
                for (auto i = 0u; i < iterations; ++i) {
                    worker.run(stringFormat("CREATE (:Item {id: {}, writer: {}});",
                        worker.index() * 100000 + i, worker.index()));
                }
                worker.shared().flag.fetch_add(1);
            },
        .expect =
            [](ConcurrencyBench& bench, const SharedArea& area) {
                uint32_t inserted = 0;
                for (auto i = 0u; i < numWriters; ++i) {
                    inserted += area.workers[i].statementsSucceeded.load();
                }
                EXPECT_EQ(bench.queryInt("MATCH (n:Item) RETURN count(n);"), inserted)
                    << "[check: inserted-count] "
                    << "every acknowledged insert must be there, once";
            },
        .autoCheckpoint = true});
}

// C9 — le contrat inter-processus d'aujourd'hui : deux processus ouvrent la base en
// écriture, un seul y parvient, l'autre est refusé par le verrou de fichier. C'est le
// cas qu'on retournera avec B.
TEST_P(ConcurrencyBench, C9_SecondWriterProcessRefused) {
    runCase({.numWorkers = 2,
        .setup =
            [](ConcurrencyBench& bench) {
                bench.mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
            },
        .scenario = [](Worker& worker) { worker.sync(); },
        .expect =
            [](ConcurrencyBench&, const SharedArea& area) {
                EXPECT_EQ(totalRefusals(area, Refusal::FileLock), 1u)
                    << "[check: one-file-lock-refusal] "
                    << "exactly one of the two writer processes must be refused";
            },
        .runsInProcesses = true,
        .onlyInProcesses = true});
}

// ── Les cas sur une table indexée par HNSW ─────────────────────────────────────────────
// Demandés par l'étude de la session cœur C++
// (docs/3-octobre-2026-15h47/02-hnsw-sous-plusieurs-ecrivains.md, §5). La table Doc porte
// un vecteur de 8 composantes et l'index doc_index ; chaque vecteur est une fonction de
// l'identifiant, pour que tout cas se rejoue à l'identique. Le vérificateur ajoute
// l'invariant de l'index (checkVectorIndexes) : une recherche exhaustive rend exactement
// les lignes vivantes qui portent un vecteur.

static constexpr int64_t NUM_BASE_DOCS = 300;

// Le vecteur d'un document, en Cypher, à partir d'une variable entière.
static std::string docVector(const std::string& variable) {
    std::string out = "[";
    const std::array<int, 8> multipliers{37, 53, 71, 89, 97, 13, 29, 41};
    const std::array<int, 8> moduli{101, 103, 107, 109, 113, 127, 131, 137};
    for (auto i = 0u; i < multipliers.size(); ++i) {
        out += stringFormat("{}CAST(({} * {}) % {} AS FLOAT) / {}", i == 0 ? "" : ", ", variable,
            multipliers[i], moduli[i], moduli[i]);
    }
    return out + "]";
}

static std::string createDocs(int64_t first, int64_t last) {
    return stringFormat("UNWIND range({}, {}) AS i CREATE (:Doc {id: i, vec: {}});", first, last,
        docVector("i"));
}

static void createIndexedDocs(ConcurrencyBench& bench) {
    bench.mustRun("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[8]);");
    bench.mustRun(createDocs(0, NUM_BASE_DOCS - 1));
    bench.mustRun("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
}

// H1 — deux écrivains insèrent des vecteurs dans des transactions ouvertes ensemble,
// puis valident l'un après l'autre. Invariant : les deux valident, tous les vecteurs sont
// retrouvés par l'index. C'est le défaut 5 de l'étude : le moteur plantait au second
// COMMIT (SIGSEGV, offsets définitifs lus comme locaux). Vert depuis b23309848 (marche
// A2, « au commit, un offset définitif n'est plus pris pour une ligne locale »). Le cas
// reste isolé, pour qu'un plantage soit un rouge et non la fin de la passe.
TEST_P(ConcurrencyBench, H1_IndexedInsertsTogether) {
    runCase({.numWorkers = 2,
        .setup = createIndexedDocs,
        .scenario =
            [](Worker& worker) {
                const int64_t first = 1000 + worker.index() * 100;
                worker.begin();
                worker.writeInTurn(createDocs(first, first + 4));
                worker.commitInOrderByEvents({0, 1});
            },
        .expect =
            [](ConcurrencyBench& bench, const SharedArea& area) {
                EXPECT_EQ(totalCommits(area), 2u) << "[check: all-commit] ";
                EXPECT_EQ(bench.queryInt("MATCH (n:Doc) RETURN count(n);"), NUM_BASE_DOCS + 10)
                    << "[check: row-count] ";
            },
        .vectorExtension = true,
        .isolated = true});
}

// H2 — un écrivain supprime des documents pendant qu'un autre en insère, dans les deux
// ordres de commit. Les lignes sont disjointes : les deux doivent valider. Invariant : les
// deux valident, l'index rend exactement les lignes vivantes.
static BenchCase indexedDeleteVersusInsert(std::vector<uint32_t> commitOrder) {
    return {.numWorkers = 2,
        .setup = createIndexedDocs,
        .scenario =
            [commitOrder](Worker& worker) {
                worker.begin();
                if (worker.index() == 0) {
                    worker.writeInTurn("MATCH (n:Doc) WHERE n.id < 10 DELETE n;");
                } else {
                    worker.writeInTurn(createDocs(1000, 1009));
                }
                worker.commitInOrderByEvents(commitOrder);
            },
        .expect =
            [](ConcurrencyBench& bench, const SharedArea& area) {
                EXPECT_EQ(totalCommits(area), 2u) << "[check: all-commit] ";
                EXPECT_EQ(bench.queryInt("MATCH (n:Doc) RETURN count(n);"), NUM_BASE_DOCS)
                    << "[check: row-count] ";
            },
        .vectorExtension = true,
        .isolated = true};
}

TEST_P(ConcurrencyBench, H2_IndexedDeleteCommitsFirst) {
    runCase(indexedDeleteVersusInsert({0, 1}));
}

TEST_P(ConcurrencyBench, H2_IndexedInsertCommitsFirst) {
    runCase(indexedDeleteVersusInsert({1, 0}));
}

// H5 — deux écrivains insèrent chacun cinq documents placés tout près du même document (le 50),
// dans des transactions ouvertes ensemble, puis valident dans l'ordre donné : leurs nouveaux
// nœuds ont des voisins communs, dont l'index réécrit les listes pour l'un et pour l'autre. Sous
// A4′, ces écritures internes de l'index ne prennent pas de verrou (takesLocks = false) et l'index
// est tenu en partagé par les deux inséreurs : rien ne les sépare. Invariant : les deux valident,
// et l'index rend exactement les lignes vivantes (le vérificateur d'intégrité). Ce que la marche
// « l'index au commit » devra garantir ; demandé par le cœur C++ le 10 octobre.
static BenchCase indexedInsertsSharingANeighbour(std::vector<uint32_t> commitOrder) {
    return {.numWorkers = 2,
        .setup = createIndexedDocs,
        .scenario =
            [commitOrder](Worker& worker) {
                const int64_t first = 1000 + worker.index() * 100;
                // Le vecteur du document 50, sa première composante décalée d'un cent-millième par
                // ligne : tout près de lui, et distincts entre eux.
                auto near = docVector("50");
                near.insert(near.find(','), " + CAST(i AS FLOAT) / 100000");
                worker.begin();
                worker.writeInTurn(stringFormat(
                    "UNWIND range({}, {}) AS i CREATE (:Doc {id: i, vec: {}});", first, first + 4,
                    near));
                worker.commitInOrderByEvents(commitOrder);
            },
        .expect =
            [](ConcurrencyBench& bench, const SharedArea& area) {
                EXPECT_EQ(totalCommits(area), 2u) << "[check: all-commit] ";
                EXPECT_EQ(bench.queryInt("MATCH (n:Doc) RETURN count(n);"), NUM_BASE_DOCS + 10)
                    << "[check: row-count] ";
            },
        .vectorExtension = true,
        .isolated = true};
}

TEST_P(ConcurrencyBench, H5_IndexedInsertsSharingANeighbourFirstCommitsFirst) {
    runCase(indexedInsertsSharingANeighbour({0, 1}));
}

TEST_P(ConcurrencyBench, H5_IndexedInsertsSharingANeighbourSecondCommitsFirst) {
    runCase(indexedInsertsSharingANeighbour({1, 0}));
}

// H3 — deux écrivains suppriment deux documents dont les voisinages dans le graphe de
// l'index se recouvrent (le cas que l'étude n'avait pas su tirer). Le recouvrement est
// construit en lisant les arêtes stockées de l'index (table interne
// _<table>_doc_index_LOWER) : on prend les deux nœuds, non voisins entre eux, qui
// partagent le plus de voisins. Supprimer un nœud réécrit les arêtes de ses voisins
// (cleanEdgesForNode) : les deux suppressions se rencontrent sur les mêmes arêtes, alors
// que l'utilisateur a touché deux lignes sans rapport. Invariant : les deux valident,
// l'index rend exactement les lignes vivantes. Rouge, et attendu tant que la suppression
// recoud les voisins pendant l'instruction : le second reçoit un « Write-write
// conflict » (vu le 3 octobre, 10 sur 10). L'étude propose de le régler en faisant toute
// la maintenance de l'index au commit (§4) ; ce cas en sera le test.
TEST_P(ConcurrencyBench, H3_IndexedDeletesWithOverlappingNeighbourhoods) {
    auto chosen = std::make_shared<std::pair<int64_t, int64_t>>(-1, -1);
    runCase({.numWorkers = 2,
        .setup =
            [chosen](ConcurrencyBench& bench) {
                createIndexedDocs(bench);
                const auto tableID =
                    bench.queryInt("CALL show_tables() WHERE name = 'Doc' RETURN id;");
                const auto adjacency = integrity::storedForwardAdjacency(*bench.conn,
                    stringFormat("_{}_doc_index_LOWER", tableID));
                // Les voisins d'un nœud : ses arêtes sortantes et entrantes.
                std::map<uint64_t, std::set<uint64_t>> neighbours;
                for (const auto& [source, destinations] : adjacency) {
                    for (const auto destination : destinations) {
                        neighbours[source].insert(destination);
                        neighbours[destination].insert(source);
                    }
                }
                size_t best = 0;
                std::pair<uint64_t, uint64_t> pair{0, 0};
                for (const auto& [a, aNeighbours] : neighbours) {
                    for (const auto& [b, bNeighbours] : neighbours) {
                        if (b <= a || aNeighbours.contains(b)) {
                            continue;
                        }
                        size_t shared = 0;
                        for (const auto n : aNeighbours) {
                            shared += bNeighbours.contains(n) ? 1 : 0;
                        }
                        if (shared > best) {
                            best = shared;
                            pair = {a, b};
                        }
                    }
                }
                ASSERT_GT(best, 0u) << "no two nodes share a neighbour in the index graph";
                const auto idAt = [&](uint64_t offset) {
                    return bench.queryInt(stringFormat(
                        "MATCH (n:Doc) WHERE offset(id(n)) = {} RETURN n.id;", offset));
                };
                *chosen = {idAt(pair.first), idAt(pair.second)};
                std::cerr << "  -- chosen documents " << chosen->first << " and " << chosen->second
                          << ", " << best << " shared neighbours\n";
            },
        .scenario =
            [chosen](Worker& worker) {
                const auto id = worker.index() == 0 ? chosen->first : chosen->second;
                worker.begin();
                worker.writeInTurn(stringFormat("MATCH (n:Doc {id: {}}) DELETE n;", id));
                worker.commitInOrderByEvents({0, 1});
            },
        .expect =
            [](ConcurrencyBench& bench, const SharedArea& area) {
                EXPECT_EQ(totalCommits(area), 2u)
                    << "[check: all-commit] two deletes of unrelated rows must both commit";
                EXPECT_EQ(bench.queryInt("MATCH (n:Doc) RETURN count(n);"), NUM_BASE_DOCS - 2)
                    << "[check: row-count] ";
            },
        .vectorExtension = true,
        .isolated = true});
}

// H4 — le mélange de C7 sur une table indexée : insertions, suppressions, nouveaux
// vecteurs, sur un petit domaine de clés. Probabiliste par construction
// (probabilistic.txt), et joué aussi sous ThreadSanitizer (exploration rapportée, non
// comparée). Isolé : la course du chemin de suppression peut faire planter le processus.
TEST_P(ConcurrencyBench, H4_IndexedRandomMix) {
    static constexpr int64_t numKeys = 40;
    const auto iterations = benchIterations(60);
    runCase({.numWorkers = 4,
        .setup = createIndexedDocs,
        .scenario =
            [iterations](Worker& worker) {
                std::mt19937_64 random(benchSeed() + 200 + worker.index());
                std::uniform_int_distribution<int64_t> key(0, numKeys - 1);
                std::uniform_int_distribution<int> operation(0, 2);
                for (auto i = 0u; i < iterations; ++i) {
                    const auto k = 2000 + key(random);
                    worker.begin();
                    switch (operation(random)) {
                    case 0:
                        worker.run(createDocs(k, k));
                        break;
                    case 1:
                        worker.run(stringFormat("MATCH (n:Doc {id: {}}) DELETE n;", k));
                        break;
                    default:
                        worker.run(stringFormat("MATCH (n:Doc {id: {}}) SET n.vec = {};", k,
                            docVector(std::to_string(k + 7))));
                        break;
                    }
                    if (random() % 8 == 0) {
                        worker.rollback();
                    } else {
                        worker.commit();
                    }
                }
            },
        .expect = [](ConcurrencyBench&, const SharedArea&) {},
        .vectorExtension = true,
        .isolated = true});
}

INSTANTIATE_TEST_SUITE_P(Launchers, ConcurrencyBench,
    ::testing::Values(BenchParam{LaunchMode::Thread, Check::Hot},
        BenchParam{LaunchMode::Thread, Check::Reopen}, BenchParam{LaunchMode::Thread, Check::Crash},
        BenchParam{LaunchMode::Process, Check::Hot}),
    paramName);

#endif
