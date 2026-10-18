// Un bâti d'index prend tous les fils de travail (ticket
// 2026-10-18-un-bati-d-index-prend-tous-les-fils-et-affame-les-autres-connexions). Trouvé par le
// scénario des vingt écrivains, qui tournait sur les deux fils de la configuration des tests :
// pendant CREATE_VECTOR_INDEX, chaque acteur, lecteurs compris, attendait de 50 ms à environ 9,7 s,
// la phase parallèle du bâti. Le bâti limité à un fil, ou la base à 16 fils, ne fait plus attendre
// personne.
//
// - IndexCreationThreads.LeavesAThreadToReaders : une base à deux fils, 50 000 lignes de vecteurs
//   FLOAT[64] ; une connexion crée l'index, une autre lit une ligne par sa clé en boucle pendant le
//   bâti. Contrôle index-creation-leaves-a-thread-to-readers : aucune lecture n'attend plus de 2 s,
//   soit largement le point de reprise forcé de la fin (0,5 à 1,3 s dans le scénario).

#ifndef __SINGLE_THREADED__

#include <algorithm>
#include <atomic>
#include <chrono>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <random>
#include <string>
#include <thread>

#include "bench_harness.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr int64_t NUM_ROWS = 50'000;
constexpr int DIM = 64;
constexpr double MAX_READ_MS = 2'000;

class IndexCreationThreads : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        systemConfig->bufferPoolSize = 2ull * 1024 * 1024 * 1024;
        systemConfig->maxNumThreads = 2;
        createDBAndConn();
        concurrency::applyBenchSettings(*conn);
        concurrency::loadVectorExtension(*conn);
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess())
            << "[check: query] " << query.substr(0, 160) << " : " << result->getErrorMessage();
    }
};

TEST_F(IndexCreationThreads, LeavesAThreadToReaders) {
    ok(stringFormat("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[{}]);", DIM));
    const auto path = databasePath + ".docs.csv";
    {
        std::ofstream out(path);
        std::mt19937 engine(5);
        std::uniform_real_distribution<float> uniform(-1, 1);
        for (int64_t id = 0; id < NUM_ROWS; ++id) {
            out << id << ",\"[";
            for (auto d = 0; d < DIM; ++d) {
                out << (d ? "," : "") << uniform(engine);
            }
            out << "]\"\n";
        }
    }
    ok("COPY Doc FROM '" + path + "' (header=false);");
    std::filesystem::remove(path);
    ok("CHECKPOINT;");

    std::atomic<bool> done{false};
    double worstRead = 0;
    int64_t reads = 0;
    int64_t refused = 0;
    std::thread reader([&] {
        rag3db::main::Connection connection(database.get());
        std::mt19937 engine(9);
        std::uniform_int_distribution<int64_t> key(0, NUM_ROWS - 1);
        while (!done) {
            const auto start = std::chrono::steady_clock::now();
            auto result =
                connection.query(stringFormat("MATCH (d:Doc {id: {}}) RETURN d.id;", key(engine)));
            const auto ms =
                std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start)
                    .count();
            refused += !result->isSuccess();
            worstRead = std::max(worstRead, ms);
            reads++;
        }
    });
    const auto start = std::chrono::steady_clock::now();
    auto created =
        conn->query("CALL CREATE_VECTOR_INDEX('Doc', 'doc_vec', 'vec', metric := 'cosine');");
    const auto createMs =
        std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start).count();
    done = true;
    reader.join();
    std::cerr << "  [bâti et fils] 2 fils, " << NUM_ROWS << " lignes : CREATE " << createMs
              << " ms ; lecteur : " << reads << " lectures, " << refused
              << " refusées, la plus longue " << worstRead << " ms\n";
    ASSERT_TRUE(created->isSuccess()) << "[check: query] " << created->getErrorMessage();
    ASSERT_GE(createMs, 2 * MAX_READ_MS)
        << "[check: plan] the index was built in " << createMs << " ms: nothing is witnessed";
    EXPECT_LE(worstRead, MAX_READ_MS)
        << "[check: index-creation-leaves-a-thread-to-readers] a read waited " << worstRead
        << " ms during a " << createMs << " ms index creation on two threads";
}

} // namespace

#endif
