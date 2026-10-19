// V2-1a : les numéros de colonnes ne changent jamais. Une colonne supprimée laisse un trou à son
// numéro (chunk nul, type ANY, colonne nulle) après le point de reprise ; les autres gardent le
// leur, un ADD prend le numéro suivant, et tout se relit après réouverture (format 42). Les quatre
// défauts lus en cartographiant les numéros (tickets du 19 octobre 2026) ont chacun leur cas.
#include <filesystem>
#include <string>

#include "catalog/catalog.h"
#include "catalog/catalog_entry/table_catalog_entry.h"
#include "graph_test/private_graph_test.h"
#include "main/client_context.h"
#include "main/connection.h"
#include "main/database.h"
#include "test_helper/test_helper.h"
#include "transaction/transaction.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class StableColumnIDs : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
    }

    void run(const std::string& q) const {
        auto result = conn->query(q);
        ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
    }
    std::string rows(const std::string& q) const {
        auto result = conn->query(q);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        std::string out;
        while (result->hasNext()) {
            out += result->getNext()->toString();
        }
        return out;
    }
    // Le numéro de colonne d'une propriété, lu au catalogue (dernier état validé, comme le point de reprise le lit : hors requête, la
    // connexion n'a pas de transaction).
    column_id_t columnID(const std::string& table, const std::string& property) const {
        auto* context = conn->getClientContext();
        auto* entry = rag3db::catalog::Catalog::Get(*context)->getTableCatalogEntry(
            &rag3db::transaction::DUMMY_CHECKPOINT_TRANSACTION, table);
        return entry->getColumnID(property);
    }
    column_id_t numColumnSlots(const std::string& table) const {
        auto* context = conn->getClientContext();
        auto* entry = rag3db::catalog::Catalog::Get(*context)->getTableCatalogEntry(
            &rag3db::transaction::DUMMY_CHECKPOINT_TRANSACTION, table);
        // Le nombre de numéros attribués, trous compris (écrit avec getMaxColumnID pour que le
        // témoin compile aussi sur un moteur d'avant : sa dernière colonne n'est jamais supprimée).
        return entry->getMaxColumnID() + 1;
    }
    void reopen() {
        conn.reset();
        database.reset();
        createDBAndConn();
    }
};

TEST_F(StableColumnIDs, ADroppedColumnLeavesItsNumberAndTheOthersKeepTheirs) {
    run("CREATE NODE TABLE T(id INT64 PRIMARY KEY, a INT64, b STRING, c DOUBLE);");
    run("UNWIND range(1, 5000) AS i CREATE (:T {id: i, a: i * 2, b: 'b' + CAST(i AS STRING), c: "
        "i * 0.5});");
    run("CHECKPOINT;");
    const auto idA = columnID("T", "a"), idB = columnID("T", "b"), idC = columnID("T", "c");
    EXPECT_EQ(idA, 1u);
    EXPECT_EQ(idB, 2u);
    EXPECT_EQ(idC, 3u);
    run("ALTER TABLE T DROP b;");
    run("CHECKPOINT;");
    EXPECT_EQ(columnID("T", "a"), idA);
    EXPECT_EQ(columnID("T", "c"), idC) << "the column after the dropped one kept its number";
    EXPECT_EQ(numColumnSlots("T"), 4u) << "the dropped number stays a hole";
    EXPECT_EQ(rows("MATCH (t:T {id: 4000}) RETURN t.a, t.c;"), "8000|2000.000000\n");
    if (!inMemMode) {
        reopen();
        EXPECT_EQ(columnID("T", "c"), idC);
        EXPECT_EQ(rows("MATCH (t:T {id: 4000}) RETURN t.a, t.c;"), "8000|2000.000000\n");
        EXPECT_EQ(rows("MATCH (t:T) RETURN count(*);"), "5000\n");
    }
    run("ALTER TABLE T ADD d INT64 DEFAULT 7;");
    EXPECT_EQ(columnID("T", "d"), 4u) << "an added column takes the next number, never a hole";
    EXPECT_EQ(rows("MATCH (t:T {id: 4000}) RETURN t.a, t.c, t.d;"), "8000|2000.000000|7\n");
    run("MATCH (t:T {id: 1}) SET t.d = 9;");
    run("CHECKPOINT;");
    EXPECT_EQ(rows("MATCH (t:T {id: 1}) RETURN t.a, t.c, t.d;"), "2|0.500000|9\n");
    if (!inMemMode) {
        reopen();
        EXPECT_EQ(rows("MATCH (t:T {id: 1}) RETURN t.a, t.c, t.d;"), "2|0.500000|9\n");
        EXPECT_EQ(rows("MATCH (t:T {id: 4000}) RETURN t.d;"), "7\n");
    }
}

