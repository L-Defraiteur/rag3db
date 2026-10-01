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
// - auto_checkpoint=false : un point de reprise déclenché par un commit attend que
//   toutes les autres transactions soient sorties et expire au bout de 5 s ; il
//   mesurerait le point de reprise, pas les conflits. Les points de reprise sont
//   explicites, dans les cas qui les veulent (C8, étape 2).
//
// Les cas déterministes tiennent la fenêtre de course ouverte par des transactions
// explicites et une barrière : BEGIN, écriture, barrière, puis les commits l'un
// après l'autre dans un ordre fixé. Aucun crochet dans le moteur.

#ifndef __SINGLE_THREADED__

#include <filesystem>
#include <iostream>

#include "bench_harness.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "integrity/integrity_checker.h"

using namespace rag3db::common;
using namespace rag3db::testing;
using namespace rag3db::testing::concurrency;

static std::vector<uint32_t> ascending(uint32_t numWorkers) {
    std::vector<uint32_t> order;
    for (auto i = 0u; i < numWorkers; ++i) {
        order.push_back(i);
    }
    return order;
}

class ConcurrencyBench : public EmptyDBTest, public ::testing::WithParamInterface<LaunchMode> {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "the bench needs an on-disk database (checkpoint, reopen)";
        }
        systemConfig->maxDBSize = 1024ull * 1024 * 1024 * 1024;
        systemConfig->bufferPoolSize = 1024 * 1024 * 1024;
        createDBAndConn();
        mustRun("CALL debug_enable_multi_writes=true;");
        mustRun("CALL auto_checkpoint=false;");
    }

    void TearDown() override {
        const auto testDirectory = std::filesystem::path(databasePath).parent_path();
        EmptyDBTest::TearDown();
        // Le nom d'un test paramétré contient des « / » : le chemin de la base prend des
        // répertoires intermédiaires que le nettoyage de base ne retire pas.
        std::error_code ignored;
        for (auto dir = testDirectory.parent_path();
            dir != TestHelper::getRootTempDir() && dir.has_parent_path(); dir = dir.parent_path()) {
            if (!std::filesystem::remove(dir, ignored)) {
                break;
            }
        }
    }

    void mustRun(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << "\n" << result->getErrorMessage();
    }

    int64_t queryInt(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << "\n" << result->getErrorMessage();
        if (!result->isSuccess() || !result->hasNext()) {
            return -1;
        }
        return result->getNext()->getValue(0)->getValue<int64_t>();
    }

    // Lance le scénario, écrit le compte rendu brut, puis passe le vérificateur sur la
    // base encore ouverte, après la fin des écrivains (jamais depuis un lecteur
    // concurrent : la course du lecteur d'un autre processus n'est pas corrigée sur
    // master, journal des chantiers §6).
    std::vector<integrity::Violation> runScenario(uint32_t numWorkers, const Scenario& scenario) {
        SharedMapping mapping(numWorkers);
        auto& area = mapping.get();
        launch(GetParam(), *database, area, scenario);
        const auto violations = integrity::checkLevel1(*conn);
        std::cerr << "[bench] " << ::testing::UnitTest::GetInstance()->current_test_info()->name()
                  << " (" << launchModeName(GetParam()) << ")\n"
                  << describe(area) << integrity::describe(violations);
        EXPECT_EQ(totalRefusals(area, Refusal::Unexpected), 0u) << "an unexpected error";
        EXPECT_EQ(area.barrierTimedOut.load(), 0u) << "a writer never reached the barrier";
        lastCommits = totalCommits(area);
        return violations;
    }

    // C1 : un même nœud de clé 7 créé par chaque écrivain (commentaire du cas, plus bas).
    std::vector<integrity::Violation> samePrimaryKey(uint32_t numWorkers) {
        return runScenario(numWorkers, [&](Worker& worker) {
            worker.begin();
            worker.run(stringFormat("CREATE (:Item {id: 7, writer: {}});", worker.index()));
            worker.sync();
            worker.commitInOrder(ascending(worker.numWorkers()));
        });
    }

    // C2 : suppression du nœud 1 contre relation 1 -> 2 (commentaire du cas, plus bas).
    std::vector<integrity::Violation> deleteVersusNewRelation(
        const std::vector<uint32_t>& commitOrder) {
        return runScenario(2, [&](Worker& worker) {
            worker.begin();
            if (worker.index() == 0) {
                worker.run("MATCH (a:Item {id: 1}) DELETE a;");
            } else {
                worker.run("MATCH (a:Item {id: 1}), (b:Item {id: 2}) "
                           "CREATE (a)-[:Link {src_id: 1, dst_id: 2}]->(b);");
            }
            worker.sync();
            worker.commitInOrder(commitOrder);
        });
    }

    uint32_t lastCommits = 0;
};

// C0 — témoin. Clés disjointes : chaque écrivain crée ses propres nœuds, chacun dans
// sa transaction, puis des relations entre ses nœuds déjà validés. Doit être vert :
// il prouve que le banc et le vérificateur ne crient pas sans raison.
TEST_P(ConcurrencyBench, C0_DisjointKeysWitness) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
    mustRun("CREATE REL TABLE Link(FROM Item TO Item, src_id INT64, dst_id INT64);");
    constexpr uint32_t numWorkers = 4;
    constexpr int64_t numNodes = 50;
    const auto violations = runScenario(numWorkers, [&](Worker& worker) {
        const int64_t base = worker.index() * 1000;
        for (auto i = 0; i < numNodes; ++i) {
            worker.run(
                stringFormat("CREATE (:Item {id: {}, writer: {}});", base + i, worker.index()));
        }
        worker.sync();
        for (auto i = 0; i + 1 < numNodes; ++i) {
            worker.run(stringFormat("MATCH (a:Item {id: {}}), (b:Item {id: {}}) "
                                    "CREATE (a)-[:Link {src_id: {}, dst_id: {}}]->(b);",
                base + i, base + i + 1, base + i, base + i + 1));
        }
    });
    EXPECT_TRUE(violations.empty());
    EXPECT_EQ(queryInt("MATCH (n:Item) RETURN count(n);"), numWorkers * numNodes);
    EXPECT_EQ(queryInt("MATCH ()-[r:Link]->() RETURN count(r);"), numWorkers * (numNodes - 1));
}

