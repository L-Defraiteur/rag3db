// Contrat de lecture, ticket :
// docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md
// Ce premier témoin couvre la projection et le zéro connu, pas encore une queue
// non vide, sa définition exacte ou le comportement sous écritures concurrentes.
#include <cstdint>
#include <filesystem>
#include <iostream>
#include <string>
#include <vector>

#include "catalog/catalog.h"
#include "common/build_id.h"
#include "graph_test/private_graph_test.h"
#include "index/hnsw_index.h"
#include "main/connection.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
#include "test_helper/test_helper.h"
#include "transaction/transaction.h"
#include "transaction/transaction_manager.h"

namespace rag3db::testing {
namespace {

class VectorIndexDiagnosticsTest : public EmptyDBTest {
protected:
    void SetUp() override {
        std::cout << "BUILD_ID=" << RAG3DB_BUILD_ID << std::endl;
        EmptyDBTest::SetUp();
        ASSERT_FALSE(inMemMode) << "[check: setup] on-disk fixture required";
        const auto extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        ASSERT_TRUE(std::filesystem::exists(extension))
            << "[check: setup] build rag3db_vector_extension before this test";
        createDBAndConn();
        for (const auto& query : {
                 "LOAD EXTENSION '" + extension + "';",
                 std::string{"CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);"},
                 std::string{"UNWIND range(0, 3) AS i CREATE (:Doc {id: i, "
                             "vec: [CAST(i AS FLOAT), 1.0, 2.0, 3.0]});"},
                 std::string{"CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');"}}) {
            auto result = conn->query(query);
            ASSERT_TRUE(result->isSuccess()) << "[check: setup] " << result->getErrorMessage();
        }
        auto listed = conn->query("CALL SHOW_INDEXES() RETURN index_name, state;");
        ASSERT_TRUE(listed->isSuccess()) << "[check: setup] " << listed->getErrorMessage();
        unsigned ready = 0;
        while (listed->hasNext()) {
            const auto row = listed->getNext();
            if (row->getValue(0)->getValue<std::string>() == "doc_index") {
                ASSERT_EQ(row->getValue(1)->getValue<std::string>(), "READY");
                ++ready;
            }
        }
        ASSERT_EQ(ready, 1u) << "[check: setup] expected exactly one ready target index";
    }

