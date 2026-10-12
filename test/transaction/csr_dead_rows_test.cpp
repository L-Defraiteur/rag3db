// La compaction des lignes mortes du CSR en mémoire (page coeur-cpp/13 §5, ticket « sans point de
// reprise, l'insertion sous HNSW ralentit lot après lot »).
//
// Une relation validée en mémoire puis supprimée restait dans la liste de son nœud (le CSR en
// mémoire, `NodeCSRIndex::rowIndices`) jusqu'au point de reprise : chaque balayage des voisins la
// relisait pour l'écarter. Désormais, quand plus aucun instantané ouvert ne peut la voir vivante
// (le plus ancien instantané actif, posé par le TransactionManager à la validation), la liste
// l'oublie — à la prochaine validation qui ajoute au nœud, et à la fin de la validation qui l'a
// supprimée. Les blocs gardent la ligne : un balayage en cours tient sa copie de la liste.
//
// Les témoins lisent la longueur de la liste en mémoire du nœud (CSRNodeGroup::getNumInMemRows),
// lignes mortes comprises.

#include <numeric>
#include <string>

#include "catalog/catalog.h"
#include "catalog/catalog_entry/rel_group_catalog_entry.h"
#include "common/enums/rel_direction.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "storage/storage_manager.h"
#include "storage/table/csr_node_group.h"
#include "storage/table/rel_table.h"
#include "transaction/transaction.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::storage;
using namespace rag3db::testing;

namespace {

class CSRDeadRows : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        reader = std::make_unique<Connection>(database.get());
    }

    void TearDown() override {
        reader.reset();
        EmptyDBTest::TearDown();
    }

    static void ok(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    void ok(const std::string& query) { ok(*conn, query); }

    static int64_t scalar(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        if (!result->isSuccess() || !result->hasNext()) {
            return -1;
        }
        return result->getNext()->getValue(0)->getValue<int64_t>();
    }
    int64_t count(Connection& connection) {
        return scalar(connection, "MATCH (:A)-[r:R]->(:B) RETURN count(r);");
    }
    int64_t sumK(Connection& connection) {
        return scalar(connection, "MATCH (:A)-[r:R]->(:B) RETURN sum(r.k);");
    }

    // La longueur de la liste en mémoire du nœud A d'offset 0, dans le sens avant de R.
    row_idx_t inMemRows() {
        auto* context = getClientContext(*conn);
        const auto tableID =
            rag3db::catalog::Catalog::Get(*context)
                ->getTableCatalogEntry(&rag3db::transaction::DUMMY_CHECKPOINT_TRANSACTION, "R")
                ->constCast<rag3db::catalog::RelGroupCatalogEntry>()
                .getSingleRelEntryInfo()
                .oid;
        auto& relTable = StorageManager::Get(*context)->getTable(tableID)->cast<RelTable>();
        auto* nodeGroup = relTable.getDirectedTableData(RelDataDirection::FWD)->getNodeGroup(0);
        return nodeGroup->cast<CSRNodeGroup>().getNumInMemRows(0);
    }

    // Un nœud A, `numB` nœuds B, et une relation de A vers chaque B avec k = id, validées après
    // le point de reprise : la liste de A est en mémoire.
    void setUp(int64_t numB) {
        ok("CREATE NODE TABLE A(id INT64 PRIMARY KEY);");
        ok("CREATE NODE TABLE B(id INT64 PRIMARY KEY);");
        ok("CREATE REL TABLE R(FROM A TO B, k INT64);");
        ok("CREATE (:A {id: 0});");
        ok("UNWIND range(0, " + std::to_string(numB - 1) + ") AS i CREATE (:B {id: i});");
        ok("CHECKPOINT;");
        ok("MATCH (a:A), (b:B) CREATE (a)-[:R {k: b.id}]->(b);");
        ASSERT_EQ(count(*conn), numB);
        ASSERT_EQ(inMemRows(), static_cast<row_idx_t>(numB));
    }

    std::unique_ptr<Connection> reader;
};

} // namespace