// C1 — même clé primaire. Chaque écrivain ouvre une transaction, crée le nœud de clé
// 7, attend les autres, puis tous valident l'un après l'autre. Invariant : un seul
// commit réussit, une seule ligne de clé 7. Attendu aujourd'hui (déduit) : rouge, la
// clé non validée de l'autre transaction est invisible au contrôle d'unicité
// (node_table.cpp, validatePkNotExists).
TEST_P(ConcurrencyBench, C1_SamePrimaryKeyTwoWriters) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
    const auto violations = samePrimaryKey(2);
    EXPECT_TRUE(violations.empty());
    EXPECT_EQ(lastCommits, 1u) << "exactly one writer of key 7 may commit";
    EXPECT_EQ(queryInt("MATCH (n:Item) WHERE n.id = 7 RETURN count(n);"), 1);
}

TEST_P(ConcurrencyBench, C1_SamePrimaryKeyThreeWriters) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
    const auto violations = samePrimaryKey(3);
    EXPECT_TRUE(violations.empty());
    EXPECT_EQ(lastCommits, 1u) << "exactly one writer of key 7 may commit";
    EXPECT_EQ(queryInt("MATCH (n:Item) WHERE n.id = 7 RETURN count(n);"), 1);
}

// C2 — relation vers un nœud supprimé. Les nœuds 1 et 2 sont validés avant. L'écrivain
// 0 supprime le nœud 1 ; l'écrivain 1 crée une relation 1 -> 2. Barrière, puis les deux
// commits dans l'ordre donné. Invariant : jamais les deux à la fois — soit le nœud est
// supprimé et il n'y a pas de relation, soit la relation existe et le nœud aussi.
// Attendu aujourd'hui (déduit) : rouge dans les deux ordres, chacun ne voit que son
// instantané (delete_executor.cpp, throwIfNodeHasRels).
TEST_P(ConcurrencyBench, C2_DeleteCommitsFirst) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
    mustRun("CREATE REL TABLE Link(FROM Item TO Item, src_id INT64, dst_id INT64);");
    mustRun("CREATE (:Item {id: 1, writer: -1}), (:Item {id: 2, writer: -1});");
    const auto violations = deleteVersusNewRelation({0, 1});
    EXPECT_TRUE(violations.empty());
    EXPECT_EQ(lastCommits, 1u) << "the delete and the new relation cannot both commit";
}

TEST_P(ConcurrencyBench, C2_RelationCommitsFirst) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
    mustRun("CREATE REL TABLE Link(FROM Item TO Item, src_id INT64, dst_id INT64);");
    mustRun("CREATE (:Item {id: 1, writer: -1}), (:Item {id: 2, writer: -1});");
    const auto violations = deleteVersusNewRelation({1, 0});
    EXPECT_TRUE(violations.empty());
    EXPECT_EQ(lastCommits, 1u) << "the delete and the new relation cannot both commit";
}

// C3 — offsets locaux qui se chevauchent (marche A2). Chaque écrivain, dans une seule
// transaction, crée ses nœuds puis une chaîne de relations entre eux ; chaque relation
// porte les clés de ses extrémités. Les deux transactions numérotent leurs nœuds à
// partir de la même taille de table. Invariant : chaque relation relie les nœuds dont
// elle porte les clés. Attendu aujourd'hui (déduit) : rouge, sans erreur — les
// relations du second à valider pointent vers les nœuds du premier (pas de
// remappage des offsets au commit).
TEST_P(ConcurrencyBench, C3_OverlappingLocalOffsets) {
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
    mustRun("CREATE REL TABLE Link(FROM Item TO Item, src_id INT64, dst_id INT64);");
    constexpr uint32_t numWorkers = 2;
    constexpr int64_t numNodes = 8;
    const auto violations = runScenario(numWorkers, [&](Worker& worker) {
        const int64_t base = (worker.index() + 1) * 100;
        worker.begin();
        for (auto i = 0; i < numNodes; ++i) {
            worker.run(
                stringFormat("CREATE (:Item {id: {}, writer: {}});", base + i, worker.index()));
        }
        for (auto i = 0; i + 1 < numNodes; ++i) {
            worker.run(stringFormat("MATCH (a:Item {id: {}}), (b:Item {id: {}}) "
                                    "CREATE (a)-[:Link {src_id: {}, dst_id: {}}]->(b);",
                base + i, base + i + 1, base + i, base + i + 1));
        }
        worker.sync();
        worker.commitInOrder(ascending(worker.numWorkers()));
    });
    EXPECT_TRUE(violations.empty());
    EXPECT_EQ(lastCommits, numWorkers);
    EXPECT_EQ(queryInt("MATCH (n:Item) RETURN count(n);"), numWorkers * numNodes);
    EXPECT_EQ(queryInt("MATCH ()-[r:Link]->() RETURN count(r);"), numWorkers * (numNodes - 1));
}

INSTANTIATE_TEST_SUITE_P(Launchers, ConcurrencyBench, ::testing::Values(LaunchMode::Thread),
    [](const ::testing::TestParamInfo<LaunchMode>& info) { return launchModeName(info.param); });

#endif