// La colonne 0 devient un trou : la première propriété déclarée, avant la clé, est supprimée. Le
// bloc relu prend son nombre de lignes sur une colonne présente (le banc : DropBeforeThePrimaryKey,
// AttachedDatabaseIsReadWithItsOwnCatalog).
TEST_F(StableColumnIDs, DroppingTheFirstColumnBeforeTheKeyKeepsEveryRow) {
    run("CREATE NODE TABLE Item(extra STRING, id INT64, name STRING, PRIMARY KEY (id));");
    run("UNWIND range(0, 99) AS i CREATE (:Item {extra: 'e', id: i, name: 'item ' + CAST(i AS "
        "STRING)});");
    run("CHECKPOINT;");
    run("ALTER TABLE Item DROP extra;");
    run("UNWIND range(100, 199) AS i CREATE (:Item {id: i, name: 'item ' + CAST(i AS STRING)});");
    run("CHECKPOINT;");
    EXPECT_EQ(rows("MATCH (n:Item) RETURN count(*);"), "200\n");
    EXPECT_EQ(rows("MATCH (n:Item {id: 50}) RETURN n.name;"), "item 50\n");
    EXPECT_EQ(rows("MATCH (n:Item {id: 150}) RETURN n.name;"), "item 150\n");
    if (!inMemMode) {
        reopen();
        EXPECT_EQ(rows("MATCH (n:Item) RETURN count(*);"), "200\n");
        EXPECT_EQ(rows("MATCH (n:Item {id: 50}) RETURN n.name;"), "item 50\n");
        run("CREATE (:Item {id: 200, name: 'item 200'});");
        run("CHECKPOINT;");
        reopen();
        EXPECT_EQ(rows("MATCH (n:Item) RETURN count(*);"), "201\n");
        EXPECT_EQ(rows("MATCH (n:Item {id: 200}) RETURN n.name;"), "item 200\n");
    }
}

// Le même trou en colonne 0, sur un groupe encore tout en mémoire au point de reprise (aucun
// point de reprise avant le DROP) : la copie vidée reprend ses numéros avec les trous, pas avec
// l'ancien type (le banc : AttachedDatabaseIsReadWithItsOwnCatalog).
TEST_F(StableColumnIDs, DroppingTheFirstColumnWithoutAPriorCheckpointKeepsEveryRow) {
    run("CREATE NODE TABLE Item(extra STRING, id INT64, name STRING, PRIMARY KEY (id));");
    run("UNWIND range(0, 99) AS i CREATE (:Item {extra: 'e', id: i, name: 'item ' + CAST(i AS "
        "STRING)});");
    run("ALTER TABLE Item DROP extra;");
    run("CHECKPOINT;");
    EXPECT_EQ(rows("MATCH (n:Item) RETURN count(*);"), "100\n");
    EXPECT_EQ(rows("MATCH (n:Item {id: 50}) RETURN n.name;"), "item 50\n");
    if (!inMemMode) {
        reopen();
        EXPECT_EQ(rows("MATCH (n:Item) RETURN count(*);"), "100\n");
        EXPECT_EQ(rows("MATCH (n:Item {id: 50}) RETURN n.name;"), "item 50\n");
    }
}

