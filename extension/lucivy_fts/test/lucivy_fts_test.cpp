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

// ── CREATE + all query types ────────────────────────────────────────────────

TEST_F(ApiTest, LucivyCreateAndQueryTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    // Create doc table + insert 3 documents.
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, "
                            "PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language focused on safety'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 1, title: 'Machine Learning', "
                    "body: 'ML is a subset of artificial intelligence'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 2, title: 'C++ Language', "
                    "body: 'C++ is a general-purpose programming language'})")
            ->isSuccess());

    // CREATE_LUCIVY_INDEX
    auto result = conn->query("CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // contains query — "programming" in body → docs 0, 2
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // term query — "programming" in body → docs 0, 2
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"term\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // fuzzy query — "programing" (typo) distance 1 → docs 0, 2
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"fuzzy\",\"field\":\"body\",\"value\":\"programing\",\"distance\":1}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // phrase query — "systems programming" → doc 0 only
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"phrase\",\"field\":\"body\",\"terms\":[\"systems\",\"programming\"]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // parse query — "rust AND programming" → doc 0 only
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"parse\",\"field\":\"body\",\"value\":\"rust AND programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // contains c++ — special chars via regex fallback → doc 2 only
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"title\",\"value\":\"c++\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

// ── DROP ────────────────────────────────────────────────────────────────────

TEST_F(ApiTest, LucivyDropTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, "
                            "PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language focused on safety'})")
            ->isSuccess());

    auto result = conn->query("CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // Query works before drop.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"term\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_GE(countResults(*result), 1u);

    // DROP_LUCIVY_INDEX
    result = conn->query("CALL DROP_LUCIVY_INDEX('doc')");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // Query after drop should fail.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"term\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_FALSE(result->isSuccess());
}

// ── Error cases ─────────────────────────────────────────────────────────────

TEST_F(ApiTest, LucivyErrorTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, "
                            "PRIMARY KEY (ID))")
                    ->isSuccess());

    // Query on table with no index → error.
    auto result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"term\",\"field\":\"body\",\"value\":\"test\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_FALSE(result->isSuccess());

    // CREATE with non-existent property → error.
    result = conn->query("CALL CREATE_LUCIVY_INDEX('doc', ['nonexistent'])");
    ASSERT_FALSE(result->isSuccess());

    // CREATE with non-STRING property → error.
    result = conn->query("CALL CREATE_LUCIVY_INDEX('doc', ['ID'])");
    ASSERT_FALSE(result->isSuccess());
}

// ── Stemmer option ──────────────────────────────────────────────────────────

TEST_F(ApiTest, LucivyStemmerTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, "
                            "PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language focused on safety'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 1, title: 'C++ Language', "
                    "body: 'C++ is a general-purpose programming language'})")
            ->isSuccess());

    // Create index with French stemmer.
    auto result =
        conn->query("CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'], stemmer := 'french')");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // Contains query still works (English words not affected by French stemmer).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_GE(countResults(*result), 1u);
}

// ── Filtered search by node IDs ─────────────────────────────────────────────

TEST_F(ApiTest, LucivyFilteredSearchTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, "
                            "PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language focused on safety'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 1, title: 'Machine Learning', "
                    "body: 'ML is a subset of artificial intelligence'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 2, title: 'C++ Language', "
                    "body: 'C++ is a general-purpose programming language'})")
            ->isSuccess());

    auto result = conn->query("CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // Without filter: "programming" in body → docs 0, 2.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // With filter: allowed_ids := [0] → only doc 0.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10, "
        "allowed_ids := [CAST(0, 'UINT64')]) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // With filter: allowed_ids := [2] → only doc 2.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10, "
        "allowed_ids := [CAST(2, 'UINT64')]) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // With filter: allowed_ids := [1] → doc 1 doesn't contain "programming" → 0 results.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10, "
        "allowed_ids := [CAST(1, 'UINT64')]) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 0u);

    // With filter: allowed_ids := [0, 2] → both docs with "programming" allowed → 2 results.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10, "
        "allowed_ids := [CAST(0, 'UINT64'), CAST(2, 'UINT64')]) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);
}

// ── Filter fields (native typed filtering during scoring) ───────────────────

