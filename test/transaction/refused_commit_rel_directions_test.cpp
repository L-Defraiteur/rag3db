// Les deux sens d'une table de relations s'accordent après l'annulation d'un COMMIT refusé (ticket
// docs/optimiseur/tickets/2026-11-14-code-lourd-et-code-principal-divergent-entre-leurs-deux-rangements.md).
// Sur code-lourd et code-principal, sous la lib 0c41f9fe6, trois tables de relations rendaient des
// relations d'un seul côté, vues deux fois, ou aux bouts différents selon le sens lu. La piste : la
// fenêtre corrigée en 373b5477b. Un COMMIT refusé s'annulait hors du verrou, et l'annulation tronquait
// chaque sens de la table à sa première ligne, emportant les lignes qu'un autre écrivain avait versées
// entre-temps — sens par sens, chacun à ses propres positions.
//
// A verse un lot de relations, son COMMIT est refusé (le journal ne se synchronise pas,
// CutFileSystem) ; B valide une relation pendant que A se déroule ; puis chaque relation validée de B
// est lue par sa source (sens avant) et par sa destination (sens arrière). Les deux comptes doivent
// valoir 1, et l'ensemble des deux sens être le même.
//
// Les bouts sont dans des groupes de nœuds différents (2 048 nœuds par groupe), comme dans une vraie
// base : les relations de A vont du groupe 0 au groupe 2, celle de B du groupe 0 au groupe 1. Au sens
// avant, A et B partagent le groupe CSR 0, et la troncature de A emportait B ; au sens arrière, B est
// seule dans le groupe 1 et survivait : une relation d'un seul côté, la forme de code-lourd.
#include <sys/wait.h>
#include <unistd.h>

#include <chrono>
#include <filesystem>
#include <fstream>
#include <memory>
#include <string>
#include <thread>

#include "common/file_system/virtual_file_system.h"
#include "common/test_hooks.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "test_helper/cut_file_system.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class RefusedCommitRelDirections : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        std::filesystem::create_directories(std::filesystem::path(databasePath).parent_path());
    }
};

int64_t countOf(Connection& c, const std::string& q) {
    auto r = c.query(q);
    return r->isSuccess() && r->hasNext() ? r->getNext()->getValue(0)->getValue<int64_t>() : -1;
}

