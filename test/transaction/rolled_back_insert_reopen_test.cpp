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
#include <filesystem>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "test_helper/test_helper.h"

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
    // Une instruction échouée rend la transaction échouée : son COMMIT est refusé et la ferme.
    auto commit = conn->query("COMMIT;");
    reopen();
    run("BEGIN TRANSACTION;");
    const auto expected = commit->isSuccess() ? "132\n" : "128\n";
    EXPECT_EQ(setOverScopes(), expected);
    run("COMMIT;");
}

// La forme du produit (le repro de rag3weaver s'appelle e2e_tx_vecteurs_apres_rollback) : la table
// défaite porte un index vectoriel, ses vecteurs sont posés par SET depuis NULL dans la
// transaction défaite ; après réouverture, la transaction suivante fait des MERGE puis le SET.
TEST_F(RolledBackInsertReopenTest, AVectorIndexedTableKeepsBothRelsAndRowsAfterARolledBackInsert) {
    const auto extension =
        TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
    if (!std::filesystem::exists(extension)) {
        GTEST_SKIP() << "the vector extension is not built (-DBUILD_EXTENSIONS=vector)";
    }
    run("LOAD EXTENSION '" + extension + "';");
    run("ALTER TABLE Content ADD vec FLOAT[4];");
    run("MATCH (c:Content) SET c.vec = [1.0, 0.5, 0.125, 0.25];");
    run("CALL CREATE_VECTOR_INDEX('Content', 'content_vec', 'vec', metric := 'l2');");
    run("CHECKPOINT;");
    run("BEGIN TRANSACTION;");
    run("UNWIND range(128, 167) AS i MATCH (s:Scope {uuid: 's' + CAST(i % 10 AS STRING)}) CREATE "
        "(c:Content {uuid: 'c' + CAST(i AS STRING), body: 'b'})-[:DERIVED_FROM {hash: 'h'}]->(s);");
    run("UNWIND range(128, 167) AS i MATCH (c:Content {uuid: 'c' + CAST(i AS STRING)}) SET c.vec "
        "= [0.5, 1.0, CAST(i AS FLOAT) / 200.0, 0.75];");
    run("ROLLBACK;");
    reopen();
    run("BEGIN TRANSACTION;");
    run("UNWIND range(500, 519) AS i MATCH (s:Scope {uuid: 's' + CAST(i % 10 AS STRING)}) "
        "MERGE (c:Content {uuid: 'c' + CAST(i AS STRING)}) MERGE (c)-[:DERIVED_FROM]->(s);");
    run("UNWIND range(500, 519) AS i MATCH (c:Content {uuid: 'c' + CAST(i AS STRING)}) SET c.vec "
        "= [0.25, 0.5, CAST(i AS FLOAT) / 1000.0, 1.0];");
    EXPECT_EQ(setOverScopes(), "148\n");
    run("COMMIT;");
    EXPECT_EQ(row("MATCH (c:Content) RETURN count(c);"), "148\n");
}

// Les écritures de la transaction suivante par COPY, comme une ingestion par paquet : un COPY écrit
// ses nœuds dans les groupes de nœuds, hors du stockage local de la transaction.
TEST_F(RolledBackInsertReopenTest, ASetOnRelsCopiedInTheSameTransactionAfterARolledBackInsert) {
    insertThenRollBack(128, 40);
    reopen();
    run("BEGIN TRANSACTION;");
    run("COPY Content FROM (UNWIND range(600, 619) AS i RETURN 'c' + CAST(i AS STRING) AS uuid, "
        "'b' "
        "AS body);");
    run("COPY DERIVED_FROM FROM (UNWIND range(600, 619) AS i RETURN 'c' + CAST(i AS STRING) AS f, "
        "'s' + CAST(i % 10 AS STRING) AS t, 'h' AS hash);");
    EXPECT_EQ(setOverScopes(), "148\n");
    run("COMMIT;");
    EXPECT_EQ(row("MATCH (:Content)-[r:DERIVED_FROM]->(:Scope) RETURN count(r);"), "148\n");
}

// Le même, sans insertion défaite ni réouverture : la cause est-elle le COPY seul ?
TEST_F(RolledBackInsertReopenTest, ASetOnRelsCopiedInTheSameTransaction) {
    run("BEGIN TRANSACTION;");
    run("COPY Content FROM (UNWIND range(600, 619) AS i RETURN 'c' + CAST(i AS STRING) AS uuid, "
        "'b' "
        "AS body);");
    run("COPY DERIVED_FROM FROM (UNWIND range(600, 619) AS i RETURN 'c' + CAST(i AS STRING) AS f, "
        "'s' + CAST(i % 10 AS STRING) AS t, 'h' AS hash);");
    EXPECT_EQ(setOverScopes(), "148\n");
    run("COMMIT;");
}