    void expectKnownZero() {
        auto result = conn->query("CALL SHOW_INDEXES() RETURN index_name, rows_outside_graph;");
        ASSERT_TRUE(result->isSuccess())
            << "[check: rows-outside-graph-is-readable] " << result->getErrorMessage();
        // La projection nommée ci-dessus est le premier rouge attendu sur la
        // base non corrigée. Ne pas le remplacer par une méthode C++ absente.
        auto schema = conn->query("CALL SHOW_INDEXES() RETURN *;");
        ASSERT_TRUE(schema->isSuccess()) << schema->getErrorMessage();
        const std::vector<std::string> names{"table_name", "index_name", "index_type",
            "property_names", "extension_loaded", "index_definition", "state",
            "extension_error", "rows_outside_graph"};
        ASSERT_EQ(schema->getColumnNames(), names) << "[check: stable-index-columns]";
        using common::LogicalType;
        // LogicalType expose le déplacement ; sa copie est explicite. Une
        // initializer_list demanderait la copie privée de ses éléments const.
        std::vector<LogicalType> expectedTypes;
        expectedTypes.reserve(names.size());
        expectedTypes.emplace_back(LogicalType::STRING());
        expectedTypes.emplace_back(LogicalType::STRING());
        expectedTypes.emplace_back(LogicalType::STRING());
        expectedTypes.emplace_back(LogicalType::LIST(LogicalType::STRING()));
        expectedTypes.emplace_back(LogicalType::BOOL());
        expectedTypes.emplace_back(LogicalType::STRING());
        expectedTypes.emplace_back(LogicalType::STRING());
        expectedTypes.emplace_back(LogicalType::STRING());
        expectedTypes.emplace_back(LogicalType::UINT64());
        const auto types = schema->getColumnDataTypes();
        ASSERT_EQ(types.size(), expectedTypes.size());
        for (size_t i = 0; i < types.size(); ++i) {
            EXPECT_EQ(types[i].toString(), expectedTypes[i].toString()) << names[i];
        }
        unsigned found = 0;
        while (result->hasNext()) {
            const auto row = result->getNext();
            if (row->getValue(0)->getValue<std::string>() != "doc_index") {
                continue;
            }
            ASSERT_FALSE(row->getValue(1)->isNull())
                << "[check: complete-index-has-known-count]";
            EXPECT_EQ(row->getValue(1)->getValue<uint64_t>(), 0u)
                << "[check: complete-index-has-no-outside-rows]";
            ++found;
        }
        EXPECT_EQ(found, 1u) << "[check: rows-outside-graph-target-was-checked]";
    }
};

TEST_F(VectorIndexDiagnosticsTest, ACompleteIndexReportsAKnownZero) {
    expectKnownZero();
}

TEST_F(VectorIndexDiagnosticsTest, ReadingTheCountInAReadOnlyTransactionTakesNoWriteLocks) {
    auto begin = conn->query("BEGIN TRANSACTION READ ONLY;");
    ASSERT_TRUE(begin->isSuccess()) << "[check: setup] " << begin->getErrorMessage();
    auto* context = getClientContext(*conn);
    auto* tx = transaction::Transaction::Get(*context);
    ASSERT_TRUE(tx->isReadOnly()) << "[check: setup] read-only transaction required";
    auto& locks = transaction::TransactionManager::Get(*context)->getLockManager();
    ASSERT_EQ(locks.getNumLocksHeld(tx->getID()), 0u);
    expectKnownZero();
    EXPECT_EQ(locks.getNumLocksHeld(tx->getID()), 0u)
        << "[check: diagnostic-takes-no-write-locks]";
    auto rollback = conn->query("ROLLBACK;");
    ASSERT_TRUE(rollback->isSuccess()) << rollback->getErrorMessage();
}

// Contre-exemple de fixture, pas reproduction d'un entrelacement réel : tout le
// graphe est déjà bâti, on ne change que la borne du balayage de secours. La vraie
// structure HNSW est utilisée, pas un miroir de sa disposition ni un hook global.
TEST_F(VectorIndexDiagnosticsTest, AScanRangeContainingAlreadyIndexedRowsIsNotAnOutsideCount) {
    auto reached = conn->query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', "
                               "[0.0, 1.0, 2.0, 3.0], 4) RETURN count(*);");
    ASSERT_TRUE(reached->isSuccess()) << "[check: setup] " << reached->getErrorMessage();
    ASSERT_TRUE(reached->hasNext());
    ASSERT_EQ(reached->getNext()->getValue(0)->getValue<int64_t>(), 4);
    auto begin = conn->query("BEGIN TRANSACTION READ ONLY;");
    ASSERT_TRUE(begin->isSuccess()) << "[check: setup] " << begin->getErrorMessage();
    auto* context = getClientContext(*conn);
    auto* tx = transaction::Transaction::Get(*context);
    auto* entry = catalog::Catalog::Get(*context)->getTableCatalogEntry(tx, "Doc");
    auto& table = storage::StorageManager::Get(*context)->getTable(entry->getTableID())
                      ->cast<storage::NodeTable>();
    auto indexes = table.captureVisibleIndexes("doc_index", tx);
    ASSERT_EQ(indexes.size(), 1u) << "[check: setup] target holder must be retained";
    ASSERT_EQ(indexes[0]->getIndexInfo().indexType, "HNSW");
    // Index HNSW réel créé par l'extension dans SetUp ; aucune concurrence dans
    // cette fixture. Restaurer la borne avant de rendre le contrôle au teardown.
    auto& info = const_cast<vector_extension::HNSWStorageInfo&>(
        static_cast<const vector_extension::HNSWStorageInfo&>(indexes[0]->getStorageInfo()));
    ASSERT_EQ(info.numCheckpointedNodes, 4u) << "[check: setup] all four rows must be indexed";
    struct RestoreWatermark {
        vector_extension::HNSWStorageInfo& info;
        common::offset_t saved;
        ~RestoreWatermark() { info.numCheckpointedNodes = saved; }
    };
    {
        const RestoreWatermark restore{info, info.numCheckpointedNodes};
        info.numCheckpointedNodes = 0;
        expectKnownZero();
    }
    auto rollback = conn->query("ROLLBACK;");
    ASSERT_TRUE(rollback->isSuccess()) << rollback->getErrorMessage();
}

TEST_F(VectorIndexDiagnosticsTest, DeletedRowsAndNullVectorsDoNotBecomeOutsideRowsAfterRebuild) {
    for (const auto* query : {"MATCH (d:Doc {id: 1}) DELETE d;",
             "MATCH (d:Doc {id: 2}) SET d.vec = NULL;",
             "CALL DROP_VECTOR_INDEX('Doc', 'doc_index');",
             "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');"}) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << "[check: setup] " << result->getErrorMessage();
    }
    auto count = conn->query("MATCH (d:Doc) WHERE d.vec IS NOT NULL RETURN count(*);");
    ASSERT_TRUE(count->isSuccess()) << "[check: setup] " << count->getErrorMessage();
    ASSERT_TRUE(count->hasNext());
    ASSERT_EQ(count->getNext()->getValue(0)->getValue<int64_t>(), 2);
    expectKnownZero();
}

} // namespace
} // namespace rag3db::testing
