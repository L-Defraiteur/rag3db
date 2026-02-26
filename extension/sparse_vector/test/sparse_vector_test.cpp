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

// ── CREATE + INSERT + QUERY ─────────────────────────────────────────────────

TEST_F(ApiTest, SparseVectorCreateAndQueryTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/sparse_vector/build/libsparse_vector.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc (ID UINT64, "
                            "sparse_indices INT64[], sparse_weights DOUBLE[], PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(conn->query("CREATE (:doc {ID: 0, "
                            "sparse_indices: [1, 2, 3], sparse_weights: [0.5, 0.3, 0.2]})")
                    ->isSuccess());
    ASSERT_TRUE(conn->query("CREATE (:doc {ID: 1, "
                            "sparse_indices: [2, 3, 4], sparse_weights: [0.4, 0.6, 0.1]})")
                    ->isSuccess());
    ASSERT_TRUE(conn->query("CREATE (:doc {ID: 2, "
                            "sparse_indices: [1, 4, 5], sparse_weights: [0.9, 0.1, 0.1]})")
                    ->isSuccess());

    auto result = conn->query(
        "CALL CREATE_SPARSE_VECTOR_INDEX('doc', 'sparse_indices', 'sparse_weights')");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // Query: tokens [1, 2] with weights [1.0, 1.0].
    // Expected scores: doc0=0.5+0.3=0.8, doc1=0.4, doc2=0.9
    result = conn->query(
        "CALL QUERY_SPARSE_VECTOR_INDEX('doc', [1, 2], [1.0, 1.0], 10) "
        "RETURN node_id, score");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 3u);
}

// ── DELETE ───────────────────────────────────────────────────────────────────