// Le cas joué dans un fils sur CutFileSystem ; écrit ses pas et rend le nombre de relations de B dont
// les deux sens ne s'accordent pas.
int play(const std::string& databasePath, rag3db::main::SystemConfig& config, std::ofstream& steps) {
    Database database(databasePath, config);
    Connection a(&database);
    Connection b(&database);
    for (const auto* q : {"CREATE NODE TABLE N(id INT64, PRIMARY KEY(id));",
             "CREATE REL TABLE R(FROM N TO N, line INT64);",
             "UNWIND range(0, 5999) AS i CREATE (:N {id: i});",
             // Des relations validées avant, dans les trois groupes : des groupes CSR déjà là, que les
             // lots prolongent.
             "UNWIND range(0, 2999) AS i MATCH (x:N {id: i}), (y:N {id: 5999 - i}) CREATE "
             "(x)-[:R {line: -1}]->(y);"}) {
        if (!a.query(q)->isSuccess()) {
            _exit(11);
        }
    }
    int disagreements = 0, refused = 0, bCommits = 0;
    for (auto round = 0; round < 8; round++) {
        a.query("BEGIN TRANSACTION;");
        a.query("UNWIND range(0, 4999) AS i MATCH (x:N {id: i % 2048}), (y:N {id: 4096 + i % 1900}) "
                "CREATE (x)-[:R {line: 100000 + i}]->(y);");
        const auto src = (round * 37) % 2048, dst = 2048 + round;
        const auto line = 1000 + round;
        b.query("BEGIN TRANSACTION;");
        b.query("MATCH (x:N {id: " + std::to_string(src) + "}), (y:N {id: " + std::to_string(dst) +
                "}) CREATE (x)-[:R {line: " + std::to_string(line) + "}]->(y);");
        auto failures = std::make_shared<int>(1);
        CutFileSystem::failSyncOf() = [failures](const std::string& path) {
            return path.ends_with(".wal") && (*failures)-- > 0;
        };
        bool aRefused = false;
        std::thread ta([&] { aRefused = !a.query("COMMIT;")->isSuccess(); });
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
        const bool bCommitted = b.query("COMMIT;")->isSuccess();
        ta.join();
        CutFileSystem::failSyncOf() = nullptr;
        refused += aRefused;
        bCommits += bCommitted;
        if (!bCommitted) {
            continue;
        }
        const auto l = std::to_string(line);
        // Par la source : le sens avant ; par la destination : le sens arrière.
        const auto forward = countOf(b, "MATCH (x:N {id: " + std::to_string(src) +
                                            "})-[r:R]->(y:N) WHERE r.line = " + l +
                                            " RETURN count(*);");
        const auto backward = countOf(b, "MATCH (x:N)-[r:R]->(y:N {id: " + std::to_string(dst) +
                                             "}) WHERE r.line = " + l + " RETURN count(*);");
        if (forward != 1 || backward != 1) {
            steps << "ROUND " << round << " A " << (aRefused ? "refused" : "accepted")
                  << " : B's relation by its source " << forward << ", by its destination "
                  << backward << std::endl;
            disagreements++;
        }
    }
    // Toute la table, d'un sens et de l'autre : les mêmes relations.
    const auto all = countOf(b, "MATCH ()-[r:R]->() RETURN count(r);");
    const auto bySources = countOf(b, "MATCH (x:N)-[r:R]->() RETURN count(r);");
    const auto byDestinations = countOf(b, "MATCH ()-[r:R]->(y:N) RETURN count(r);");
    steps << "REFUSED-A " << refused << std::endl;
    steps << "B-COMMITS " << bCommits << std::endl;
    steps << "TABLE " << all << " " << bySources << " " << byDestinations << std::endl;
    steps << "DISAGREE " << disagreements << std::endl;
    return disagreements;
}

