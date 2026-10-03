// Le témoin rouge du vérificateur. Un vérificateur qui a déjà rendu un faux vert (C2,
// étape 1 : il lisait une propriété de l'extrémité supprimée) a besoin d'un cas où le
// rouge est fabriqué à coup sûr, indépendamment des défauts du moteur que le banc
// cherche — sinon, le jour où une marche corrige C2, plus rien ne prouverait que le
// vérificateur voit une relation pendante.
//
// Le nœud est supprimé par NodeTable::delete_, sans passer par l'exécuteur : le
// contrôle des relations attachées (throwIfNodeHasRels) n'a donc pas lieu.

#include <iostream>

#include "catalog/catalog.h"
#include "catalog/catalog_entry/table_catalog_entry.h"
#include "common/data_chunk/data_chunk_state.h"
#include "common/vector/value_vector.h"
#include "graph_test/private_graph_test.h"
#include "integrity/integrity_checker.h"
#include "main/client_context.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
#include "transaction/transaction.h"

using namespace rag3db::testing;

namespace {

bool contains(const std::vector<integrity::Violation>& violations, const std::string& invariant) {
    for (const auto& violation : violations) {
        if (violation.invariant == invariant) {
            return true;
        }
    }
    return false;
}

class IntegrityCheckerWitness : public EmptyDBTest {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
    }

    void mustRun(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << "\n" << result->getErrorMessage();
    }
};

// Supprime le nœud de clé deletedKey par NodeTable::delete_, puis exige que chaque
// niveau voie la relation 1 -> 2 pendante.
void deleteBehindTheExecutorAndCheck(IntegrityCheckerWitness& test, int64_t deletedKey) {
    auto& conn = test.conn;
    test.mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, writer INT64);");
    test.mustRun("CREATE REL TABLE Link(FROM Item TO Item, src_id INT64, dst_id INT64);");
    test.mustRun("CREATE (:Item {id: 1, writer: 0}), (:Item {id: 2, writer: 0});");
    test.mustRun("MATCH (a:Item {id: 1}), (b:Item {id: 2}) "
                 "CREATE (a)-[:Link {src_id: 1, dst_id: 2}]->(b);");
    // Avant : une base saine, les deux niveaux n'ont rien à dire.
    EXPECT_TRUE(integrity::checkLevel1(*conn).empty());
    EXPECT_TRUE(integrity::checkLevel2(*conn).empty());

    test.mustRun("BEGIN TRANSACTION;");
    {
        auto* context = conn->getClientContext();
        auto* transaction = rag3db::transaction::Transaction::Get(*context);
        const auto* entry =
            rag3db::catalog::Catalog::Get(*context)->getTableCatalogEntry(transaction, "Item");
        auto& nodeTable = rag3db::storage::StorageManager::Get(*context)
                              ->getTable(entry->getTableID())
                              ->cast<rag3db::storage::NodeTable>();
        auto* mm = rag3db::storage::MemoryManager::Get(*context);
        const auto state = rag3db::common::DataChunkState::getSingleValueDataChunkState();
        rag3db::common::ValueVector nodeID(rag3db::common::LogicalType::INTERNAL_ID(), mm, state);
        rag3db::common::ValueVector key(rag3db::common::LogicalType::INT64(), mm, state);
        key.setValue<int64_t>(0, deletedKey);
        rag3db::common::offset_t offset = rag3db::common::INVALID_OFFSET;
        ASSERT_TRUE(nodeTable.lookupPK(transaction, &key, 0, offset));
        nodeID.setValue<rag3db::common::nodeID_t>(0, {offset, nodeTable.getTableID()});
        rag3db::storage::NodeTableDeleteState deleteState(nodeID, key);
        ASSERT_TRUE(nodeTable.delete_(transaction, deleteState));
        nodeTable.finalizeDelete(transaction, deleteState);
    }
    test.mustRun("COMMIT;");

    // Après : la relation 1 -> 2 pend, et chaque niveau doit le dire.
    const auto level1 = integrity::checkLevel1(*conn);
    const auto level2 = integrity::checkLevel2(*conn);
    std::cerr << "[witness] level 1:\n"
              << integrity::describe(level1) << "[witness] level 2:\n"
              << integrity::describe(level2);
    EXPECT_TRUE(contains(level1, "rel-endpoints-exist"));
    EXPECT_TRUE(contains(level1, "rel-directions-agree"));
    EXPECT_TRUE(contains(level2, "stored-endpoints-visible"));
}

TEST_F(IntegrityCheckerWitness, SeesARelationWhoseSourceWasDeletedBehindTheExecutor) {
    deleteBehindTheExecutorAndCheck(*this, 1);
}

// La destination : l'autre direction de la CSR. Ce chemin a déjà caché une relation
// pendante au niveau 1 (3 octobre, avant l'indication de jointure du balayage).
TEST_F(IntegrityCheckerWitness, SeesARelationWhoseDestinationWasDeletedBehindTheExecutor) {
    deleteBehindTheExecutorAndCheck(*this, 2);
}

} // namespace
