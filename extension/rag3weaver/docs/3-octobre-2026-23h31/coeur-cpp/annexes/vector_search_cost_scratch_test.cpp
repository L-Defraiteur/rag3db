// Brouillon de mesure, non livré : le coût de la recherche vectorielle seule, sur dix
// mille vecteurs, à un fil puis à quatre fils lecteurs. Les temps sortent sur stderr.

#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <thread>
#include <vector>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "test_helper/test_helper.h"

using namespace rag3db::testing;

namespace {

class VectorSearchCostScratch : public EmptyDBTest {
protected:
    static constexpr int64_t NUM_DOCS = 10000;

    static int64_t dim() {
        const auto value = std::getenv("COUT_DIM");
        return value ? std::atoll(value) : 8;
    }

    static std::string cypherVector(const std::string& variable) {
        std::string out = "[";
        for (int64_t x = 1; x <= dim(); ++x) {
            out += (x == 1 ? "" : ", ");
            out += "CAST((" + variable + " * " + std::to_string(x * 2 + 35) + ") % " +
                   std::to_string(x + 100) + " AS FLOAT) / " + std::to_string(x + 100);
        }
        return out + "]";
    }

    static std::string literalVector(int64_t id) {
        std::string out = "[";
        for (int64_t x = 1; x <= dim(); ++x) {
            char buffer[32];
            snprintf(buffer, sizeof(buffer), "%.9g",
                static_cast<double>((id * (x * 2 + 35)) % (x + 100)) / (x + 100));
            out += (x == 1 ? "" : ", ");
            out += buffer;
        }
        return out + "]";
    }

    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        ok(*conn, "LOAD EXTENSION '" +
                      TestHelper::appendRag3dbRootPath(
                          "extension/vector/build/libvector.rag3db_extension") +
                      "';");
        ok(*conn, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[" +
                      std::to_string(dim()) + "]);");
        ok(*conn, "UNWIND range(0, " + std::to_string(NUM_DOCS - 1) +
                      ") AS i CREATE (:Doc {id: i, vec: " + cypherVector("i") + "});");
        ok(*conn, "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
        if (std::getenv("COUT_CHECKPOINT")) {
            ok(*conn, "CHECKPOINT;");
        }
    }

    static void ok(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    double run(int numThreads, int searchesPerThread) {
        std::atomic<int64_t> failures{0};
        const auto start = std::chrono::steady_clock::now();
        std::vector<std::thread> threads;
        for (auto t = 0; t < numThreads; ++t) {
            threads.emplace_back([&, t]() {
                rag3db::main::Connection reader(database.get());
                reader.setMaxNumThreadForExec(1);
                for (auto i = 0; i < searchesPerThread; ++i) {
                    const auto probe = (static_cast<int64_t>(t) * 2503 + i * 7) % NUM_DOCS;
                    auto result =
                        reader.query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', " +
                                     literalVector(probe) + ", 10, efs := 200) RETURN node.id;");
                    if (!result->isSuccess() || result->getNumTuples() != 10) {
                        failures++;
                    }
                }
            });
        }
        for (auto& thread : threads) {
            thread.join();
        }
        EXPECT_EQ(failures.load(), 0);
        return std::chrono::duration<double, std::milli>(
            std::chrono::steady_clock::now() - start)
            .count();
    }
};

TEST_F(VectorSearchCostScratch, Measure) {
    run(1, 200); // à blanc
    for (auto pass = 0; pass < 5; ++pass) {
        const auto one = run(1, 2000);
        const auto four = run(4, 2000);
        fprintf(stderr, "[cout] dim %ld passe %d : 1 fil x2000 = %.0f ms ; 4 fils x2000 = %.0f ms\n",
            static_cast<long>(dim()), pass, one, four);
    }
}

} // namespace
