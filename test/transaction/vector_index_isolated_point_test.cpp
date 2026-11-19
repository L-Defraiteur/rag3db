// A point left alone far from the rest of an indexed table, then joined by a cloud of new points
// (ticket
// docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-07-un-rang-deplace-pendant-un-create-n-est-pas-toujours-rattrape-premier.md,
// 19 Nov. 2026). At the build, the lone point links to nodes of the far cloud that are not on the
// route of a search coming from its direction; the points inserted next to it do not find it and do
// not link to it. A search for its exact vector then stops in the new cloud, at its nearest
// neighbour. No concurrency is needed: the same happens through SET and through INSERT, in memory
// and on disk.

#include <cstdint>
#include <cstdlib>
#include <filesystem>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "test_helper/test_helper.h"

namespace rag3db::testing {
namespace {

class VectorIndexIsolatedPoint : public EmptyDBTest {
protected:
    static constexpr int ROUNDS = 10;
    // The build is parallel: with 1 or 4 threads the lone point was never missed (0/12 each), with
    // 8 it was missed 5 times in 12, with 24 4 times (luciepc, 19 Nov.). The test sets its own
    // count, or it would run at the 2 threads of the test configuration and never see the defect.
    static constexpr uint64_t BUILD_THREADS = 8;
    static constexpr int CLOUD = 150;

    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        const auto extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        ASSERT_TRUE(std::filesystem::exists(extension))
            << "[check: setup] build rag3db_vector_extension before this test";
        systemConfig->maxNumThreads = BUILD_THREADS;
        createDBAndConn();
        ok("LOAD EXTENSION '" + extension + "';");
        ok("CALL threads=" + std::to_string(BUILD_THREADS) + ";");
    }

    void ok(const std::string& query) {
        const auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    static std::string cloudVector(int j) {
        return "[" + std::to_string(500 + j) + ", " + std::to_string(j % 7) +
               ", 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1]";
    }

    // One round on a fresh table: 20 000 rows, row 1 moved far away, the index built, then a cloud
    // formed next to row 1, by SET of existing rows or by INSERT of new ones. Returns the id the
    // search for row 1's exact vector puts first.
    int64_t round(int r, bool bySet) {
        const auto table = "C" + std::to_string(r);
        ok("CREATE NODE TABLE " + table + "(id INT64 PRIMARY KEY, vec FLOAT[16]);");
        ok("UNWIND range(0, 19999) AS i CREATE (:" + table +
            " {id: i, vec: [i % 17, i % 23, i % 29, i % 31, i % 37, i % 41, i % 43, i % 47, i % "
            "53, "
            "i % 59, i % 61, i % 67, i % 71, i % 73, i % 79, i / 1000.0]});");
        ok("MATCH (n:" + table +
            ") WHERE n.id = 1 SET n.vec = [500, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1];");
        ok("CALL CREATE_VECTOR_INDEX('" + table + "', 'idx_" + table +
            "', 'vec', metric := 'l2');");
        for (int j = 1; j <= CLOUD; j++) {
            if (bySet) {
                ok("MATCH (n:" + table + ") WHERE n.id = " + std::to_string(2 * j + 1) +
                    " SET n.vec = " + cloudVector(j) + ";");
            } else {
                ok("CREATE (:" + table + " {id: " + std::to_string(100000 + j) +
                    ", vec: " + cloudVector(j) + "});");
            }
        }
        const auto result =
            conn->query("CALL QUERY_VECTOR_INDEX('" + table + "', 'idx_" + table +
                        "', [500, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1], 1, "
                        "efs := 64) RETURN node.id;");
        EXPECT_TRUE(result->isSuccess()) << "[check: query] " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    void expectFoundFirstEveryRound(bool bySet, const std::string& check) {
        std::string misses;
        for (int r = 0; r < ROUNDS; r++) {
            const auto first = round(r, bySet);
            if (HasFatalFailure()) {
                return;
            }
            if (first != 1) {
                misses += " round " + std::to_string(r) + " got " + std::to_string(first) + ";";
            }
        }
        EXPECT_TRUE(misses.empty()) << "[check: " << check << "] row 1 not found first at its "
                                    << "exact vector:" << misses;
    }
};

// The same, with the test-only pause between taking a slot of a neighbour list and writing it
// (RAG3DB_TEST_HNSW_APPEND_PAUSE_EVERY): it widens the window where an append and a shrink of the
// same list overlapped, so the lost edges show without depending on how busy the machine is.
TEST_F(VectorIndexIsolatedPoint, ALonePointIsFoundFirstEvenWhenAppendsPauseMidway) {
    struct PauseAppends {
        PauseAppends() { setenv("RAG3DB_TEST_HNSW_APPEND_PAUSE_EVERY", "64", 1); }
        ~PauseAppends() { unsetenv("RAG3DB_TEST_HNSW_APPEND_PAUSE_EVERY"); }
    } pause;
    expectFoundFirstEveryRound(true, "lone-point-found-first-with-paused-appends");
}

TEST_F(VectorIndexIsolatedPoint, ALonePointIsFoundFirstOnceSetRowsFormACloudBesideIt) {
    expectFoundFirstEveryRound(true, "lone-point-found-first-after-set-cloud");
}

TEST_F(VectorIndexIsolatedPoint, ALonePointIsFoundFirstOnceInsertedRowsFormACloudBesideIt) {
    expectFoundFirstEveryRound(false, "lone-point-found-first-after-insert-cloud");
}

} // namespace
} // namespace rag3db::testing
