// The rows a vector search scans outside the graph, readable without writing (ticket
// docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md,
// Lucie's choice (a) of 18 Nov. 2026). SHOW_INDEXES gets a ninth column,
// rows_outside_graph_at_most: for a vector index, the rows of [numCheckpointedNodes, numTotalRows)
// at the caller's snapshot, the range every search scans one by one. It is an upper bound: rows of
// that range already linked by a CREATE's catch-up, or without a vector, are counted too. NULL for
// an index without a graph. The first witness of this read was written by Codex on 16 Nov. (branch
// cx-hotfixs-rows-outside-graph-red-pilot-v2), for an exact count; this one checks the bound.

#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "test_helper/test_helper.h"
#include "transaction/transaction.h"
#include "transaction/transaction_manager.h"

namespace rag3db::testing {
namespace {

class VectorIndexDiagnosticsTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        const auto extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        ASSERT_TRUE(std::filesystem::exists(extension))
            << "[check: setup] build rag3db_vector_extension before this test";
        createDBAndConn();
        ok("LOAD EXTENSION '" + extension + "';");
        ok("CALL auto_checkpoint=false;");
        ok("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
        ok(insertRows(0, 400));
        ok("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    }

    void ok(const std::string& query) {
        const auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    static std::string insertRows(int64_t from, int64_t count) {
        return "UNWIND range(" + std::to_string(from) + ", " + std::to_string(from + count - 1) +
               ") AS i CREATE (:Doc {id: i, vec: [CAST(i AS FLOAT), 1.0, 2.0, 3.0]});";
    }

    // The bound read for doc_index, or -1 when the column is NULL.
    int64_t rowsOutsideGraphAtMost() {
        const auto result =
            conn->query("CALL SHOW_INDEXES() RETURN index_name, rows_outside_graph_at_most;");
        EXPECT_TRUE(result->isSuccess())
            << "[check: rows-outside-graph-at-most-is-readable] " << result->getErrorMessage();
        if (!result->isSuccess()) {
            return -2;
        }
        while (result->hasNext()) {
            const auto row = result->getNext();
            if (row->getValue(0)->getValue<std::string>() == "doc_index") {
                return row->getValue(1)->isNull() ?
                           -1 :
                           static_cast<int64_t>(row->getValue(1)->getValue<uint64_t>());
            }
        }
        return -3;
    }
};

TEST_F(VectorIndexDiagnosticsTest, AFreshIndexReportsZeroAndTheColumnsKeepTheirOrder) {
    EXPECT_EQ(rowsOutsideGraphAtMost(), 0) << "[check: a-fresh-index-scans-nothing-outside]";
    const auto schema = conn->query("CALL SHOW_INDEXES() RETURN *;");
    ASSERT_TRUE(schema->isSuccess()) << schema->getErrorMessage();
    const std::vector<std::string> names{"table_name", "index_name", "index_type", "property_names",
        "extension_loaded", "index_definition", "state", "extension_error",
        "rows_outside_graph_at_most"};
    EXPECT_EQ(schema->getColumnNames(), names) << "[check: the-eight-columns-keep-their-order]";
}

TEST_F(VectorIndexDiagnosticsTest, RowsInsertedSinceTheLastCheckpointAreCountedThenFolded) {
    ok(insertRows(400, 150));
    EXPECT_EQ(rowsOutsideGraphAtMost(), 150) << "[check: rows-after-the-checkpoint-are-counted]";
    ok("CHECKPOINT;");
    EXPECT_EQ(rowsOutsideGraphAtMost(), 0) << "[check: a-checkpoint-folds-them-into-the-graph]";
}

TEST_F(VectorIndexDiagnosticsTest, TheReadTakesNoLockInAReadOnlyTransaction) {
    ok(insertRows(400, 20));
    ok("BEGIN TRANSACTION READ ONLY;");
    auto* context = getClientContext(*conn);
    auto* transaction = transaction::Transaction::Get(*context);
    auto& locks = transaction::TransactionManager::Get(*context)->getLockManager();
    ASSERT_EQ(locks.getNumLocksHeld(transaction->getID()), 0u);
    EXPECT_EQ(rowsOutsideGraphAtMost(), 20);
    EXPECT_EQ(locks.getNumLocksHeld(transaction->getID()), 0u) << "[check: the-read-takes-no-lock]";
    ok("COMMIT;");
}

} // namespace
} // namespace rag3db::testing
