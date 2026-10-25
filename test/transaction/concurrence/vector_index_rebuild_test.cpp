// L'index vectoriel retiré puis recréé pendant que des recherches tournent en face (exploration du
// banc, 12 octobre 2026). C'est le chemin de rag3weaver `ajuster_l_index_pour_le_retard` : pendant un
// long rattrapage d'embarquement, l'index tombe (DROP_VECTOR_INDEX), les vecteurs se posent sans
// lui, puis `rebuild_vector_index` le recrée (CREATE_VECTOR_INDEX). L'hypothèse à éprouver venait
// du gel de l'hôte de luciepc : un DROP/CREATE contre une recherche ouverte en face, sous le mode
// multi-écrivains par défaut.
//
// Une connexion répète le cycle ; l'autre, dans un fil, cherche en boucle. Chaque recherche est
// soit cohérente (le vecteur cherché revient premier), soit refusée par un nom attendu (l'index
// n'existe pas, le temps du rattrapage) ; aucune autre erreur, et aucune recherche ne doit tenir
// plus de quelques secondes (un figement).

#include <atomic>
#include <chrono>
#include <future>
#include <iostream>
#include <map>
#include <string>
#include <thread>

#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "bench_harness.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr int64_t NUM_ROWS = 2000;
constexpr auto RUN_FOR = std::chrono::seconds(6);
constexpr double SLOW_QUERY_MS = 3000;

class VectorIndexRebuild : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        concurrency::loadVectorExtension(*conn);
        ok(*conn, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
        ok(*conn, stringFormat("UNWIND range(0, {}) AS i CREATE (:Doc {id: i, vec: [i % 17, i % 23, "
                               "i % 29, i]});",
                      NUM_ROWS - 1));
        ok(*conn, "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    }

    static void ok(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    // Un refus attendu pendant le CREATE concurrent : l'index est là, en construction.
    static bool namedConstruction(const std::string& message) {
        return message.find("is under construction") != std::string::npos;
    }

    // Un refus attendu pendant le rattrapage : l'index n'est pas là.
    static bool namedAbsence(const std::string& message) {
        for (const auto* fragment :
            {"does not exist", "doesn't exist", "not found", "no index", "doesn't have an index"}) {
            if (message.find(fragment) != std::string::npos) {
                return true;
            }
        }
        return false;
    }
};

TEST_F(VectorIndexRebuild, SearchesAcrossDropAndCreateAreConsistentOrNamed) {
    rag3db::main::Connection rebuilder(database.get());
    rag3db::main::Connection searcher(database.get());
    std::atomic<bool> stop{false};
    std::map<std::string, int64_t> outcomes;
    int64_t consistent = 0;
    int64_t inconsistent = 0;
    double slowestMs = 0;
    std::string firstInconsistent;
    auto searching = std::async(std::launch::async, [&] {
        while (!stop.load()) {
            const auto start = std::chrono::steady_clock::now();
            auto result = searcher.query(
                "CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [7, 7, 7, 1000], 5) RETURN node.id, "
                "distance;");
            const auto ms = std::chrono::duration<double, std::milli>(
                std::chrono::steady_clock::now() - start)
                                .count();
            slowestMs = std::max(slowestMs, ms);
            if (!result->isSuccess()) {
                const auto message = result->getErrorMessage();
                // Depuis le CREATE concurrent ((A)), l'index existe dès T_A et la recherche le refuse par
                // le nom de son état, en construction, jusqu'à la fin du bâti.
                outcomes[namedAbsence(message)       ? "named absence" :
                         namedConstruction(message) ? "named construction" :
                                                      "OTHER: " + message.substr(0, 160)]++;
                continue;
            }
            // Le plus proche de [7, 7, 7, 1000] est la ligne 1000 : [1000 % 17, 1000 % 23,
            // 1000 % 29, 1000] = [14, 11, 14, 1000], ou une voisine de même quatrième composante.
            int64_t best = -1;
            double bestDistance = 1e30;
            while (result->hasNext()) {
                auto tuple = result->getNext();
                const auto distance = std::stod(tuple->getValue(1)->toString());
                if (distance < bestDistance) {
                    bestDistance = distance;
                    best = tuple->getValue(0)->getValue<int64_t>();
                }
            }
            if (best >= 995 && best <= 1005) {
                consistent++;
            } else {
                inconsistent++;
                if (firstInconsistent.empty()) {
                    firstInconsistent = stringFormat("best {} at {}", best, bestDistance);
                }
            }
        }
    });
    int64_t cycles = 0;
    std::map<std::string, int64_t> rebuilderErrors;
    const auto deadline = std::chrono::steady_clock::now() + RUN_FOR;
    while (std::chrono::steady_clock::now() < deadline) {
        for (const auto* step :
            {"CALL DROP_VECTOR_INDEX('Doc', 'doc_index');",
                "MATCH (n:Doc) WHERE n.id % 50 = 0 SET n.vec = [n.id % 17, n.id % 23, n.id % 29, "
                "n.id];",
                "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');"}) {
            auto result = rebuilder.query(step);
            if (!result->isSuccess()) {
                rebuilderErrors[std::string(step).substr(0, 30) + " : " +
                                result->getErrorMessage().substr(0, 140)]++;
            }
        }
        cycles++;
    }
    stop.store(true);
    const auto finished = searching.wait_for(std::chrono::seconds(30));
    ASSERT_EQ(finished, std::future_status::ready)
        << "[check: no-hang] the searching thread did not stop within 30 s";
    std::cerr << "  [reconstruction] cycles " << cycles << ", recherches cohérentes " << consistent
              << ", incohérentes " << inconsistent << ", la plus lente " << slowestMs << " ms\n";
    for (const auto& [what, count] : outcomes) {
        std::cerr << "    recherche " << count << " × " << what << "\n";
    }
    for (const auto& [what, count] : rebuilderErrors) {
        std::cerr << "    reconstruction " << count << " × " << what << "\n";
    }
    int64_t other = 0;
    for (const auto& [what, count] : outcomes) {
        if (what.rfind("OTHER", 0) == 0) {
            other += count;
        }
    }
    EXPECT_EQ(other, 0) << "[check: no-unnamed-search-error] ";
    EXPECT_TRUE(rebuilderErrors.empty()) << "[check: rebuild-succeeds] ";
    EXPECT_EQ(inconsistent, 0) << "[check: consistent-results] " << firstInconsistent;
    EXPECT_LT(slowestMs, SLOW_QUERY_MS) << "[check: no-stuck-search] ";
    EXPECT_GT(consistent, 0) << "[check: some-searches-ran] ";
}

} // namespace
