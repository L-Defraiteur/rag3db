// The concurrent CREATE_VECTOR_INDEX reads its vectors from a column cached at the start of T_B
// (_CACHE_ARRAY_COLUMN_LOCALLY), but bounds its insertion by the table's row count read later, at
// the bind of _CREATE_HNSW_INDEX. Rows committed in between by another connection lie past the
// cached column: the build read their null mask past its allocation (heap-buffer-overflow under
// ASAN, in NullMask::isNull), and their vectors past the cached data (ticket
// 2026-11-08-le-create-vector-index-concurrent-lit-une-ligne-au-dela-de-la-colonne-en-cache).
//
// The witness sends the query that rewriteCreateHNSWQuery
// (extension/vector/src/function/create_hnsw_index.cpp) builds, with rows inserted right between
// the cache and the build. Deterministic, no thread.

#include <filesystem>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/client_context.h"
#include "main/connection.h"
#include "test_helper/test_helper.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class VectorIndexCachedColumn : public EmptyDBTest {
protected:
    static constexpr int64_t CACHED_ROWS = 2000;
    static constexpr int64_t LATE_ROWS = 6000;

    void SetUp() override {
        EmptyDBTest::SetUp();
        extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        if (!std::filesystem::exists(extension)) {
            GTEST_SKIP() << "the vector extension is not built (-DBUILD_EXTENSIONS=vector)";
        }
        createDBAndConn();
        ok(*conn, "LOAD EXTENSION '" + extension + "';");
        ok(*conn, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
        ok(*conn, insertRows(0, CACHED_ROWS));
    }

    static void ok(Connection& connection, const std::string& query) {
        const auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    // Distinct vectors: row i sits alone at [i, i % 7, i % 11, -i].
    static std::string insertRows(int64_t from, int64_t count) {
        return "UNWIND range(" + std::to_string(from) + ", " + std::to_string(from + count - 1) +
               ") AS i CREATE (:Doc {id: i, vec: [CAST(i AS FLOAT), CAST(i % 7 AS FLOAT), "
               "CAST(i % 11 AS FLOAT), CAST(-i AS FLOAT)]});";
    }

    std::string tableID() {
        const auto result = conn->query("CALL show_tables() WHERE name = 'Doc' RETURN id;");
        EXPECT_TRUE(result->isSuccess() && result->hasNext()) << result->getErrorMessage();
        return result->getNext()->getValue(0)->toString();
    }

    std::string extension;
};

TEST_F(VectorIndexCachedColumn, RowsCommittedAfterTheColumnIsCachedAreNotReadPastIt) {
    const auto id = tableID();
    const std::string params = "metric := 'l2', cache_embeddings := true";
    // The query that rewriteCreateHNSWQuery builds, sent as one query as it is (the NOTED entry's
    // token lives as long as that query), with LATE_ROWS rows inserted by T_B itself between the
    // cache and the build: the build's bind counts them (getNumTotalRows counts the local rows),
    // the cached column does not have them.
    ok(*conn, "CALL enable_internal_catalog=true;");
    ok(*conn, "BEGIN TRANSACTION;"
              "CREATE REL TABLE _" +
                  id +
                  "_doc_index_UPPER (FROM Doc TO Doc) WITH (storage_direction='fwd');"
                  "CREATE REL TABLE _" +
                  id +
                  "_doc_index_LOWER (FROM Doc TO Doc) WITH (storage_direction='fwd');"
                  "CALL _CREATE_HNSW_NOTE('Doc', 'doc_index', 'vec', " +
                  params +
                  ");"
                  "COMMIT;"
                  "CALL _WAIT_HNSW_WRITERS('Doc', 'doc_index');"
                  "BEGIN TRANSACTION;"
                  "CALL _CACHE_ARRAY_COLUMN_LOCALLY('Doc', 'vec');" +
                  insertRows(CACHED_ROWS, LATE_ROWS) +
                  "CALL _CREATE_HNSW_INDEX('Doc', 'doc_index', 'vec', " + params +
                  ");"
                  "COMMIT;");
    ok(*conn, "CALL enable_internal_catalog=false;");

    // Every row, cached or late, is found first at its exact vector.
    uint64_t notFirst = 0;
    std::string firstMiss;
    for (int64_t i = 0; i < CACHED_ROWS + LATE_ROWS; i += 97) {
        const auto query = "CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [CAST(" +
                           std::to_string(i) + " AS FLOAT), CAST(" + std::to_string(i % 7) +
                           " AS FLOAT), CAST(" + std::to_string(i % 11) + " AS FLOAT), CAST(" +
                           std::to_string(-i) + " AS FLOAT)], 1, efs := 500) RETURN node.id;";
        const auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        const auto got = result->hasNext() ? result->getNext()->getValue(0)->toString() : "none";
        if (got != std::to_string(i)) {
            if (notFirst++ == 0) {
                firstMiss = "row " + std::to_string(i) + " gave " + got;
            }
        }
    }
    EXPECT_EQ(notFirst, 0u) << "[check: rows-past-the-cached-column-found-first] " << firstMiss;
}

} // namespace
