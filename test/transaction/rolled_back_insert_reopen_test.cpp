// Une insertion défaite, puis la base fermée et rouverte (question d'arbre principal, 13 novembre
// 2026, lib 207701bb2) : un premier processus insère des nœuds et leurs relations dans une
// transaction, la défait (ROLLBACK), et ferme proprement ; le second rouvre la base, ouvre une
// transaction, et fait un SET sur les relations qui joignent ces nœuds. Le moteur refusait :
// « could not serialize access due to concurrent update — row 128 of table ScopeContent was
// deleted by another transaction after this one started », alors qu'il est seul écrivain.
//
// Chaque cas : 128 nœuds Content validés avec une relation chacun vers Scope, puis une transaction
// qui ajoute des nœuds (à partir du décalage 128) et leurs relations, défaite ; fermeture,
// réouverture ; puis dans une transaction le SET sur les relations, avec ou sans écritures avant.
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class RolledBackInsertReopenTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed: the subject is a reopening";
        }
        createDBAndConn();
        run("CREATE NODE TABLE Scope(uuid STRING, PRIMARY KEY(uuid));");
        run("CREATE NODE TABLE Content(uuid STRING, body STRING, PRIMARY KEY(uuid));");
        run("CREATE REL TABLE DERIVED_FROM(FROM Content TO Scope, hash STRING);");
        run("UNWIND range(0, 9) AS i CREATE (:Scope {uuid: 's' + CAST(i AS STRING)});");
        run("UNWIND range(0, 127) AS i MATCH (s:Scope {uuid: 's' + CAST(i % 10 AS STRING)}) "
            "CREATE (c:Content {uuid: 'c' + CAST(i AS STRING), body: 'b'})-[:DERIVED_FROM "
            "{hash: 'h'}]->(s);");
    }
    void run(const std::string& q) const {
        auto result = conn->query(q);
        ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
    }
    std::string row(const std::string& q) const {
        auto result = conn->query(q);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        return result->hasNext() ? result->getNext()->toString() : "AUCUNE LIGNE";
    }
    // Des nœuds Content (décalages 128 et suivants) et leurs relations, dans une transaction
    // défaite.
    void insertThenRollBack(int from, int count) {
        run("BEGIN TRANSACTION;");
        run("UNWIND range(" + std::to_string(from) + ", " + std::to_string(from + count - 1) +
            ") AS i MATCH (s:Scope {uuid: 's' + CAST(i % 10 AS STRING)}) CREATE (c:Content "
            "{uuid: 'c' + CAST(i AS STRING), body: 'b'})-[:DERIVED_FROM {hash: 'h'}]->(s);");
        run("ROLLBACK;");
    }
    void reopen() {
        conn.reset();
        database.reset();
        createDBAndConn();
    }
    // Le SET de la question, sur toutes les relations qui joignent les Scope donnés.
    std::string setOverScopes() const {
        return row("UNWIND ['s0', 's1', 's2', 's3', 's4', 's5', 's6', 's7', 's8', 's9'] AS u "
                   "MATCH (d:Content)-[r:DERIVED_FROM]->(s:Scope {uuid: u}) SET r.hash = '' "
                   "RETURN count(r);");
    }
};

TEST_F(RolledBackInsertReopenTest, ASetAfterReopeningSeesOnlyCommittedRels) {
    insertThenRollBack(128, 40);
    reopen();
    run("BEGIN TRANSACTION;");
    EXPECT_EQ(setOverScopes(), "128\n");
    run("COMMIT;");
    EXPECT_EQ(row("MATCH (c:Content) RETURN count(c);"), "128\n");
    EXPECT_EQ(row("MATCH (:Content)-[r:DERIVED_FROM]->(:Scope) RETURN count(r);"), "128\n");
}

TEST_F(RolledBackInsertReopenTest, ASetAfterNewInsertsInTheSameTransactionSeesItsOwnRels) {
    insertThenRollBack(128, 40);
    reopen();
    run("BEGIN TRANSACTION;");
    // Des écritures avant, comme les MERGE de la question : de nouveaux nœuds reprennent les
    // décalages laissés par l'insertion défaite.
    run("UNWIND range(500, 519) AS i MATCH (s:Scope {uuid: 's' + CAST(i % 10 AS STRING)}) "
        "MERGE (c:Content {uuid: 'c' + CAST(i AS STRING)}) MERGE (c)-[:DERIVED_FROM]->(s);");
    EXPECT_EQ(setOverScopes(), "148\n");
    run("COMMIT;");
    EXPECT_EQ(row("MATCH (:Content)-[r:DERIVED_FROM]->(:Scope) RETURN count(r);"), "148\n");
}

TEST_F(RolledBackInsertReopenTest, ASetWithoutReopeningSeesOnlyCommittedRels) {
    insertThenRollBack(128, 40);
    run("BEGIN TRANSACTION;");
    EXPECT_EQ(setOverScopes(), "128\n");
    run("COMMIT;");
}

TEST_F(RolledBackInsertReopenTest, ARolledBackStatementInsideACommittedTransactionLeavesNoRel) {
    // Le paquet défait au milieu d'une transaction qui valide ensuite (une instruction qui
    // échoue) : ses nœuds et ses relations ne restent pas.
    run("BEGIN TRANSACTION;");
    run("UNWIND range(128, 131) AS i MATCH (s:Scope {uuid: 's' + CAST(i % 10 AS STRING)}) CREATE "
        "(c:Content {uuid: 'c' + CAST(i AS STRING), body: 'b'})-[:DERIVED_FROM {hash: 'h'}]->(s);");
    auto failed = conn->query("UNWIND range(132, 171) AS i MATCH (s:Scope {uuid: 's' + CAST(i % 10 "
                              "AS STRING)}) CREATE (c:Content {uuid: CASE WHEN i = 171 THEN 'c0' "
                              "ELSE 'c' + CAST(i AS STRING) END, body: 'b'})-[:DERIVED_FROM {hash: "
                              "'h'}]->(s);");
    EXPECT_FALSE(failed->isSuccess()) << "the duplicate key c0 must fail the statement";
    auto commit = conn->query("COMMIT;");
    if (!commit->isSuccess()) {
        run("ROLLBACK;");
    }
    reopen();
    run("BEGIN TRANSACTION;");
    const auto expected = commit->isSuccess() ? "132\n" : "128\n";
    EXPECT_EQ(setOverScopes(), expected);
    run("COMMIT;");
}

} // namespace