// La forme exacte du paquet défait de rag3weaver (trace d'arbre principal, 13 novembre) : dans la
// transaction défaite, des Scope neufs par COPY, des Content neufs par MERGE, et leurs relations
// par MERGE avec SET, puis un SET sur ces relations ; ROLLBACK, fermeture. Après réouverture, la
// transaction suivante refait les mêmes Scope par MERGE — ils reprennent les places des Scope
// défaits — puis le SET sur les relations qui les joignent : il n'y en a aucune.
TEST_F(RolledBackInsertReopenTest, ScopesCopiedInARolledBackTransactionLeaveNoRelAtTheirPlaces) {
    run("BEGIN TRANSACTION;");
    run("COPY Scope FROM (UNWIND range(10, 41) AS i RETURN 'n' + CAST(i AS STRING) AS uuid);");
    run("UNWIND range(128, 159) AS i MERGE (c:Content {uuid: 'c' + CAST(i AS STRING)}) SET c.body "
        "= 'b';");
    run("UNWIND range(128, 159) AS i MATCH (a:Content {uuid: 'c' + CAST(i AS STRING)}) MATCH "
        "(b:Scope {uuid: 'n' + CAST(i - 118 AS STRING)}) MERGE (a)-[r:DERIVED_FROM]->(b) SET "
        "r.hash = 'h';");
    run("UNWIND range(10, 41) AS i MATCH (d:Content)-[r:DERIVED_FROM]->(s:Scope {uuid: 'n' + "
        "CAST(i AS STRING)}) SET r.hash = '';");
    run("ROLLBACK;");
    reopen();
    run("BEGIN TRANSACTION;");
    run("UNWIND range(10, 41) AS i MERGE (s:Scope {uuid: 'n' + CAST(i AS STRING)});");
    EXPECT_EQ(row("UNWIND range(10, 41) AS i MATCH (d:Content)-[r:DERIVED_FROM]->(s:Scope {uuid: "
                  "'n' + CAST(i AS STRING)}) SET r.hash = '' RETURN count(r);"),
        "0\n");
    run("COMMIT;");
    EXPECT_EQ(row("MATCH (:Content)-[r:DERIVED_FROM]->(:Scope) RETURN count(r);"), "128\n");
}

// Ce que la base de rag3weaver montre (sonde du 13 novembre sur la base gardée par arbre
// principal) : 128 relations du paquet défait survivent sur disque. Dans ce paquet, chaque MERGE de
// relations est suivi d'un COPY dans une autre table de nœuds, dans la même transaction.
TEST_F(RolledBackInsertReopenTest, ACopyAfterRelInsertsInARolledBackTransactionLeavesNoRel) {
    run("CREATE NODE TABLE Symbol(uuid STRING, PRIMARY KEY(uuid));");
    run("BEGIN TRANSACTION;");
    run("COPY Scope FROM (UNWIND range(10, 41) AS i RETURN 'n' + CAST(i AS STRING) AS uuid);");
    run("UNWIND range(128, 159) AS i MERGE (c:Content {uuid: 'c' + CAST(i AS STRING)}) SET c.body "
        "= 'b';");
    run("UNWIND range(128, 159) AS i MATCH (a:Content {uuid: 'c' + CAST(i AS STRING)}) MATCH "
        "(b:Scope {uuid: 'n' + CAST(i - 118 AS STRING)}) MERGE (a)-[r:DERIVED_FROM]->(b) SET "
        "r.hash = 'h';");
    run("COPY Symbol FROM (UNWIND range(0, 31) AS i RETURN 'y' + CAST(i AS STRING) AS uuid);");
    run("ROLLBACK;");
    EXPECT_EQ(row("MATCH ()-[r:DERIVED_FROM]->() RETURN count(r);"), "128\n") << "before reopening";
    reopen();
    EXPECT_EQ(row("MATCH ()-[r:DERIVED_FROM]->() RETURN count(r);"), "128\n") << "after reopening";
    run("BEGIN TRANSACTION;");
    run("UNWIND range(10, 41) AS i MERGE (s:Scope {uuid: 'n' + CAST(i AS STRING)});");
    EXPECT_EQ(row("MATCH (d)-[r:DERIVED_FROM]->(s:Scope) WHERE s.uuid STARTS WITH 'n' RETURN "
                  "count(r);"),
        "0\n")
        << "rels of the rolled-back transaction came back at the places of the new Scope";
    EXPECT_EQ(row("UNWIND range(10, 41) AS i MATCH (d:Content)-[r:DERIVED_FROM]->(s:Scope {uuid: "
                  "'n' + CAST(i AS STRING)}) SET r.hash = '' RETURN count(r);"),
        "0\n");
    run("ROLLBACK;");
}