TEST_F(ApiTest, LucivyFilterFieldsTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    // Table with text + typed filter columns.
    ASSERT_TRUE(conn->query("CREATE NODE TABLE article (ID UINT64, title STRING, body STRING, "
                            "category INT64, score DOUBLE, PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language', category: 1, score: 9.5})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 1, title: 'Python Guide', "
                    "body: 'Python is a programming language for beginners', category: 1, score: 7.0})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 2, title: 'Cooking Recipes', "
                    "body: 'A guide to cooking Italian food', category: 2, score: 8.0})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 3, title: 'C++ Programming', "
                    "body: 'C++ is a general-purpose programming language', category: 1, score: 8.5})")
            ->isSuccess());

    // Create index with filter fields.
    auto result = conn->query(
        "CALL CREATE_LUCIVY_INDEX('article', ['title', 'body'], "
        "filter_fields := ['category', 'score'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // Without filters: "programming" → 3 articles (0, 1, 3).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 3u);

    // With filter: category == 1 → still 3 articles (0, 1, 3 are all category 1).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"category\",\"op\":\"eq\",\"value\":1}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 3u);

    // With filter: category == 2 AND "programming" → 0 results (cooking is category 2).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"category\",\"op\":\"eq\",\"value\":2}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 0u);

    // With filter: score > 8.0 AND "programming" → articles 0 (9.5) and 3 (8.5).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"score\",\"op\":\"gt\",\"value\":8.0}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // With filter: score >= 8.5 AND "programming" → articles 0 (9.5) and 3 (8.5).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"score\",\"op\":\"gte\",\"value\":8.5}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // With multiple filters: category == 1 AND score > 9.0 AND "programming" → article 0 only.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"category\",\"op\":\"eq\",\"value\":1},"
        "{\"field\":\"score\",\"op\":\"gt\",\"value\":9.0}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

// ── DELETE removes docs from search results ─────────────────────────────────

TEST_F(ApiTest, LucivyDeleteTest2) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, "
                            "PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language focused on safety'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 1, title: 'Machine Learning', "
                    "body: 'ML is a subset of artificial intelligence'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 2, title: 'C++ Language', "
                    "body: 'C++ is a general-purpose programming language'})")
            ->isSuccess());

    auto result = conn->query("CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // Before delete: "programming" in body → docs 0, 2.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // Delete doc 0 (Rust Programming).
    result = conn->query("MATCH (n:doc) WHERE n.ID = 0 DELETE n");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // After delete: "programming" in body → only doc 2.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // "intelligence" → only doc 1, unaffected by the delete.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"intelligence\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

// ── UPDATE modifies indexed content ─────────────────────────────────────────

TEST_F(ApiTest, LucivyUpdateTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, "
                            "PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language focused on safety'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 1, title: 'Machine Learning', "
                    "body: 'ML is a subset of artificial intelligence'})")
            ->isSuccess());

    auto result = conn->query("CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // Before update: "programming" → doc 0 only.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // "cooking" → 0 results.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"cooking\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 0u);

    // Update doc 0's body from programming → cooking.
    result = conn->query(
        "MATCH (n:doc) WHERE n.ID = 0 SET n.body = 'Cooking is a wonderful creative hobby'");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // After update: "programming" → 0 results (old content removed).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 0u);

    // "cooking" → doc 0 (new content indexed).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"cooking\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // "intelligence" → doc 1, unaffected.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"intelligence\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

// ── Persistence (always file mode) ──────────────────────────────────────────