TEST_F(ApiTest, SparseVectorDeleteTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/sparse_vector/build/libsparse_vector.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc2 (ID UINT64, "
                            "si INT64[], sw DOUBLE[], PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(conn->query("CREATE (:doc2 {ID: 0, si: [1], sw: [1.0]})")->isSuccess());
    ASSERT_TRUE(conn->query("CREATE (:doc2 {ID: 1, si: [1], sw: [0.5]})")->isSuccess());

    auto result = conn->query("CALL CREATE_SPARSE_VECTOR_INDEX('doc2', 'si', 'sw')");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    ASSERT_TRUE(conn->query("MATCH (d:doc2) WHERE d.ID = 0 DELETE d")->isSuccess());

    result = conn->query(
        "CALL QUERY_SPARSE_VECTOR_INDEX('doc2', [1], [1.0], 10) "
        "RETURN node_id, score");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

// ── UPDATE ──────────────────────────────────────────────────────────────────

TEST_F(ApiTest, SparseVectorUpdateTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/sparse_vector/build/libsparse_vector.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc3 (ID UINT64, "
                            "si INT64[], sw DOUBLE[], PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(conn->query("CREATE (:doc3 {ID: 0, si: [1], sw: [1.0]})")->isSuccess());

    auto result = conn->query("CALL CREATE_SPARSE_VECTOR_INDEX('doc3', 'si', 'sw')");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // Update: token 1 → token 2.
    ASSERT_TRUE(conn->query("MATCH (d:doc3) WHERE d.ID = 0 SET d.si = [2], d.sw = [0.9]")
                    ->isSuccess());

    // Token 1: 0 results.
    result = conn->query(
        "CALL QUERY_SPARSE_VECTOR_INDEX('doc3', [1], [1.0], 10) "
        "RETURN node_id, score");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 0u);

    // Token 2: 1 result.
    result = conn->query(
        "CALL QUERY_SPARSE_VECTOR_INDEX('doc3', [2], [1.0], 10) "
        "RETURN node_id, score");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

// ── Persistence ─────────────────────────────────────────────────────────────

TEST_F(ApiTest, SparseVectorPersistenceTest) {
    auto tempDir = TestHelper::getTempDBPathStr("SparseVectorPersistenceTest");

    auto loadExtension = [&](main::Connection& c) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
        ASSERT_TRUE(c.query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/sparse_vector/build/libsparse_vector.rag3db_extension")))
                        ->isSuccess());
#endif
    };

    // Phase 1: Create DB, table, docs, index.
    {
        auto db = std::make_unique<main::Database>(tempDir, *systemConfig);
        auto c = std::make_unique<main::Connection>(db.get());
        loadExtension(*c);

        ASSERT_TRUE(c->query("CREATE NODE TABLE doc4 (ID UINT64, "
                             "si INT64[], sw DOUBLE[], PRIMARY KEY (ID))")
                        ->isSuccess());
        ASSERT_TRUE(c->query("CREATE (:doc4 {ID: 0, si: [1, 2], sw: [0.5, 0.3]})")
                        ->isSuccess());
        ASSERT_TRUE(c->query("CREATE (:doc4 {ID: 1, si: [2, 3], sw: [0.8, 0.2]})")
                        ->isSuccess());

        auto result = c->query("CALL CREATE_SPARSE_VECTOR_INDEX('doc4', 'si', 'sw')");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

        result = c->query(
            "CALL QUERY_SPARSE_VECTOR_INDEX('doc4', [2], [1.0], 10) "
            "RETURN node_id, score");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        ASSERT_EQ(countResults(*result), 2u);

        c.reset();
        db.reset();
    }

    // Phase 2: Reopen DB, verify same results.
    {
        auto db = std::make_unique<main::Database>(tempDir, *systemConfig);
        auto c = std::make_unique<main::Connection>(db.get());
        loadExtension(*c);

        auto result = c->query(
            "CALL QUERY_SPARSE_VECTOR_INDEX('doc4', [2], [1.0], 10) "
            "RETURN node_id, score");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        ASSERT_EQ(countResults(*result), 2u);
    }
}

// ── Filtered search ─────────────────────────────────────────────────────────

TEST_F(ApiTest, SparseVectorFilteredSearchTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/sparse_vector/build/libsparse_vector.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc5 (ID UINT64, "
                            "si INT64[], sw DOUBLE[], PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(conn->query("CREATE (:doc5 {ID: 0, si: [1], sw: [0.5]})")->isSuccess());
    ASSERT_TRUE(conn->query("CREATE (:doc5 {ID: 1, si: [1], sw: [0.9]})")->isSuccess());
    ASSERT_TRUE(conn->query("CREATE (:doc5 {ID: 2, si: [1], sw: [0.7]})")->isSuccess());

    auto result = conn->query("CALL CREATE_SPARSE_VECTOR_INDEX('doc5', 'si', 'sw')");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // allowed_ids: only node offsets 0 and 2.
    result = conn->query(
        "CALL QUERY_SPARSE_VECTOR_INDEX('doc5', [1], [1.0], 10, "
        "allowed_ids := [CAST(0, 'UINT64'), CAST(2, 'UINT64')]) "
        "RETURN node_id, score");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);
}

// ── DROP ────────────────────────────────────────────────────────────────────

TEST_F(ApiTest, SparseVectorDropTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/sparse_vector/build/libsparse_vector.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc6 (ID UINT64, "
                            "si INT64[], sw DOUBLE[], PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(conn->query("CREATE (:doc6 {ID: 0, si: [1], sw: [1.0]})")->isSuccess());

    auto result = conn->query("CALL CREATE_SPARSE_VECTOR_INDEX('doc6', 'si', 'sw')");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    auto dropResult = conn->query("CALL DROP_SPARSE_VECTOR_INDEX('doc6')");
    ASSERT_TRUE(dropResult->isSuccess()) << dropResult->getErrorMessage();

    result = conn->query(
        "CALL QUERY_SPARSE_VECTOR_INDEX('doc6', [1], [1.0], 10) "
        "RETURN node_id, score");
    ASSERT_FALSE(result->isSuccess());
}

} // namespace testing
} // namespace rag3db