// La forme de la base de rag3weaver (sondes du 13 novembre) : après A seul, aucune relation
// orpheline ; elles naissent quand B refait par MERGE les mêmes nœuds que la transaction défaite
// de A. Si une clé défaite survit dans l'index de clé primaire, le MERGE de B la trouve, prend la
// ligne défaite (invisible) pour existante, et pose dessus des relations validées.
TEST_F(RolledBackInsertReopenTest, MergingTheKeysOfARolledBackInsertCreatesTheRows) {
    run("BEGIN TRANSACTION;");
    run("UNWIND range(128, 159) AS i MERGE (c:Content {uuid: 'c' + CAST(i AS STRING)}) SET c.body "
        "= 'a';");
    run("UNWIND range(128, 159) AS i MATCH (a:Content {uuid: 'c' + CAST(i AS STRING)}) MATCH "
        "(b:Scope {uuid: 's' + CAST(i % 10 AS STRING)}) MERGE (a)-[r:DERIVED_FROM]->(b) SET "
        "r.hash = 'h';");
    run("ROLLBACK;");
    reopen();
    run("BEGIN TRANSACTION;");
    run("UNWIND range(128, 159) AS i MERGE (c:Content {uuid: 'c' + CAST(i AS STRING)}) SET c.body "
        "= 'b';");
    run("UNWIND range(128, 159) AS i MATCH (a:Content {uuid: 'c' + CAST(i AS STRING)}) MATCH "
        "(b:Scope {uuid: 's' + CAST(i % 10 AS STRING)}) MERGE (a)-[r:DERIVED_FROM]->(b) SET "
        "r.hash = 'h';");
    run("COMMIT;");
    EXPECT_EQ(row("MATCH (c:Content) RETURN count(c);"), "160\n")
        << "the MERGE found the rolled-back rows instead of creating them";
    EXPECT_EQ(row("MATCH (c:Content)-[r:DERIVED_FROM]->(:Scope) RETURN count(r);"), "160\n");
    EXPECT_EQ(row("MATCH ()-[r:DERIVED_FROM]->(:Scope) RETURN count(r);"), "160\n");
    run("BEGIN TRANSACTION;");
    EXPECT_EQ(setOverScopes(), "160\n");
    run("COMMIT;");
}

// Le paquet défait de rag3weaver modifie ses nœuds après leur MERGE, dans une instruction à part
// (SET n._chunked_hash) : la ligne insérée porte alors une mise à jour de la même transaction.
TEST_F(RolledBackInsertReopenTest, KeysOfRowsUpdatedAfterTheirInsertLeaveWithTheRollback) {
    run("BEGIN TRANSACTION;");
    run("UNWIND range(128, 223) AS i MERGE (c:Content {uuid: 'c' + CAST(i AS STRING)}) SET c.body "
        "= 'a';");
    run("UNWIND range(128, 223) AS i MATCH (a:Content {uuid: 'c' + CAST(i AS STRING)}) MATCH "
        "(b:Scope {uuid: 's' + CAST(i % 10 AS STRING)}) MERGE (a)-[r:DERIVED_FROM]->(b) SET "
        "r.hash = 'h';");
    run("UNWIND range(128, 223) AS i MATCH (c:Content {uuid: 'c' + CAST(i AS STRING)}) SET c.body "
        "= 'modifié';");
    run("ROLLBACK;");
    reopen();
    run("BEGIN TRANSACTION;");
    run("UNWIND range(128, 255) AS i MERGE (c:Content {uuid: 'c' + CAST(i AS STRING)}) SET c.body "
        "= 'b';");
    run("UNWIND range(128, 255) AS i MATCH (a:Content {uuid: 'c' + CAST(i AS STRING)}) MATCH "
        "(b:Scope {uuid: 's' + CAST(i % 10 AS STRING)}) MERGE (a)-[r:DERIVED_FROM]->(b) SET "
        "r.hash = 'h';");
    run("COMMIT;");
    EXPECT_EQ(row("MATCH (c:Content) RETURN count(c);"), "256\n")
        << "the MERGE found keys of rolled-back rows instead of creating them";
    EXPECT_EQ(row("MATCH ()-[r:DERIVED_FROM]->(:Scope) RETURN count(r);"), "256\n");
    run("BEGIN TRANSACTION;");
    EXPECT_EQ(setOverScopes(), "256\n");
    run("COMMIT;");
}

} // namespace