TEST_F(ApiTest, LucivyPersistenceTest) {
    // This test always uses file mode to verify persistence across DB close/reopen.
    auto tempDir = TestHelper::getTempDBPathStr("LucivyPersistenceTest");

    auto loadExtension = [&](main::Connection& c) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
        ASSERT_TRUE(c.query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                        ->isSuccess());
#endif
    };

    // ── Phase 1: Create DB, table, docs, index, verify queries ──
    {
        auto db = std::make_unique<main::Database>(tempDir, *systemConfig);
        auto c = std::make_unique<main::Connection>(db.get());
        loadExtension(*c);

        ASSERT_TRUE(c->query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, "
                             "PRIMARY KEY (ID))")
                        ->isSuccess());
        ASSERT_TRUE(
            c->query("CREATE (:doc {ID: 0, title: 'Rust Programming', "
                     "body: 'Rust is a systems programming language focused on safety'})")
                ->isSuccess());
        ASSERT_TRUE(
            c->query("CREATE (:doc {ID: 1, title: 'Machine Learning', "
                     "body: 'ML is a subset of artificial intelligence'})")
                ->isSuccess());
        ASSERT_TRUE(
            c->query("CREATE (:doc {ID: 2, title: 'C++ Language', "
                     "body: 'C++ is a general-purpose programming language'})")
                ->isSuccess());

        auto result = c->query("CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'])");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

        // Verify queries before close.
        result = c->query(
            "CALL QUERY_LUCIVY_INDEX('doc', "
            "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
            "RETURN node_id, score, highlights");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        ASSERT_EQ(countResults(*result), 2u);

        result = c->query(
            "CALL QUERY_LUCIVY_INDEX('doc', "
            "'{\"type\":\"contains\",\"field\":\"title\",\"value\":\"c++\"}', 10) "
            "RETURN node_id, score, highlights");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        ASSERT_EQ(countResults(*result), 1u);

        // Close DB (conn first, then database).
        c.reset();
        db.reset();
    }

    // ── Phase 2: Reopen DB and verify index persisted ──
    {
        auto db = std::make_unique<main::Database>(tempDir, *systemConfig);
        auto c = std::make_unique<main::Connection>(db.get());
        loadExtension(*c);

        // Same queries should return same results after reopen.
        auto result = c->query(
            "CALL QUERY_LUCIVY_INDEX('doc', "
            "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
            "RETURN node_id, score, highlights");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        ASSERT_EQ(countResults(*result), 2u);

        result = c->query(
            "CALL QUERY_LUCIVY_INDEX('doc', "
            "'{\"type\":\"contains\",\"field\":\"title\",\"value\":\"c++\"}', 10) "
            "RETURN node_id, score, highlights");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        ASSERT_EQ(countResults(*result), 1u);

        // Also verify term query works.
        result = c->query(
            "CALL QUERY_LUCIVY_INDEX('doc', "
            "'{\"type\":\"term\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
            "RETURN node_id, score, highlights");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        ASSERT_EQ(countResults(*result), 2u);

        c.reset();
        db.reset();
    }

    // Cleanup.
    removeParentDirectoryOfDBPath(tempDir);
}

// ── Ngram contains without stemmer + BM25 scoring ───────────────────────────