// Un lecteur ouvert avant les suppressions garde ses relations : tant qu'il est là, les listes
// gardent leurs lignes mortes (la compaction attend le plus ancien instantané) ; une fois parti,
// la validation suivante qui ajoute au nœud les oublie, et une suppression seule les oublie à
// la fin de sa propre validation.
TEST_F(CSRDeadRows, AnOldReaderKeepsItsRelsAndTheListShrinksAfterItLeaves) {
    setUp(1000);
    const int64_t sumBefore = 999 * 1000 / 2;
    ASSERT_EQ(sumK(*conn), sumBefore);
    ok(*reader, "BEGIN TRANSACTION READ ONLY;");
    ASSERT_EQ(count(*reader), 1000);
    auto round = [&](int64_t i) {
        const auto lo = std::to_string(200 * i);
        const auto hi = std::to_string(200 * (i + 1));
        ok("MATCH (:A)-[r:R]->(b:B) WHERE b.id >= " + lo + " AND b.id < " + hi + " DELETE r;");
        ok("MATCH (a:A), (b:B) WHERE b.id >= " + lo + " AND b.id < " + hi +
           " CREATE (a)-[:R {k: b.id + 1000}]->(b);");
    };
    for (int64_t i = 0; i < 3; i++) {
        round(i);
        EXPECT_EQ(count(*reader), 1000) << "[check: old-reader-count] round " << i;
        EXPECT_EQ(sumK(*reader), sumBefore) << "[check: old-reader-sum] round " << i;
        EXPECT_EQ(count(*conn), 1000) << "[check: writer-count] round " << i;
        EXPECT_EQ(inMemRows(), 1000u + 200u * (i + 1))
            << "[check: dead-rows-kept-while-reader-open] round " << i;
    }
    ok(*reader, "COMMIT;");
    // La validation de la création du tour 3 ajoute au nœud : les 600 mortes d'avant et les 200
    // que ce tour vient de supprimer partent (plus aucun instantané ne les voit).
    round(3);
    EXPECT_EQ(inMemRows(), 1000u) << "[check: compacted-at-the-next-append]";
    EXPECT_EQ(count(*conn), 1000);
    // Une suppression seule : ses lignes partent à la fin de sa validation.
    ok("MATCH (:A)-[r:R]->(b:B) WHERE b.id >= 800 DELETE r;");
    EXPECT_EQ(inMemRows(), 800u) << "[check: compacted-after-the-deleting-commit]";
    EXPECT_EQ(count(*conn), 800);
    ok("MATCH (a:A), (b:B) WHERE b.id >= 800 CREATE (a)-[:R {k: b.id + 1000}]->(b);");
    EXPECT_EQ(inMemRows(), 1000u);
    const int64_t sumAfter = sumBefore + 1000 * 1000;
    EXPECT_EQ(count(*conn), 1000);
    EXPECT_EQ(sumK(*conn), sumAfter) << "[check: final-sum]";
    EXPECT_EQ(count(*reader), 1000) << "[check: new-reader-count]";
    EXPECT_EQ(sumK(*reader), sumAfter);
    ok("CHECKPOINT;");
    EXPECT_EQ(count(*conn), 1000) << "[check: after-checkpoint]";
    EXPECT_EQ(sumK(*conn), sumAfter);
    reader.reset();
    createDBAndConn();
    EXPECT_EQ(count(*conn), 1000) << "[check: after-reopen]";
    EXPECT_EQ(sumK(*conn), sumAfter);
}

// La forme d'un rétrécissement HNSW : dans une même transaction, les relations du nœud sont
// supprimées puis recréées. Sans compaction la liste grandit de 64 par tour ; avec, elle revient
// à 64 à la fin de chaque validation — sauf pendant qu'un lecteur plus ancien est ouvert, et elle
// revient à 64 dès la validation qui suit son départ.
TEST_F(CSRDeadRows, DeleteAndRecreateInOneTransactionLikeAShrink) {
    setUp(64);
    auto shrink = [&](int64_t round) {
        ok("BEGIN TRANSACTION;");
        ok("MATCH (:A)-[r:R]->(:B) DELETE r;");
        ok("MATCH (a:A), (b:B) CREATE (a)-[:R {k: b.id + " + std::to_string(round) + "}]->(b);");
        ok("COMMIT;");
    };
    for (int64_t round = 1; round < 10; round++) {
        shrink(round);
        EXPECT_EQ(inMemRows(), 64u) << "[check: list-stays-at-64] round " << round;
        EXPECT_EQ(count(*conn), 64) << "round " << round;
    }
    ok(*reader, "BEGIN TRANSACTION READ ONLY;");
    const int64_t readerSum = 63 * 64 / 2 + 9 * 64;
    ASSERT_EQ(sumK(*reader), readerSum);
    for (int64_t round = 10; round < 20; round++) {
        shrink(round);
        EXPECT_EQ(inMemRows(), 64u * (round - 10 + 2))
            << "[check: list-grows-while-reader-open] round " << round;
        EXPECT_EQ(sumK(*reader), readerSum) << "[check: old-reader-sum] round " << round;
        EXPECT_EQ(count(*conn), 64) << "round " << round;
    }
    ok(*reader, "COMMIT;");
    for (int64_t round = 20; round < 30; round++) {
        shrink(round);
        EXPECT_EQ(inMemRows(), 64u) << "[check: list-back-to-64-after-reader-left] round "
                                    << round;
        EXPECT_EQ(count(*conn), 64) << "round " << round;
    }
    const int64_t finalSum = 63 * 64 / 2 + 29 * 64;
    EXPECT_EQ(sumK(*conn), finalSum) << "[check: final-sum]";
    ok("CHECKPOINT;");
    EXPECT_EQ(sumK(*conn), finalSum) << "[check: after-checkpoint]";
    reader.reset();
    createDBAndConn();
    EXPECT_EQ(count(*conn), 64) << "[check: after-reopen]";
    EXPECT_EQ(sumK(*conn), finalSum);
}
