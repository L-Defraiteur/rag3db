#include "api_test/api_test.h"

using namespace rag3db::common;

namespace rag3db {
namespace testing {

static uint64_t countResults(main::QueryResult& result) {
    uint64_t count = 0;
    while (result.hasNext()) {
        result.getNext();
        count++;
    }
    return count;
}

static void loadVectorExtension(main::Connection& c) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(c.query(stringFormat("LOAD EXTENSION '{}'",
                            TestHelper::appendRag3dbRootPath(
                                "extension/vector/build/libvector.rag3db_extension")))
                    ->isSuccess());
#endif
}

static void createTableAndIndex(main::Connection& c) {
    ASSERT_TRUE(c.query("CREATE NODE TABLE vec_doc (ID UINT64, title STRING, "
                        "emb FLOAT[3], PRIMARY KEY (ID))")
                    ->isSuccess());
    // Insert 5 docs with 3-dim embeddings.
    ASSERT_TRUE(c.query("CREATE (:vec_doc {ID: 0, title: 'alpha', emb: [1.0, 0.0, 0.0]})")
                    ->isSuccess());
    ASSERT_TRUE(c.query("CREATE (:vec_doc {ID: 1, title: 'beta', emb: [0.0, 1.0, 0.0]})")
                    ->isSuccess());
    ASSERT_TRUE(c.query("CREATE (:vec_doc {ID: 2, title: 'gamma', emb: [0.0, 0.0, 1.0]})")
                    ->isSuccess());
    ASSERT_TRUE(c.query("CREATE (:vec_doc {ID: 3, title: 'delta', emb: [0.9, 0.1, 0.0]})")
                    ->isSuccess());
    ASSERT_TRUE(c.query("CREATE (:vec_doc {ID: 4, title: 'epsilon', emb: [0.1, 0.9, 0.0]})")
                    ->isSuccess());

    auto result = c.query(
        "CALL CREATE_VECTOR_INDEX('vec_doc', 'vec_idx', 'emb', metric := 'cosine')");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
}

// ── VECTOR_SEARCH in WHERE — Basic ──────────────────────────────────────────

TEST_F(ApiTest, VectorSearchInWhere_Basic) {
    loadVectorExtension(*conn);
    createTableAndIndex(*conn);

    // Search for vectors close to [1, 0, 0], k=3.
    auto result = conn->query(
        "MATCH (d:vec_doc) "
        "WHERE VECTOR_SEARCH(d.emb, [1.0, 0.0, 0.0], 3) "
        "RETURN d.title");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 3u);
}

// ── VECTOR_SEARCH — Distance ────────────────────────────────────────────────

TEST_F(ApiTest, VectorSearchInWhere_Distance) {
    loadVectorExtension(*conn);
    createTableAndIndex(*conn);

    auto result = conn->query(
        "MATCH (d:vec_doc) "
        "WHERE VECTOR_SEARCH(d.emb, [1.0, 0.0, 0.0], 5) "
        "RETURN d.title, VECTOR_DISTANCE() AS dist "
        "ORDER BY dist ASC");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    auto tuple = result->getNext();
    auto dist0 = tuple->getValue(1)->getValue<double>();
    // The closest vector to [1,0,0] should have distance ~0 (itself or very close).
    ASSERT_GE(dist0, 0.0);

    if (result->hasNext()) {
        auto tuple2 = result->getNext();
        auto dist1 = tuple2->getValue(1)->getValue<double>();
        // Distances should be non-decreasing (ORDER BY ASC).
        ASSERT_GE(dist1, dist0);
    }
}

// ── VECTOR_SEARCH — Order By Limit ──────────────────────────────────────────

TEST_F(ApiTest, VectorSearchInWhere_OrderByLimit) {
    loadVectorExtension(*conn);
    createTableAndIndex(*conn);

    auto result = conn->query(
        "MATCH (d:vec_doc) "
        "WHERE VECTOR_SEARCH(d.emb, [0.0, 1.0, 0.0], 5) "
        "RETURN d.title, VECTOR_DISTANCE() AS dist "
        "ORDER BY dist ASC LIMIT 2");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);
}

// ── VECTOR_SEARCH — No Index Error ──────────────────────────────────────────

TEST_F(ApiTest, VectorSearchInWhere_NoIndex) {
    loadVectorExtension(*conn);

    ASSERT_TRUE(conn->query("CREATE NODE TABLE vec_noindex (ID UINT64, "
                            "emb FLOAT[3], PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(conn->query("CREATE (:vec_noindex {ID: 0, emb: [1.0, 0.0, 0.0]})")
                    ->isSuccess());

    auto result = conn->query(
        "MATCH (d:vec_noindex) "
        "WHERE VECTOR_SEARCH(d.emb, [1.0, 0.0, 0.0], 3) "
        "RETURN d.ID");
    ASSERT_FALSE(result->isSuccess());
}

} // namespace testing
} // namespace rag3db
