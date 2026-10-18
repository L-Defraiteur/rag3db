// Une colonne ajoutée puis annulée (ticket « une colonne ajoutée puis annulée reste au stockage,
// et l'ajout suivant la relit à sa place », E10 de l'étude des migrations concurrentes).
//
// Le catalogue est versionné : un ALTER copie l'entrée, la copie prend l'indice de colonne N, et
// l'annulation la retire de la chaîne, si bien que l'entrée d'origine rend N à l'ADD suivant. Le
// stockage, lui, ne s'annulait pas : la colonne de l'ADD annulé restait à l'indice N, et l'ADD
// suivant, rangé en N + 1, se lisait et s'écrivait dans la colonne morte — sa valeur par défaut,
// son type. Le point de reprise gardait la morte et jetait la vraie. Désormais l'ADD laisse un
// enregistrement dans le tampon d'annulation et le ROLLBACK retire la colonne du stockage ; et
// deux ALTER d'une même table ne se chevauchent plus (conflit écrivain-écrivain, comme un CREATE).
//
// Le banc joue la même chose à vingt clients (minute 6 du scénario des migrations) : ses
// contrôles `added-column-reads-its-default` et `added-column-reads-what-was-set`.

#include <memory>
#include <string>

#include "catalog/catalog.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
#include "transaction/transaction.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::storage;
using namespace rag3db::testing;

namespace {

class AddColumnRollback : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        ok("CREATE NODE TABLE T(id INT64 PRIMARY KEY);");
        ok("CREATE REL TABLE R(FROM T TO T);");
        ok("CREATE (:T {id: 1});");
        ok("MATCH (a:T) CREATE (a)-[:R]->(a);");
    }

    static void ok(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    void ok(const std::string& query) { ok(*conn, query); }

    static std::string error(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        return result->isSuccess() ? "" : result->getErrorMessage();
    }

    // La première valeur de la première ligne, en texte.
    static std::string first(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        if (!result->isSuccess()) {
            return "<erreur : " + result->getErrorMessage() + ">";
        }
        if (!result->hasNext()) {
            return "<vide>";
        }
        return result->getNext()->getValue(0)->toString();
    }
    std::string first(const std::string& query) { return first(*conn, query); }

    // Le nombre de colonnes de T dans le stockage, celles que le catalogue ne nomme plus comprises.
    column_id_t storedColumnsOfT() {
        auto* context = getClientContext(*conn);
        const auto tableID =
            rag3db::catalog::Catalog::Get(*context)
                ->getTableCatalogEntry(&rag3db::transaction::DUMMY_CHECKPOINT_TRANSACTION, "T")
                ->getTableID();
        return StorageManager::Get(*context)->getTable(tableID)->cast<NodeTable>().getNumColumns();
    }

    // `query` rend `expected` tout de suite, après un point de reprise, puis après réouverture.
    void expectThroughCheckpointAndReopen(const std::string& query, const std::string& expected) {
        EXPECT_EQ(first(query), expected) << "avant le point de reprise";
        ok("CHECKPOINT;");
        EXPECT_EQ(first(query), expected) << "après le point de reprise";
        if (inMemMode) {
            return;
        }
        createDBAndConn();
        EXPECT_EQ(first(query), expected) << "après réouverture";
    }
};

TEST_F(AddColumnRollback, TheColumnReaddedAfterARollbackReadsItsOwnDefault) {
    ok("BEGIN TRANSACTION;");
    ok("ALTER TABLE T ADD b INT64 DEFAULT 1;");
    ok("ROLLBACK;");
    ok("ALTER TABLE T ADD b INT64 DEFAULT 2;");
    expectThroughCheckpointAndReopen("MATCH (t:T) RETURN t.b;", "2");
}

TEST_F(AddColumnRollback, TheColumnReaddedWithAnotherTypeReadsItsOwnType) {
    ok("BEGIN TRANSACTION;");
    ok("ALTER TABLE T ADD b INT64 DEFAULT 1;");
    ok("ROLLBACK;");
    ok("ALTER TABLE T ADD b STRING DEFAULT 'deux';");
    expectThroughCheckpointAndReopen("MATCH (t:T) RETURN t.b;", "deux");
}

TEST_F(AddColumnRollback, AnotherNameAfterTheRollbackReadsItsOwnDefaultToo) {
    ok("BEGIN TRANSACTION;");
    ok("ALTER TABLE T ADD b INT64 DEFAULT 1;");
    ok("ROLLBACK;");
    ok("ALTER TABLE T ADD c INT64 DEFAULT 3;");
    expectThroughCheckpointAndReopen("MATCH (t:T) RETURN t.c;", "3");
    EXPECT_NE(error(*conn, "MATCH (t:T) RETURN t.b;"), "");
}

TEST_F(AddColumnRollback, ARolledBackAddLeavesNoColumnInTheStorage) {
    const auto before = storedColumnsOfT();
    ok("BEGIN TRANSACTION;");
    ok("ALTER TABLE T ADD b INT64 DEFAULT 1;");
    EXPECT_EQ(storedColumnsOfT(), before + 1);
    ok("ROLLBACK;");
    EXPECT_EQ(storedColumnsOfT(), before);
    ok("ALTER TABLE T ADD c INT64 DEFAULT 3;");
    EXPECT_EQ(storedColumnsOfT(), before + 1);
}

TEST_F(AddColumnRollback, TheSameHoldsForARelTable) {
    ok("BEGIN TRANSACTION;");
    ok("ALTER TABLE R ADD w INT64 DEFAULT 1;");
    ok("ROLLBACK;");
    ok("ALTER TABLE R ADD w INT64 DEFAULT 2;");
    expectThroughCheckpointAndReopen("MATCH ()-[r:R]->() RETURN r.w;", "2");
}

TEST_F(AddColumnRollback, RowsInsertedBeforeTheRolledBackAddGoWithIt) {
    ok("BEGIN TRANSACTION;");
    ok("CREATE (:T {id: 2});");
    ok("ALTER TABLE T ADD b INT64 DEFAULT 1;");
    ok("ROLLBACK;");
    EXPECT_EQ(first("MATCH (t:T) RETURN count(*);"), "1");
    ok("ALTER TABLE T ADD b INT64 DEFAULT 2;");
    ok("CREATE (:T {id: 2});");
    expectThroughCheckpointAndReopen("MATCH (t:T) RETURN sum(t.b);", "4");
}

TEST_F(AddColumnRollback, TwoAltersOfOneTableDoNotOverlap) {
    auto other = std::make_unique<Connection>(database.get());
    ok("BEGIN TRANSACTION;");
    ok("ALTER TABLE T ADD b INT64 DEFAULT 1;");
    // L'autre ne voit pas b, mais prendrait son indice : refusé par un nom, comme un CREATE.
    const auto refused = error(*other, "ALTER TABLE T ADD c INT64 DEFAULT 3;");
    EXPECT_NE(refused.find("Write-write conflict"), std::string::npos) << refused;
    ok("COMMIT;");
    ok(*other, "ALTER TABLE T ADD c INT64 DEFAULT 3;");
    EXPECT_EQ(first("MATCH (t:T) RETURN t.b + t.c;"), "4");
    other.reset();
}

} // namespace
