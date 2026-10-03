// La transaction en échec (marche T0 de docs/3-octobre-2026-15h47/01-note-de-conception-
// les-verrous.md). Quand une instruction échoue entre BEGIN et COMMIT, la transaction est
// annulée — et le bloc reste ouvert : toute instruction y est refusée jusqu'au ROLLBACK.
// Avant, la connexion repassait en auto-commit sans rien dire, et les instructions
// suivantes étaient validées une à une : un client qui enchaînait sans lire chaque
// résultat perdait l'atomicité en silence. PostgreSQL refuse de même (« current
// transaction is aborted, commands ignored until end of transaction block ») ; Neo4j aussi.

#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/prepared_statement.h"
#include "transaction/transaction_context.h"

using namespace rag3db::testing;
using rag3db::transaction::TransactionContext;

namespace {

class FailedTransactionTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        ok("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);");
        ok("CREATE (:Item {id: 1, v: 0});");
    }

    void ok(const std::string& query) { ok(*conn, query); }

    static void ok(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    // L'instruction échoue, pour une raison qui n'est PAS le refus.
    void fails(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_FALSE(result->isSuccess()) << query;
        EXPECT_EQ(result->getErrorMessage().find(TransactionContext::MANUAL_TRANSACTION_FAILED),
            std::string::npos)
            << query << " : " << result->getErrorMessage();
    }

    // L'instruction est refusée par le nom du refus.
    void refused(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_FALSE(result->isSuccess()) << query << " a été servie dans un bloc en échec";
        EXPECT_NE(result->getErrorMessage().find(TransactionContext::MANUAL_TRANSACTION_FAILED),
            std::string::npos)
            << query << " : " << result->getErrorMessage();
    }

    int64_t single(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }
};

TEST_F(FailedTransactionTest, EverythingIsRefusedUntilRollback) {
    ok("BEGIN TRANSACTION;");
    ok("CREATE (:Item {id: 2, v: 0});");
    fails("CREATE (:Item {id: 1, v: 9});"); // clé en double
    refused("CREATE (:Item {id: 3, v: 0});");
    refused("MATCH (n:Item {id: 1}) SET n.v = 5;");
    refused("MATCH (n:Item) RETURN count(n);");
    refused("BEGIN TRANSACTION;");
    refused("CHECKPOINT;");
    ok("ROLLBACK;");
    // Rien de la transaction ne reste, pas même ce qui précédait l'erreur, et rien de ce
    // qui la suivait n'a été validé.
    EXPECT_EQ(single("MATCH (n:Item) RETURN count(n);"), 1);
    EXPECT_EQ(single("MATCH (n:Item {id: 1}) RETURN n.v;"), 0);
    // La connexion sert de nouveau, en auto-commit comme dans un nouveau bloc.
    ok("CREATE (:Item {id: 4, v: 0});");
    ok("BEGIN TRANSACTION;");
    ok("CREATE (:Item {id: 5, v: 0});");
    ok("COMMIT;");
    EXPECT_EQ(single("MATCH (n:Item) RETURN count(n);"), 3);
}

TEST_F(FailedTransactionTest, CommitClosesTheBlockAndSaysNothingWasCommitted) {
    ok("BEGIN TRANSACTION;");
    ok("CREATE (:Item {id: 2, v: 0});");
    fails("CREATE (:Item {id: 1, v: 9});");
    // COMMIT ne peut rien valider : il le dit, et ferme le bloc.
    refused("COMMIT;");
    EXPECT_EQ(single("MATCH (n:Item) RETURN count(n);"), 1);
    ok("CREATE (:Item {id: 2, v: 0});");
}

TEST_F(FailedTransactionTest, AnErrorFoundBeforeExecutionFailsTheBlockToo) {
    ok("BEGIN TRANSACTION;");
    ok("CREATE (:Item {id: 2, v: 0});");
    fails("MATCH (n:Item) RETURN n.no_such_property;"); // refusée à la liaison
    refused("CREATE (:Item {id: 3, v: 0});");
    ok("ROLLBACK;");
    EXPECT_EQ(single("MATCH (n:Item) RETURN count(n);"), 1);
}

TEST_F(FailedTransactionTest, AStatementPreparedBeforeTheFailureIsRefusedToo) {
    auto prepared = conn->prepare("CREATE (:Item {id: 7, v: 0});");
    ASSERT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
    ok("BEGIN TRANSACTION;");
    fails("CREATE (:Item {id: 1, v: 9});");
    auto result = conn->execute(prepared.get());
    ASSERT_FALSE(result->isSuccess());
    EXPECT_NE(result->getErrorMessage().find(TransactionContext::MANUAL_TRANSACTION_FAILED),
        std::string::npos)
        << result->getErrorMessage();
    ok("ROLLBACK;");
    EXPECT_EQ(single("MATCH (n:Item) RETURN count(n);"), 1);
}

TEST_F(FailedTransactionTest, AutoCommitAndOtherConnectionsAreNotAffected) {
    // Hors de tout bloc, une erreur ne laisse aucun état.
    fails("CREATE (:Item {id: 1, v: 9});");
    ok("CREATE (:Item {id: 2, v: 0});");
    // Un bloc en échec sur une connexion ne refuse rien sur une autre.
    ok("BEGIN TRANSACTION;");
    fails("CREATE (:Item {id: 1, v: 9});");
    rag3db::main::Connection other(database.get());
    ok(other, "CREATE (:Item {id: 3, v: 0});");
    ok("ROLLBACK;");
    EXPECT_EQ(single("MATCH (n:Item) RETURN count(n);"), 3);
}

} // namespace
