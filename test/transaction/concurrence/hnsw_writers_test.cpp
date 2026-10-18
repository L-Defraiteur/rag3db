// Sur une table indexée par HNSW, les écrivains passent un par un (ticket
// 2026-10-18-sur-une-table-hnsw-les-ecrivains-passent-un-par-un, E11). Chaque écriture prend
// l'index de la table en exclusif (NodeTable::insert, NodeTable::lockIndexForWrite) et ne le rend
// qu'à la validation (LockManager::releaseAll) : un écrivain qui garde sa transaction ouverte après
// son insertion fait attendre tous les autres écrivains de la table jusqu'à sa validation.
//
// - HnswWriters.DoNotWaitForEachOther : une table de 1 000 lignes indexée par HNSW. A ouvre un
// bloc,
//   insère une ligne, puis garde sa transaction ouverte HOLD_MS. Pendant ce temps, B insère une
//   autre ligne en automatique. Contrôle writers-of-an-hnsw-table-do-not-wait-for-each-other :
//   l'insertion de B finit avant la validation de A. Aujourd'hui, elle attend cette validation.

#ifndef __SINGLE_THREADED__

#include <atomic>
#include <chrono>
#include <iostream>
#include <string>
#include <thread>

#include "bench_harness.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

using Clock = std::chrono::steady_clock;
constexpr int64_t NUM_ROWS = 1'000;
constexpr int64_t HOLD_MS = 1'500;

class HnswWriters : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        concurrency::applyBenchSettings(*conn);
        concurrency::loadVectorExtension(*conn);
    }

    static void ok(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess())
            << "[check: query] " << query.substr(0, 160) << " : " << result->getErrorMessage();
    }

    static std::string vectorOf(int64_t id) {
        return stringFormat("[{}, {}, {}, {}]", id % 7, id % 11, id % 13, id);
    }
};

TEST_F(HnswWriters, DoNotWaitForEachOther) {
    ok(*conn, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
    ok(*conn, stringFormat("UNWIND range(0, {}) AS i CREATE (:Doc {id: i, vec: [i % 7, i % 11, "
                           "i % 13, i]});",
                  NUM_ROWS - 1));
    ok(*conn, "CALL CREATE_VECTOR_INDEX('Doc', 'doc_vec', 'vec', metric := 'l2');");

    rag3db::main::Connection a(database.get());
    rag3db::main::Connection b(database.get());
    ok(a, "BEGIN TRANSACTION;");
    ok(a, stringFormat("CREATE (:Doc {id: {}, vec: {}});", NUM_ROWS, vectorOf(NUM_ROWS)));
    if (HasFatalFailure()) {
        return;
    }
    const auto start = Clock::now();
    std::atomic<double> bEnd{-1};
    std::string bError;
    std::thread writer([&] {
        auto result = b.query(
            stringFormat("CREATE (:Doc {id: {}, vec: {}});", NUM_ROWS + 1, vectorOf(NUM_ROWS + 1)));
        bError = result->isSuccess() ? "" : result->getErrorMessage();
        bEnd = std::chrono::duration<double, std::milli>(Clock::now() - start).count();
    });
    std::this_thread::sleep_for(std::chrono::milliseconds(HOLD_MS));
    const auto aCommitStart =
        std::chrono::duration<double, std::milli>(Clock::now() - start).count();
    ok(a, "COMMIT;");
    writer.join();
    std::cerr << "  [écrivains HNSW] A garde sa transaction " << HOLD_MS
              << " ms ; l'insertion de B finit à " << bEnd.load() << " ms ("
              << (bError.empty() ? "validée" : "refusée : " + bError.substr(0, 120))
              << "), la validation de A part à " << aCommitStart << " ms\n";
    EXPECT_TRUE(bError.empty() && bEnd.load() < aCommitStart)
        << "[check: writers-of-an-hnsw-table-do-not-wait-for-each-other] B's insert ended at "
        << bEnd.load() << " ms, A's COMMIT started at " << aCommitStart << " ms"
        << (bError.empty() ? "" : " ; B was refused : " + bError);
    // Les deux lignes, si B est passée.
    if (bError.empty()) {
        auto count =
            conn->query(stringFormat("MATCH (d:Doc) WHERE d.id >= {} RETURN count(d);", NUM_ROWS));
        ASSERT_TRUE(count->isSuccess());
        EXPECT_EQ(count->getNext()->getValue(0)->getValue<int64_t>(), 2) << "[check: query]";
    }
}

} // namespace

#endif