TEST_F(ApiTest, LucivyNgramContainsNoStemmerTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    // Doc 0 mentions "programming" 3 times (high tf).
    // Doc 1 has no "programming".
    // Doc 2 mentions "programming" 1 time (low tf) in a longer body.
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, "
                            "PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 0, title: 'Programming Guide', "
                    "body: 'Programming is fun. I love programming. Programming rocks.'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 1, title: 'Cooking Guide', "
                    "body: 'Cooking is a wonderful creative hobby for everyone'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 2, title: 'Intro to Code', "
                    "body: 'This is an introduction to programming languages and computer science fundamentals for beginners'})")
            ->isSuccess());

    // Create index WITHOUT stemmer — ngram fast path should still work.
    auto result = conn->query("CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // 1. Contains query without stemmer → should find docs 0 and 2.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // 2. Fuzzy query without stemmer — "programing" (typo, distance 1) → should find docs 0 and 2.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"fuzzy\",\"field\":\"body\",\"value\":\"programing\",\"distance\":1}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // 3. Contains with fuzzy distance — "programing" via contains (typo) → should still find docs.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programing\",\"distance\":1}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // 4. BM25 score ordering: doc 0 (tf=3, short body) should score HIGHER than doc 2 (tf=1, long body).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    double score0 = -1.0, score2 = -1.0;
    while (result->hasNext()) {
        auto tuple = result->getNext();
        auto nodeId = tuple->getValue(0)->getValue<uint64_t>();
        auto score = tuple->getValue(1)->getValue<double>();
        ASSERT_GT(score, 0.0) << "BM25 score should be positive, got " << score;
        if (nodeId == 0) score0 = score;
        if (nodeId == 2) score2 = score;
    }
    ASSERT_GT(score0, 0.0) << "Doc 0 should be in results";
    ASSERT_GT(score2, 0.0) << "Doc 2 should be in results";
    ASSERT_GT(score0, score2) << "Doc 0 (tf=3, short) should score higher than doc 2 (tf=1, long)";

    // 5. Substring contains — "program" should match "programming" via ngram.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"program\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // 6. No match — "cooking" in body → only doc 1, not 0 or 2.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"cooking\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

// ── Regex contains (trigram-accelerated + hybrid fuzzy) ──────────────────────

TEST_F(ApiTest, LucivyRegexContainsTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, "
                            "PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language focused on safety'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 1, title: 'Machine Learning', "
                    "body: 'ML is a subset of artificial intelligence'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 2, title: 'C++ Language', "
                    "body: 'C++ is a general-purpose programming language'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 3, title: 'Programmer Guide', "
                    "body: 'Every programmer should learn about version v2.0 systems'})")
            ->isSuccess());

    auto result = conn->query("CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // 1. Regex accelerated by trigrams: "program[a-z]+" matches "programming", "programmer".
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"program[a-z]+\",\"regex\":true}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 3u); // docs 0, 2 (programming), 3 (programmer)

    // 2. BM25 scoring (not constant): different docs should have different scores.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"program[a-z]+\",\"regex\":true}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    double firstScore = -1.0;
    bool hasVariation = false;
    while (result->hasNext()) {
        auto tuple = result->getNext();
        auto score = tuple->getValue(1)->getValue<double>();
        ASSERT_GT(score, 0.0) << "BM25 score should be positive";
        if (firstScore < 0.0) {
            firstScore = score;
        } else if (std::abs(score - firstScore) > 0.001) {
            hasVariation = true;
        }
    }
    ASSERT_TRUE(hasVariation) << "BM25 scores should vary between docs with different tf/fieldnorm";

    // 3. Regex + fuzzy hybrid: pattern has typo "programing[a-z]+" (1 m), distance=1.
    //    Regex won't match "programming" but fuzzy on literal "programing" should.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\","
        "\"value\":\"programing[a-z]+\",\"regex\":true,\"distance\":1}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_GE(countResults(*result), 1u) << "Hybrid should match via fuzzy on literal";

    // 4. Regex with no match.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"python[a-z]+\",\"regex\":true}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 0u);

    // 5. Regex with highlights: verify highlights are returned.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"program[a-z]+\",\"regex\":true}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    bool hasHighlights = false;
    while (result->hasNext()) {
        auto tuple = result->getNext();
        auto highlights = tuple->getValue(2)->toString();
        if (highlights != "[]" && !highlights.empty()) {
            hasHighlights = true;
        }
    }
    ASSERT_TRUE(hasHighlights) << "Regex contains should produce highlights";

    // 6. Non-regex contains still works (regression check).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // 7. Regex with short literals (< 3 chars): full segment scan fallback with BM25.
    //    Pattern "v[0-9]" has literal "v" (1 char) — no trigram acceleration, but BM25.
    //    Doc 3 body contains "version v2.0 systems", should match "v2".
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('doc', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"v[0-9]\",\"regex\":true}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_GE(countResults(*result), 1u) << "Short-literal regex should match via full scan";
}

// ── Boolean + Timestamp filter fields ─────────────────────────────────────────

TEST_F(ApiTest, LucivyBoolTimestampFilterTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    // Table with text + boolean + timestamp filter columns.
    ASSERT_TRUE(conn->query("CREATE NODE TABLE article (ID UINT64, title STRING, body STRING, "
                            "published BOOLEAN, created_at TIMESTAMP, PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language', "
                    "published: true, created_at: timestamp('2024-01-15 10:00:00')})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 1, title: 'Python Guide', "
                    "body: 'Python is a programming language for beginners', "
                    "published: false, created_at: timestamp('2024-06-01 12:00:00')})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 2, title: 'Cooking Recipes', "
                    "body: 'A guide to cooking Italian food', "
                    "published: true, created_at: timestamp('2025-01-01 00:00:00')})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 3, title: 'C++ Programming', "
                    "body: 'C++ is a general-purpose programming language', "
                    "published: true, created_at: timestamp('2023-06-15 08:30:00')})")
            ->isSuccess());

    // Create index with boolean and timestamp filter fields.
    auto result = conn->query(
        "CALL CREATE_LUCIVY_INDEX('article', ['title', 'body'], "
        "filter_fields := ['published', 'created_at'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // Without filters: "programming" → 3 articles (0, 1, 3).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 3u);

    // Filter: published == 1 (true) AND "programming" → articles 0, 3 (article 1 is unpublished).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"published\",\"op\":\"eq\",\"value\":1}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // Filter: published == 0 (false) AND "programming" → article 1 only.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"published\",\"op\":\"eq\",\"value\":0}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // Filter: created_at > 2024-03-01 (epoch us = 1709251200000000) → articles 1, 2 (June 2024, Jan 2025).
    // Among those, "programming" matches only article 1.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"created_at\",\"op\":\"gt\",\"value\":1709251200000000}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // Filter: created_at < 2024-03-01 → articles 0 (Jan 2024), 3 (Jun 2023).
    // Both have "programming" → 2 results.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"created_at\",\"op\":\"lt\",\"value\":1709251200000000}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // Combined: published == 1 AND created_at > 2024-03-01 AND "guide" → article 2 only (cooking, published, Jan 2025).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"guide\","
        "\"filters\":[{\"field\":\"published\",\"op\":\"eq\",\"value\":1},"
        "{\"field\":\"created_at\",\"op\":\"gt\",\"value\":1709251200000000}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

// ── String filter field (ngram contains/starts_with on filter field) ──────────

TEST_F(ApiTest, LucivyStringFilterFieldTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    // Table with text body + string filter column (category name, not full-text).
    ASSERT_TRUE(conn->query("CREATE NODE TABLE article (ID UINT64, title STRING, body STRING, "
                            "tag STRING, PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language', tag: 'programming'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 1, title: 'Python Guide', "
                    "body: 'Python is a programming language for beginners', tag: 'programming'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 2, title: 'Cooking Recipes', "
                    "body: 'A guide to cooking Italian food', tag: 'cooking'})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 3, title: 'C++ Programming', "
                    "body: 'C++ is a general-purpose programming language', tag: 'systems'})")
            ->isSuccess());

    // Create index with string filter field.
    auto result = conn->query(
        "CALL CREATE_LUCIVY_INDEX('article', ['title', 'body'], "
        "filter_fields := ['tag'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // 1. eq filter: tag == "programming" AND "programming" in body → articles 0, 1.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"tag\",\"op\":\"eq\",\"value\":\"programming\"}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // 2. eq filter: tag == "systems" AND "programming" → article 3 only.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"tag\",\"op\":\"eq\",\"value\":\"systems\"}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // 3. eq filter: tag == "cooking" AND "programming" → 0 results.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"tag\",\"op\":\"eq\",\"value\":\"cooking\"}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 0u);

    // 4. starts_with filter: tag starts with "prog" → articles 0, 1 (tag=programming).
    //    Among those, all have "programming" in body → 2 results.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"tag\",\"op\":\"starts_with\",\"value\":\"prog\"}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // 5. contains filter on string field: tag contains "ystem" → article 3 (tag=systems).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"tag\",\"op\":\"contains\",\"value\":\"ystem\"}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // 6. ne filter: tag != "programming" AND "guide" in body → article 2 (cooking).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"guide\","
        "\"filters\":[{\"field\":\"tag\",\"op\":\"ne\",\"value\":\"programming\"}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

// ── Between filter + in filter ───────────────────────────────────────────────

TEST_F(ApiTest, LucivyBetweenAndInFilterTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE article (ID UINT64, title STRING, body STRING, "
                            "category INT64, score DOUBLE, PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language', category: 1, score: 9.5})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 1, title: 'Python Guide', "
                    "body: 'Python is a programming language for beginners', category: 2, score: 7.0})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 2, title: 'Cooking Recipes', "
                    "body: 'A guide to cooking Italian food', category: 3, score: 8.0})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 3, title: 'C++ Programming', "
                    "body: 'C++ is a general-purpose programming language', category: 1, score: 8.5})")
            ->isSuccess());

    auto result = conn->query(
        "CALL CREATE_LUCIVY_INDEX('article', ['title', 'body'], "
        "filter_fields := ['category', 'score'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // 1. between filter: score between [8.0, 9.0] AND "programming" → article 3 (score 8.5).
    //    Article 0 (score 9.5) is outside range; article 1 (score 7.0) is outside range.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"score\",\"op\":\"between\",\"value\":[8.0,9.0]}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // 2. between filter: score between [7.0, 9.5] → all 3 programming articles.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"score\",\"op\":\"between\",\"value\":[7.0,9.5]}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 3u);

    // 3. in filter: category in [1, 3] AND "programming" → articles 0, 3 (category 1).
    //    Article 2 is category 3 but no "programming".
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"category\",\"op\":\"in\",\"value\":[1,3]}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // 4. in filter: category in [2] AND "programming" → article 1 only.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"category\",\"op\":\"in\",\"value\":[2]}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);

    // 5. Combined between + in: score between [7.0, 9.0] AND category in [1] AND "programming"
    //    → article 3 only (score 8.5, category 1).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"score\",\"op\":\"between\",\"value\":[7.0,9.0]},"
        "{\"field\":\"category\",\"op\":\"in\",\"value\":[1]}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

// ── Filters + allowed_ids combined ───────────────────────────────────────────

TEST_F(ApiTest, LucivyFiltersWithAllowedIdsTest) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE article (ID UINT64, title STRING, body STRING, "
                            "category INT64, PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language', category: 1})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 1, title: 'Python Guide', "
                    "body: 'Python is a programming language for beginners', category: 1})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 2, title: 'Cooking Recipes', "
                    "body: 'A guide to cooking Italian food', category: 2})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:article {ID: 3, title: 'C++ Programming', "
                    "body: 'C++ is a general-purpose programming language', category: 1})")
            ->isSuccess());

    auto result = conn->query(
        "CALL CREATE_LUCIVY_INDEX('article', ['title', 'body'], "
        "filter_fields := ['category'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();

    // Without any restriction: "programming" → 3 articles (0, 1, 3).
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 3u);

    // 1. Only filters: category == 1 → articles 0, 1, 3 (all have "programming") → 3.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"category\",\"op\":\"eq\",\"value\":1}]}', 10) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 3u);

    // 2. Only allowed_ids: [0, 1] → articles 0, 1 have "programming" → 2.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\"}', 10, "
        "allowed_ids := [CAST(0, 'UINT64'), CAST(1, 'UINT64')]) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // 3. Filters + allowed_ids combined:
    //    category == 1 (articles 0, 1, 3) AND allowed_ids [0, 3] → intersection = {0, 3}.
    //    Both have "programming" → 2 results.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"category\",\"op\":\"eq\",\"value\":1}]}', 10, "
        "allowed_ids := [CAST(0, 'UINT64'), CAST(3, 'UINT64')]) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);

    // 4. Filters + allowed_ids with empty intersection:
    //    category == 2 (article 2=cooking) AND allowed_ids [0, 1, 3] → intersection = {} → 0 results.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"category\",\"op\":\"eq\",\"value\":2}]}', 10, "
        "allowed_ids := [CAST(0, 'UINT64'), CAST(1, 'UINT64'), CAST(3, 'UINT64')]) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 0u);

    // 5. Filters restrict, allowed_ids restrict further:
    //    category == 1 (articles 0, 1, 3) AND allowed_ids [1] → only article 1.
    result = conn->query(
        "CALL QUERY_LUCIVY_INDEX('article', "
        "'{\"type\":\"contains\",\"field\":\"body\",\"value\":\"programming\","
        "\"filters\":[{\"field\":\"category\",\"op\":\"eq\",\"value\":1}]}', 10, "
        "allowed_ids := [CAST(1, 'UINT64')]) "
        "RETURN node_id, score, highlights");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

// ── SEARCH() in WHERE ──────────────────────────────────────────────────────

// Helper: create doc table, insert docs, create index, load extension
static void setupSearchTest(main::Connection* conn) {
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE doc (ID UINT64, title STRING, body STRING, "
                            "year INT64, PRIMARY KEY (ID))")
                    ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 0, title: 'Rust Programming', "
                    "body: 'Rust is a systems programming language focused on safety', year: 2024})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 1, title: 'Machine Learning', "
                    "body: 'ML is a subset of artificial intelligence', year: 2023})")
            ->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:doc {ID: 2, title: 'C++ Language', "
                    "body: 'C++ is a general-purpose programming language', year: 2025})")
            ->isSuccess());
    auto result = conn->query("CALL CREATE_LUCIVY_INDEX('doc', ['title', 'body'])");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
}

TEST_F(ApiTest, SearchInWhere_Contains) {
    setupSearchTest(conn.get());
    auto result = conn->query(
        "MATCH (d:doc) WHERE SEARCH(d.body, 'programming') RETURN d.title");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);
}

TEST_F(ApiTest, SearchInWhere_ContainsSplit) {
    setupSearchTest(conn.get());
    auto result = conn->query(
        "MATCH (d:doc) WHERE SEARCH(d.body, 'rust safety', 'contains_split') RETURN d.title");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    // "rust" matches doc 0, "safety" matches doc 0 → at least 1
    ASSERT_GE(countResults(*result), 1u);
}

TEST_F(ApiTest, SearchInWhere_Fuzzy) {
    setupSearchTest(conn.get());
    // "programing" (typo) with fuzzy distance 1
    auto result = conn->query(
        "MATCH (d:doc) WHERE SEARCH(d.body, 'programing', 'fuzzy', 1) RETURN d.title");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 2u);
}

TEST_F(ApiTest, SearchInWhere_Parse) {
    setupSearchTest(conn.get());
    auto result = conn->query(
        "MATCH (d:doc) WHERE SEARCH(d.body, 'rust AND programming', 'parse') RETURN d.title");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

TEST_F(ApiTest, SearchInWhere_Score) {
    setupSearchTest(conn.get());
    auto result = conn->query(
        "MATCH (d:doc) WHERE SEARCH(d.body, 'programming') "
        "RETURN d.title, SEARCH_SCORE() AS score ORDER BY score DESC");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_TRUE(result->hasNext());
    auto tuple = result->getNext();
    auto score = tuple->getValue(1)->getValue<double>();
    ASSERT_GT(score, 0.0) << "Score should be positive";
}

TEST_F(ApiTest, SearchInWhere_Highlights) {
    setupSearchTest(conn.get());
    auto result = conn->query(
        "MATCH (d:doc) WHERE SEARCH(d.body, 'programming') "
        "RETURN d.title, SEARCH_HIGHLIGHTS() AS hl");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_TRUE(result->hasNext());
    auto tuple = result->getNext();
    auto hl = tuple->getValue(1)->getValue<std::string>();
    // Highlights should be a JSON object
    ASSERT_FALSE(hl.empty());
    ASSERT_EQ(hl[0], '{');
}

TEST_F(ApiTest, SearchInWhere_NoIndex_Error) {
    // Table without index
#ifndef __STATIC_LINK_EXTENSION_TEST__
    ASSERT_TRUE(conn->query(stringFormat("LOAD EXTENSION '{}'",
                                TestHelper::appendRag3dbRootPath(
                                    "extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")))
                    ->isSuccess());
#endif
    ASSERT_TRUE(conn->query("CREATE NODE TABLE noindex (ID UINT64, body STRING, PRIMARY KEY (ID))")
                    ->isSuccess());
    auto result = conn->query(
        "MATCH (d:noindex) WHERE SEARCH(d.body, 'test') RETURN d.body");
    ASSERT_FALSE(result->isSuccess());
}

TEST_F(ApiTest, SearchInWhere_WithCypherFilter) {
    setupSearchTest(conn.get());
    auto result = conn->query(
        "MATCH (d:doc) WHERE SEARCH(d.body, 'programming') AND d.year >= 2025 "
        "RETURN d.title, d.year");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    // Only doc 2 (C++ Language, year 2025) matches both
    ASSERT_EQ(countResults(*result), 1u);
}

TEST_F(ApiTest, SearchInWhere_OrderByLimit) {
    setupSearchTest(conn.get());
    auto result = conn->query(
        "MATCH (d:doc) WHERE SEARCH(d.body, 'programming') "
        "RETURN d.title, SEARCH_SCORE() AS score ORDER BY score DESC LIMIT 1");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_EQ(countResults(*result), 1u);
}

} // namespace testing
} // namespace rag3db