// Ticket « les statistiques par colonne ne sont pas compactées au point de reprise après un DROP ».
TEST_F(StableColumnIDs, TheRemainingColumnsKeepTheirOwnStatistics) {
    run("CREATE NODE TABLE T(id INT64 PRIMARY KEY, a INT64, b INT64);");
    run("UNWIND range(1, 1000) AS i CREATE (:T {id: i, a: i, b: i % 2});");
    run("CHECKPOINT;");
    auto before = conn->query("CALL STATS_INFO('T') RETURN b_distinct_count;");
    ASSERT_TRUE(before->isSuccess()) << before->getErrorMessage();
    const auto distinctB = before->getNext()->getValue(0)->getValue<int64_t>();
    EXPECT_LE(distinctB, 4) << "b has two values";
    run("ALTER TABLE T DROP a;");
    run("CHECKPOINT;");
    auto after = conn->query("CALL STATS_INFO('T') RETURN b_distinct_count;");
    ASSERT_TRUE(after->isSuccess()) << after->getErrorMessage();
    EXPECT_EQ(after->getNext()->getValue(0)->getValue<int64_t>(), distinctB)
        << "b must not inherit a's statistics";
    if (!inMemMode) {
        reopen();
        auto reread = conn->query("CALL STATS_INFO('T') RETURN b_distinct_count;");
        ASSERT_TRUE(reread->isSuccess()) << reread->getErrorMessage();
        EXPECT_EQ(reread->getNext()->getValue(0)->getValue<int64_t>(), distinctB);
    }
}

// Ticket « un groupe vidé par le point de reprise garde la colonne supprimée ».
TEST_F(StableColumnIDs, AGroupEmptiedByTheCheckpointKeepsTheHole) {
    run("CREATE NODE TABLE T(id INT64 PRIMARY KEY, a INT64, b STRING);");
    run("UNWIND range(1, 10) AS i CREATE (:T {id: i, a: i, b: 'x'});");
    run("CHECKPOINT;");
    run("MATCH (t:T) DELETE t;");
    run("ALTER TABLE T DROP a;");
    run("CHECKPOINT;");
    if (!inMemMode) {
        reopen();
    }
    run("CREATE (:T {id: 11, b: 'y'});");
    EXPECT_EQ(rows("MATCH (t:T) RETURN t.id, t.b;"), "11|y\n");
    run("CHECKPOINT;");
    if (!inMemMode) {
        reopen();
        EXPECT_EQ(rows("MATCH (t:T) RETURN t.id, t.b;"), "11|y\n");
    }
}

// Ticket « un ADD COLUMN sur une relation, après le DROP de sa dernière propriété, range la
// colonne locale au mauvais numéro ».
TEST_F(StableColumnIDs, AnAddOnARelationAfterDroppingItsLastPropertyWritesTheRightColumn) {
    run("CREATE NODE TABLE A(id INT64 PRIMARY KEY); CREATE NODE TABLE B(id INT64 PRIMARY KEY);");
    run("CREATE REL TABLE R(FROM A TO B, p INT64, q INT64);");
    run("CREATE (:A {id: 1}); CREATE (:B {id: 2});");
    run("ALTER TABLE R DROP q;");
    run("BEGIN TRANSACTION;");
    run("ALTER TABLE R ADD r INT64;");
    run("MATCH (a:A), (b:B) CREATE (a)-[:R {p: 1, r: 7}]->(b);");
    EXPECT_EQ(rows("MATCH ()-[x:R]->() RETURN x.p, x.r;"), "1|7\n");
    run("COMMIT;");
    EXPECT_EQ(rows("MATCH ()-[x:R]->() RETURN x.p, x.r;"), "1|7\n");
    run("CHECKPOINT;");
    if (!inMemMode) {
        reopen();
        EXPECT_EQ(rows("MATCH ()-[x:R]->() RETURN x.p, x.r;"), "1|7\n");
    }
}