TEST_F(RefusedCommitRelDirections, ACommitRefusedBetweenItsDirectionsLeavesBothDirectionsWhole) {
    const auto stepsPath = databasePath + ".steps";
    std::filesystem::remove(stepsPath);
    const auto pid = fork();
    if (pid == 0) {
        std::ofstream steps(stepsPath);
        playBetweenDirections(databasePath, *systemConfig, steps);
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
        << "[check: setup] A's commit was never refused between its directions\n"
        << steps;
    EXPECT_NE(steps.find("WRONG 0\n"), std::string::npos)
        << "[check: a-commit-refused-between-its-directions-leaves-both-directions-whole]\n"
        << steps;
}

TEST_F(RefusedCommitRelDirections, BothDirectionsKeepTheRelationsOfAnotherWriter) {
    const auto stepsPath = databasePath + ".steps";
    std::filesystem::remove(stepsPath);
    const auto pid = fork();
    if (pid == 0) {
        VirtualFileSystem::setDefaultFileSystemFactoryForTesting(
            [](const std::string& home) { return std::make_unique<CutFileSystem>(home); });
        std::ofstream steps(stepsPath);
        play(databasePath, *systemConfig, steps);
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
    EXPECT_NE(steps.find("DISAGREE 0\n"), std::string::npos)
        << "[check: both-directions-keep-the-relations-of-another-writer]\n"
        << steps;
}

// Le second cas : le COMMIT de A lève ENTRE ses deux sens (le point de faute
// rel-commit-between-directions), comme un tampon plein au second sens. Sous l'ancienne lib,
// l'enregistrement d'annulation ne couvrait que le premier sens versé, et l'annulation hors du verrou
// tronquait ce sens, avec la relation de B versée à la suite ; l'autre sens gardait B : une relation
// d'un seul côté, la forme de code-lourd. Sur master, l'annulation sous le verrou rend les deux sens
// à leur état d'avant : la relation de B lisible des deux côtés, et rien de A d'aucun côté.
int playBetweenDirections(const std::string& databasePath, rag3db::main::SystemConfig& config,
    std::ofstream& steps) {
    Database database(databasePath, config);
    Connection a(&database);
    Connection b(&database);
    for (const auto* q : {"CREATE NODE TABLE N(id INT64, PRIMARY KEY(id));",
             "CREATE REL TABLE R(FROM N TO N, line INT64);",
             "UNWIND range(0, 5999) AS i CREATE (:N {id: i});",
             "UNWIND range(0, 2999) AS i MATCH (x:N {id: i}), (y:N {id: 5999 - i}) CREATE "
             "(x)-[:R {line: -1}]->(y);"}) {
        if (!a.query(q)->isSuccess()) {
            _exit(11);
        }
    }
    int wrong = 0, refused = 0, bCommits = 0;
    for (auto round = 0; round < 8; round++) {
        a.query("BEGIN TRANSACTION;");
        a.query("UNWIND range(0, 4999) AS i MATCH (x:N {id: i % 2048}), (y:N {id: 4096 + i % 1900}) "
                "CREATE (x)-[:R {line: 100000 + i}]->(y);");
        const auto src = (round * 37) % 2048, dst = 2048 + round;
        const auto line = 1000 + round;
        b.query("BEGIN TRANSACTION;");
        b.query("MATCH (x:N {id: " + std::to_string(src) + "}), (y:N {id: " + std::to_string(dst) +
                "}) CREATE (x)-[:R {line: " + std::to_string(line) + "}]->(y);");
        TestHooks::armFailPoint(TestFailPoint::REL_COMMIT_BETWEEN_DIRECTIONS);
        bool aRefused = false;
        std::thread ta([&] { aRefused = !a.query("COMMIT;")->isSuccess(); });
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
        const bool bCommitted = b.query("COMMIT;")->isSuccess();
        ta.join();
        TestHooks::armFailPoint(TestFailPoint::NONE);
        if (!aRefused || !bCommitted) {
            // Le point a pris B au lieu de A : le tour ne dit rien.
            steps << "ROUND " << round << " skipped : A " << (aRefused ? "refused" : "committed")
                  << ", B " << (bCommitted ? "committed" : "refused") << std::endl;
            continue;
        }
        refused++;
        bCommits++;
        const auto l = std::to_string(line);
        const auto forward = countOf(b, "MATCH (x:N {id: " + std::to_string(src) +
                                            "})-[r:R]->(y:N) WHERE r.line = " + l +
                                            " RETURN count(*);");
        const auto backward = countOf(b, "MATCH (x:N)-[r:R]->(y:N {id: " + std::to_string(dst) +
                                             "}) WHERE r.line = " + l + " RETURN count(*);");
        // Rien de A d'aucun côté : par une source du groupe 0 (sens avant), par une destination du
        // groupe 2 (sens arrière).
        const auto aForward = countOf(b,
            "MATCH (x:N {id: 5})-[r:R]->(y:N) WHERE r.line >= 100000 RETURN count(*);");
        const auto aBackward = countOf(b,
            "MATCH (x:N)-[r:R]->(y:N {id: 4100}) WHERE r.line >= 100000 RETURN count(*);");
        if (forward != 1 || backward != 1 || aForward != 0 || aBackward != 0) {
            steps << "ROUND " << round << " : B's relation by its source " << forward
                  << ", by its destination " << backward << " ; A's relations left by a source "
                  << aForward << ", by a destination " << aBackward << std::endl;
            wrong++;
        }
    }
    steps << "REFUSED-A " << refused << std::endl;
    steps << "B-COMMITS " << bCommits << std::endl;
    steps << "WRONG " << wrong << std::endl;
    return wrong;
}

} // namespace