TEST_F(StableColumnIDs, ADroppedRelationPropertyLeavesAHole) {
    run("CREATE NODE TABLE A(id INT64 PRIMARY KEY); CREATE NODE TABLE B(id INT64 PRIMARY KEY);");
    run("CREATE REL TABLE R(FROM A TO B, p INT64, q STRING, r INT64);");
    run("UNWIND range(1, 300) AS i CREATE (:A {id: i});");
    run("UNWIND range(1, 300) AS i CREATE (:B {id: i});");
    run("MATCH (a:A), (b:B) WHERE a.id = b.id CREATE (a)-[:R {p: a.id, q: 'q', r: a.id * "
        "10}]->(b);");
    run("CHECKPOINT;");
    const auto idP = columnID("R", "p"), idR = columnID("R", "r");
    run("ALTER TABLE R DROP q;");
    run("CHECKPOINT;");
    EXPECT_EQ(columnID("R", "p"), idP);
    EXPECT_EQ(columnID("R", "r"), idR);
    EXPECT_EQ(rows("MATCH (a:A {id: 42})-[x:R]->() RETURN x.p, x.r;"), "42|420\n");
    EXPECT_EQ(rows("MATCH ()-[x:R]->() RETURN count(*);"), "300\n");
    if (!inMemMode) {
        reopen();
        EXPECT_EQ(rows("MATCH (a:A {id: 42})-[x:R]->() RETURN x.p, x.r;"), "42|420\n");
        EXPECT_EQ(rows("MATCH ()-[x:R]->() RETURN count(*);"), "300\n");
    }
    run("ALTER TABLE R ADD s STRING DEFAULT 's';");
    run("MATCH (a:A {id: 42})-[x:R]->() SET x.s = 'z';");
    run("CHECKPOINT;");
    EXPECT_EQ(rows("MATCH (a:A {id: 42})-[x:R]->() RETURN x.p, x.r, x.s;"), "42|420|z\n");
    EXPECT_EQ(rows("MATCH (a:A {id: 43})-[x:R]->() RETURN x.s;"), "s\n");
    if (!inMemMode) {
        reopen();
        EXPECT_EQ(rows("MATCH (a:A {id: 42})-[x:R]->() RETURN x.p, x.r, x.s;"), "42|420|z\n");
    }
}

// L'annulation d'un ADD (E10) après un trou : la colonne annulée est la dernière, le trou reste.
TEST_F(StableColumnIDs, RollingBackAnAddAfterADrop) {
    run("CREATE NODE TABLE T(id INT64 PRIMARY KEY, a INT64, b INT64);");
    run("UNWIND range(1, 100) AS i CREATE (:T {id: i, a: i, b: i});");
    run("ALTER TABLE T DROP a;");
    run("CHECKPOINT;");
    run("BEGIN TRANSACTION;");
    run("ALTER TABLE T ADD e INT64 DEFAULT 1;");
    run("ROLLBACK;");
    EXPECT_EQ(numColumnSlots("T"), 3u);
    run("ALTER TABLE T ADD f INT64 DEFAULT 2;");
    EXPECT_EQ(columnID("T", "f"), 3u);
    EXPECT_EQ(rows("MATCH (t:T {id: 50}) RETURN t.b, t.f;"), "50|2\n");
    run("CHECKPOINT;");
    if (!inMemMode) {
        reopen();
        EXPECT_EQ(rows("MATCH (t:T {id: 50}) RETURN t.b, t.f;"), "50|2\n");
    }
}

// Ticket « le cache de colonne du CREATE_VECTOR_INDEX est clé par identifiant de propriété ».
TEST_F(StableColumnIDs, AVectorIndexIsCreatedOnATableWhereAPropertyWasDropped) {
    const auto extension =
        TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
    if (!std::filesystem::exists(extension)) {
        GTEST_SKIP() << "no vector extension built";
    }
    run("LOAD EXTENSION '" + extension + "';");
    run("CREATE NODE TABLE D(id INT64 PRIMARY KEY, junk INT64, vec FLOAT[4]);");
    run("CREATE (:D {id: 1, junk: 0, vec: [1.0, 0.0, 0.0, 0.0]});");
    run("CREATE (:D {id: 2, junk: 0, vec: [0.0, 1.0, 0.0, 0.0]});");
    run("ALTER TABLE D DROP junk;");
    run("CHECKPOINT;");
    run("CALL CREATE_VECTOR_INDEX('D', 'idx', 'vec');");
    EXPECT_EQ(rows("CALL QUERY_VECTOR_INDEX('D', 'idx', CAST([1.0, 0.0, 0.0, 0.0] AS FLOAT[4]), "
                   "1) RETURN node.id;"),
        "1\n");
}

} // namespace
