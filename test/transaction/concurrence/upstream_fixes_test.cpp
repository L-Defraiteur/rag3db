// Marche « correctifs de l'amont à reprendre » : les défauts hérités de Kuzu que Ladybug
// (tag ladybug-main-2026-08-31) ou Vela (tag vela-master-2026-09-03) ont corrigés et que
// nous portons encore (revue du 4 octobre). Un seul écrivain, en service. Chaque témoin est
// écrit ici d'après le symptôme, sans reprendre le code ni les tests des amonts ; le commit
// de l'amont est nommé pour que la session cœur C++ lise ce qu'il fait.
//
// Les cas qui font planter le processus tournent dans un fils (inChild) : un plantage est
// un rouge nommé, pas une passe perdue.

#ifndef __SINGLE_THREADED__

#include <sys/wait.h>
#include <unistd.h>

#include <chrono>
#include <fstream>
#include <functional>
#include <iostream>
#include <thread>

#include "bench_harness.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "processor/result/flat_tuple.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

class UpstreamFixes : public EmptyDBTest {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        mustRun("CALL auto_checkpoint=false;");
    }

    void mustRun(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << "[check: query] " << query << "\n"
                                         << result->getErrorMessage();
    }

    int64_t queryInt(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << "[check: query] " << query << "\n"
                                         << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    uint64_t numRows(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << "[check: query] " << query << "\n"
                                         << result->getErrorMessage();
        return result->isSuccess() ? result->getNumTuples() : UINT64_MAX;
    }

    void reopen() {
        conn.reset();
        database.reset();
        createDBAndConn();
    }

    // Ferme la base du test et fait tourner work dans un fils, sur la même base. Rend ""
    // si le fils sort par 0 ; sinon ce qui l'a arrêté (un signal, ou le code que work rend
    // pour dire que le résultat est faux). La base du test reste fermée : après un fils qui
    // a planté, la fermeture d'une base rouverte ici ferait un point de reprise qui peut
    // planter aussi, et emporter la passe. Les vérifications suivantes passent par un fils.
    std::string inChild(const std::function<int(rag3db::main::Connection&)>& work) {
        conn.reset();
        database.reset();
        const auto pid = fork();
        if (pid == 0) {
            concurrency::disableCoreDumps();
            int code = 0;
            try {
                rag3db::main::Database childDatabase(databasePath, *systemConfig);
                rag3db::main::Connection childConnection(&childDatabase);
                childConnection.query("CALL auto_checkpoint=false;");
                // Sortie base ouverte : la fermeture ferait son propre point de reprise, et
                // un plantage de fermeture se lirait comme un défaut de ce que work éprouve.
                _exit(work(childConnection));
            } catch (const std::exception& e) {
                std::cerr << "  child failed: " << e.what() << "\n";
                code = 3;
            }
            _exit(code);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        if (WIFSIGNALED(status)) {
            return "killed by signal " + std::to_string(WTERMSIG(status));
        }
        return WEXITSTATUS(status) == 0 ? "" : "exit code " + std::to_string(WEXITSTATUS(status));
    }
};

// Dans un fils : la requête doit réussir ; 4 sinon.
int runOrFail(rag3db::main::Connection& connection, const std::string& query) {
    auto result = connection.query(query);
    if (!result->isSuccess()) {
        std::cerr << "  " << query << ": " << result->getErrorMessage() << "\n";
        return 4;
    }
    return 0;
}

// --- Ce qui perd ou corrompt des données -------------------------------------------------

// Ladybug 0be6fa597, Vela e5e700e73 (défaut d'origine Kuzu eaf97b755). Au point de reprise
// d'un sens d'une table de relations, seules les régions modifiées (1 024 positions du nœud
// lié) sont comptées ; si elles finissent vides, tout le groupe de 131 072 nœuds est libéré, et
// les relations des régions intactes disparaissent dans ce sens.
TEST_F(UpstreamFixes, RelationsOfAnUntouchedRegionSurviveACheckpoint) {
    mustRun("CREATE NODE TABLE person(id INT64 PRIMARY KEY);");
    mustRun("CREATE REL TABLE knows(FROM person TO person, MANY_MANY);");
    mustRun("UNWIND range(0, 1024) AS id CREATE (:person {id: id});");
    mustRun("MATCH (a:person), (b:person) WHERE a.id IN [0, 1024] AND b.id = 1 "
            "CREATE (a)-[:knows]->(b);");
    mustRun("CHECKPOINT;");
    mustRun("MATCH (a:person)-[r:knows]->(:person) WHERE a.id = 0 DELETE r;");
    mustRun("CHECKPOINT;");
    for (const auto* moment : {"after the checkpoint", "after reopening"}) {
        EXPECT_EQ(queryInt("MATCH (a:person {id: 1024})-[r:knows]->() RETURN count(r);"), 1)
            << "[check: relations-survive-checkpoint] " << moment
            << ": the relation of node 1024 is lost in the forward direction";
        EXPECT_EQ(queryInt("MATCH ()<-[r:knows]-(a:person) RETURN count(r);"),
            queryInt("MATCH (a:person)-[r:knows]->() RETURN count(r);"))
            << "[check: directions-agree] " << moment;
        reopen();
    }
}

// Vela e5e700e73. Après ALTER TABLE … DROP d'une colonne qui n'est pas la dernière, le
// point de reprise d'un groupe qui a des données sur disque indexe ses tableaux de colonnes
// par l'identifiant de colonne au lieu de la position : SIGSEGV. Après réouverture, tout
// point de reprise suivant plante encore.
TEST_F(UpstreamFixes, CheckpointAfterDroppingANodeColumn) {
    mustRun("CREATE NODE TABLE t(id INT64 PRIMARY KEY, a INT64, b STRING, c INT64);");
    mustRun("CREATE (:t {id: 0, a: 1, b: 'x', c: 10});");
    mustRun("CHECKPOINT;");
    const auto first = inChild([](rag3db::main::Connection& connection) {
        if (const auto failed = runOrFail(connection, "ALTER TABLE t DROP b;") +
                                runOrFail(connection, "CREATE (:t {id: 1, a: 2, c: 20});") +
                                runOrFail(connection, "MATCH (n:t {id: 0}) SET n.c = 99;")) {
            return failed;
        }
        return runOrFail(connection, "CHECKPOINT;");
    });
    EXPECT_EQ(first, "") << "[check: checkpoint-survives] the checkpoint after DROP: " << first;
    const auto second = inChild([](rag3db::main::Connection& connection) {
        return runOrFail(connection, "CHECKPOINT;");
    });
    EXPECT_EQ(second, "") << "[check: checkpoint-after-reopen] " << second;
    const auto values = inChild([](rag3db::main::Connection& connection) {
        auto result = connection.query("MATCH (n:t) RETURN sum(n.a * 1000 + n.c);");
        return result->isSuccess() &&
                       result->getNext()->getValue(0)->getValue<int64_t>() == 1099 + 2020 ?
                   0 :
                   5;
    });
    EXPECT_EQ(values, "") << "[check: values-after-drop] " << values;
}

// Vela e5e700e73, même défaut sur une table de relations (csrState.columns[columnID]).
TEST_F(UpstreamFixes, CheckpointAfterDroppingARelationColumn) {
    mustRun("CREATE NODE TABLE p(id INT64 PRIMARY KEY);");
    mustRun("CREATE REL TABLE r(FROM p TO p, a INT64, b INT64);");
    mustRun("CREATE (:p {id: 0}), (:p {id: 1});");
    mustRun("MATCH (x:p {id: 0}), (y:p {id: 1}) CREATE (x)-[:r {a: 1, b: 2}]->(y);");
    mustRun("CHECKPOINT;");
    const auto first = inChild([](rag3db::main::Connection& connection) {
        // Une insertion et une mise à jour de la relation déjà sur disque : les deux chemins du
        // point de reprise d'une table de relations.
        if (const auto failed = runOrFail(connection, "ALTER TABLE r DROP a;") +
                                runOrFail(connection, "MATCH (x:p {id: 1}), (y:p {id: 0}) "
                                                      "CREATE (x)-[:r {b: 3}]->(y);") +
                                runOrFail(connection, "MATCH ()-[e:r]->() WHERE e.b = 2 "
                                                      "SET e.b = 7;")) {
            return failed;
        }
        return runOrFail(connection, "CHECKPOINT;");
    });
    EXPECT_EQ(first, "") << "[check: checkpoint-survives] the checkpoint after DROP: " << first;
    const auto second = inChild([](rag3db::main::Connection& connection) {
        return runOrFail(connection, "CHECKPOINT;");
    });
    EXPECT_EQ(second, "") << "[check: checkpoint-after-reopen] " << second;
    const auto values = inChild([](rag3db::main::Connection& connection) {
        auto result = connection.query("MATCH ()-[e:r]->() RETURN sum(e.b);");
        return result->isSuccess() && result->getNext()->getValue(0)->getValue<int64_t>() == 10 ?
                   0 :
                   5;
    });
    EXPECT_EQ(values, "") << "[check: values-after-drop] " << values;
}

// Garde-fou vert : la lecture partielle d'une colonne de chaînes de relations au point de
// reprise d'une seule région (Ladybug 254a7444d, chaînes réécrites aux mauvaises lignes).
// Non reproduit chez nous avec cette forme (4 octobre) : il reste comme témoin.
TEST_F(UpstreamFixes, RelationStringsKeepTheirRowsAcrossARegionCheckpoint) {
    mustRun("CREATE NODE TABLE N(id INT64 PRIMARY KEY);");
    mustRun("CREATE REL TABLE R(FROM N TO N, s STRING);");
    mustRun("UNWIND range(0, 2999) AS i CREATE (:N {id: i});");
    mustRun("MATCH (a:N), (b:N {id: 0}) WHERE a.id >= 1 "
            "CREATE (a)-[:R {s: concat('v', CAST(a.id % 3 AS STRING))}]->(b);");
    mustRun("CHECKPOINT;");
    mustRun("MATCH (a:N {id: 1500})-[r:R]->() DELETE r;");
    mustRun("CHECKPOINT;");
    reopen();
    EXPECT_EQ(queryInt("MATCH (a:N)-[r:R]->() WHERE r.s <> concat('v', CAST(a.id % 3 AS STRING)) "
                       "RETURN count(*);"),
        0)
        << "[check: relation-strings-kept] ";
}

// --- Ce qui fait planter le service --------------------------------------------------------

// Ladybug ba5f38815. Une relation créée puis supprimée depuis le dernier point de reprise :
// le point de reprise suivant sort sans finaliser l'en-tête CSR, et la lecture qui suit
// tombe sur une sentinelle (SIGSEGV).
TEST_F(UpstreamFixes, ReadAfterCheckpointOfACreatedThenDeletedRelation) {
    mustRun("CREATE NODE TABLE N(id INT64 PRIMARY KEY);");
    mustRun("CREATE REL TABLE R(FROM N TO N, k INT64);");
    mustRun("CREATE (:N {id: 1}), (:N {id: 2});");
    mustRun("MATCH (a:N {id: 1}), (b:N {id: 2}) CREATE (a)-[:R {k: 1}]->(b);");
    mustRun("CHECKPOINT;");
    const auto outcome = inChild([](rag3db::main::Connection& connection) {
        if (const auto failed =
                runOrFail(connection, "MATCH (a:N {id: 1}), (b:N {id: 2}) "
                                      "CREATE (a)-[:R {k: 2}]->(b);") +
                runOrFail(connection, "MATCH ()-[r:R]->() WHERE r.k = 2 DELETE r;") +
                runOrFail(connection, "CHECKPOINT;")) {
            return failed;
        }
        auto result = connection.query("MATCH (:N {id: 1})-[r:R]->() RETURN r.k;");
        if (!result->isSuccess() || result->getNumTuples() != 1) {
            return 5;
        }
        return runOrFail(connection, "CHECKPOINT;");
    });
    EXPECT_EQ(outcome, "") << "[check: read-after-checkpoint-survives] " << outcome;
}

// Ladybug a0063158d. MERGE avec deux ON MATCH SET sur la même relation, et la même clé deux
// fois dans le lot : le second SET lit un identifiant non initialisé (SIGSEGV).
TEST_F(UpstreamFixes, MergeWithTwoOnMatchSetsAndARepeatedKey) {
    mustRun("CREATE NODE TABLE A(id STRING PRIMARY KEY);");
    mustRun("CREATE NODE TABLE B(id STRING PRIMARY KEY);");
    mustRun("CREATE REL TABLE R(FROM A TO B, t STRING, u INT64);");
    mustRun("CREATE (:A {id: 'a'}), (:B {id: 'b'});");
    const auto outcome = inChild([](rag3db::main::Connection& connection) {
        if (const auto failed = runOrFail(connection,
                "UNWIND [{s: 'a', t: 'b', v: 'x', n: 1}, {s: 'a', t: 'b', v: 'y', n: 2}] AS row "
                "MATCH (s:A {id: row.s}) MATCH (t:B {id: row.t}) MERGE (s)-[r:R]->(t) "
                "ON CREATE SET r.t = row.v, r.u = row.n ON MATCH SET r.t = row.v, r.u = row.n;")) {
            return failed;
        }
        auto result = connection.query("MATCH ()-[r:R]->() RETURN r.u;");
        return result->isSuccess() && result->getNumTuples() == 1 &&
                       result->getNext()->getValue(0)->getValue<int64_t>() == 2 ?
                   0 :
                   5;
    });
    EXPECT_EQ(outcome, "") << "[check: merge-survives] " << outcome;
}

// Ladybug 26be67f84. Une branche de UNION qui projette deux fois la même propriété.
TEST_F(UpstreamFixes, UnionArmProjectingAPropertyTwice) {
    mustRun("CREATE NODE TABLE Person(id INT64 PRIMARY KEY, age INT64);");
    mustRun("CREATE (:Person {id: 1, age: 30});");
    const auto outcome = inChild([](rag3db::main::Connection& connection) {
        auto result = connection.query("MATCH (a:Person) RETURN 1, 2 UNION ALL "
                                       "MATCH (b:Person) RETURN b.age, b.age;");
        // L'ordre des branches n'est pas garanti : on cherche la ligne de la seconde.
        return result->isSuccess() && result->getNumTuples() == 2 &&
                       result->toString().find("\n30|30\n") != std::string::npos ?
                   0 :
                   5;
    });
    EXPECT_EQ(outcome, "") << "[check: query-survives] " << outcome;
    // La branche qui projette deux fois la même propriété en premier.
    const auto reversed = inChild([](rag3db::main::Connection& connection) {
        auto result = connection.query("MATCH (b:Person) RETURN b.age, b.age UNION ALL "
                                       "MATCH (a:Person) RETURN 1, 2;");
        return result->isSuccess() && result->getNumTuples() == 2 ? 0 : 5;
    });
    EXPECT_EQ(reversed, "") << "[check: query-survives] reversed: " << reversed;
}

// Ladybug 0c19d7816. Un chemin utilisé seulement dans le corps d'un lambda.
TEST_F(UpstreamFixes, PathUsedOnlyInsideALambda) {
    mustRun("CREATE NODE TABLE Person(name STRING PRIMARY KEY);");
    mustRun("CREATE REL TABLE K(FROM Person TO Person);");
    mustRun("CREATE (:Person {name: 'Alice'})-[:K]->(:Person {name: 'Bob'})-[:K]->"
            "(:Person {name: 'Charlie'});");
    const auto outcome = inChild([](rag3db::main::Connection& connection) {
        auto result = connection.query(
            "MATCH p = (a)-[*1..2]-(b)-[*1..2]-(c) WHERE a.name = 'Alice' AND b.name = 'Bob' "
            "AND c.name = 'Charlie' RETURN any(x IN [1] WHERE p IS NOT NULL);");
        return result->isSuccess() && result->getNumTuples() >= 1 ? 0 : 5;
    });
    EXPECT_EQ(outcome, "") << "[check: query-survives] " << outcome;
}

// Ladybug 0f3f03d63 et 57dc0b4ed. Une relation comparée à une variable de lambda, et
// label() sur un élément de nodes(p).
TEST_F(UpstreamFixes, RelationComparedToALambdaVariable) {
    mustRun("CREATE NODE TABLE person(ID INT64 PRIMARY KEY);");
    mustRun("CREATE REL TABLE knows(FROM person TO person);");
    mustRun("UNWIND range(0, 3) AS i CREATE (:person {ID: i});");
    mustRun("MATCH (a:person), (b:person) WHERE a.ID <> b.ID CREATE (a)-[:knows]->(b);");
    const auto outcome = inChild([](rag3db::main::Connection& connection) {
        auto result = connection.query(
            "MATCH (a:person)-[t:knows]->(d:person) WITH a, d, t "
            "MATCH p = (a)-[:knows*1..3]->(d) WHERE length(p) >= 2 AND "
            "ALL(r IN relationships(p) WHERE r <> t) WITH DISTINCT t RETURN count(t);");
        // Graphe complet sur quatre nœuds : chacune des douze relations a un autre chemin.
        return result->isSuccess() && result->getNext()->getValue(0)->getValue<int64_t>() == 12 ?
                   0 :
                   5;
    });
    EXPECT_EQ(outcome, "") << "[check: query-survives] " << outcome;
    const auto label = inChild([](rag3db::main::Connection& connection) {
        auto result = connection.query(
            "MATCH p = (n)-[:knows]->(m:person {ID: 2}) WITH nodes(p) AS ns RETURN label(ns[1]);");
        if (!result->isSuccess() || result->getNumTuples() != 3) {
            return 5;
        }
        while (result->hasNext()) {
            if (result->getNext()->getValue(0)->toString() != "person") {
                return 6;
            }
        }
        return 0;
    });
    EXPECT_EQ(label, "") << "[check: query-survives] label(): " << label;
}

// Ladybug 8281c845e. WITH sans colonne utile, filtré par un paramètre (SIGSEGV).
TEST_F(UpstreamFixes, WithGateFilteredByAParameter) {
    mustRun("CREATE NODE TABLE person(ID INT64 PRIMARY KEY);");
    mustRun("UNWIND range(0, 7) AS i CREATE (:person {ID: i});");
    const auto outcome = inChild([](rag3db::main::Connection& connection) {
        auto prepared =
            connection.prepare("WITH 1 AS gate WHERE $depth >= 2 MATCH (a:person) RETURN a.ID;");
        auto result =
            connection.execute(prepared.get(), std::make_pair(std::string("depth"), int64_t{1}));
        return result->isSuccess() && result->getNumTuples() == 0 ? 0 : 5;
    });
    EXPECT_EQ(outcome, "") << "[check: query-survives] " << outcome;
}

// --- Ce qui bloque le service ----------------------------------------------------------------

// Ladybug 231a0b854. L'UUID nul est stocké comme INT128_MIN, et la compression d'une colonne
// qui le contient lève « cannot negate INT128_MIN » : aucun point de reprise ne passe plus,
// même après réouverture.
TEST_F(UpstreamFixes, CheckpointOfAColumnHoldingTheNilUuid) {
    mustRun("CREATE NODE TABLE T(id INT64 PRIMARY KEY, u UUID);");
    mustRun("CREATE (:T {id: 1, u: UUID('00000000-0000-0000-0000-000000000000')}), "
            "(:T {id: 2, u: UUID('00000000-0000-0000-0000-000000000001')});");
    auto checkpoint = conn->query("CHECKPOINT;");
    EXPECT_TRUE(checkpoint->isSuccess())
        << "[check: checkpoint-succeeds] " << checkpoint->getErrorMessage();
    checkpoint.reset();
    reopen();
    auto again = conn->query("CHECKPOINT;");
    EXPECT_TRUE(again->isSuccess())
        << "[check: checkpoint-after-reopen] " << again->getErrorMessage();
    again.reset();
    reopen();
    auto values = conn->query("MATCH (t:T) RETURN t.u ORDER BY t.id;");
    ASSERT_TRUE(values->isSuccess()) << "[check: query] " << values->getErrorMessage();
    EXPECT_EQ(values->toString(), "t.u\n00000000-0000-0000-0000-000000000000\n"
                                  "00000000-0000-0000-0000-000000000001\n")
        << "[check: uuid-values-kept] ";
}

// Vela 326d40dbd. Un CHECKPOINT garde le verrou que la validation d'un lecteur demande, et
// attend que ce lecteur parte : le lecteur reste bloqué jusqu'au délai (5 s), puis le
// CHECKPOINT échoue. Un lecteur de moins d'une seconde seul en dure plus de cinq.
TEST_F(UpstreamFixes, CheckpointLetsAFinishingReaderGo) {
    mustRun("CREATE NODE TABLE P(id INT64 PRIMARY KEY);");
    mustRun("UNWIND range(1, 20000) AS i CREATE (:P {id: i});");
    const auto readerQuery = "MATCH (a:P), (b:P) WHERE a.id + b.id = 7 RETURN count(*);";
    auto start = std::chrono::steady_clock::now();
    conn->query(readerQuery);
    const auto alone = std::chrono::steady_clock::now() - start;
    rag3db::main::Connection reader(database.get());
    std::chrono::steady_clock::duration withCheckpoint{};
    std::thread readerThread([&] {
        const auto readerStart = std::chrono::steady_clock::now();
        reader.query(readerQuery);
        withCheckpoint = std::chrono::steady_clock::now() - readerStart;
    });
    std::this_thread::sleep_for(alone / 5);
    auto checkpoint = conn->query("CHECKPOINT;");
    readerThread.join();
    const auto ms = [](auto d) {
        return std::chrono::duration_cast<std::chrono::milliseconds>(d).count();
    };
    std::cerr << "  reader alone " << ms(alone) << " ms, during a checkpoint "
              << ms(withCheckpoint) << " ms\n";
    EXPECT_TRUE(checkpoint->isSuccess())
        << "[check: checkpoint-succeeds] " << checkpoint->getErrorMessage();
    EXPECT_LT(withCheckpoint, alone * 3 + std::chrono::seconds(1))
        << "[check: reader-not-held] the reader took " << ms(withCheckpoint) << " ms, "
        << ms(alone) << " ms alone";
}

// --- Ce qui rend une réponse fausse ----------------------------------------------------------

// Ladybug 079aa8458. SKIP sans LIMIT poussé dans DISTINCT (limite = skip + UINT64_MAX qui
// déborde) et dans une extension récursive.
TEST_F(UpstreamFixes, SkipWithoutLimit) {
    mustRun("CREATE NODE TABLE T(id INT64 PRIMARY KEY);");
    mustRun("UNWIND range(0, 4999) AS i CREATE (:T {id: i});");
    EXPECT_EQ(numRows("MATCH (n:T) RETURN DISTINCT n.id SKIP 10;"), 4990u)
        << "[check: skip-keeps-the-rest] DISTINCT";
    mustRun("CREATE NODE TABLE P(ID INT64 PRIMARY KEY);");
    mustRun("CREATE REL TABLE K(FROM P TO P);");
    mustRun("UNWIND range(0, 4) AS i CREATE (:P {ID: i});");
    mustRun("MATCH (a:P), (b:P) WHERE a.ID < b.ID CREATE (a)-[:K]->(b);");
    const auto all = numRows("MATCH (a:P)-[:K*1..2]->(b) RETURN CASE WHEN a.ID >= 0 THEN 1 "
                             "ELSE 0 END;");
    EXPECT_EQ(numRows("MATCH (a:P)-[:K*1..2]->(b) RETURN CASE WHEN a.ID >= 0 THEN 1 ELSE 0 END "
                      "SKIP 5;"),
        all - 5)
        << "[check: skip-keeps-the-rest] recursive extend";
}

// Ladybug 8281c845e. Un prédicat qui ne dépend que de paramètres disparaît du plan.
TEST_F(UpstreamFixes, PredicateOnParametersOnly) {
    mustRun("CREATE NODE TABLE person(ID INT64 PRIMARY KEY);");
    mustRun("UNWIND range(0, 7) AS i CREATE (:person {ID: i});");
    auto prepared = conn->prepare("MATCH (a:person) WHERE $depth >= 2 RETURN a.ID;");
    auto result = conn->execute(prepared.get(), std::make_pair(std::string("depth"), int64_t{1}));
    ASSERT_TRUE(result->isSuccess()) << "[check: query] " << result->getErrorMessage();
    EXPECT_EQ(result->getNumTuples(), 0u) << "[check: parameter-predicate-applied] ";
}

// Ladybug 413079079. L'étiquette explicite d'une variable déjà liée sans étiquette est
// ignorée.
TEST_F(UpstreamFixes, ExplicitLabelOfAVariableBoundEarlier) {
    mustRun("CREATE NODE TABLE L1(id INT64 PRIMARY KEY);");
    mustRun("CREATE NODE TABLE L2(id INT64 PRIMARY KEY);");
    mustRun("CREATE NODE TABLE L3(id INT64 PRIMARY KEY);");
    mustRun("CREATE REL TABLE T5(FROM L1 TO L3);");
    mustRun("CREATE (:L1 {id: 1})-[:T5]->(:L3 {id: 2});");
    EXPECT_EQ(queryInt("MATCH (n1), (n0:L3)<-[:T5]-(n1:L2) RETURN count(*);"), 0)
        << "[check: explicit-label-applied] ";
}

// Ladybug e36c93e59. Un nœud déjà lié, répété seul dans un OPTIONAL MATCH, est rebalayé.
TEST_F(UpstreamFixes, OptionalMatchRepeatingABoundNode) {
    mustRun("CREATE NODE TABLE L1(id INT64 PRIMARY KEY, k11 INT64);");
    mustRun("CREATE NODE TABLE X(id INT64 PRIMARY KEY);");
    mustRun("CREATE REL TABLE T1(FROM X TO L1);");
    mustRun("CREATE (:X {id: 1})-[:T1]->(:L1 {id: 1, k11: 1});");
    EXPECT_EQ(queryInt("MATCH (n0:L1), (n3) OPTIONAL MATCH (n6)-[:T1]->(n0), (n3) "
                       "WHERE n0.k11 = 1 RETURN count(*);"),
        2)
        << "[check: optional-match-cardinality] ";
}

// Ladybug c5a3c7385. MERGE de la même clé deux fois dans un lot : la seconde ligne rend une
// valeur par défaut réévaluée, ou l'identifiant d'un autre nœud.
TEST_F(UpstreamFixes, MergeOfARepeatedKeyReturnsTheStoredNode) {
    mustRun("CREATE NODE TABLE A(id STRING DEFAULT gen_random_uuid(), stuff INT64, "
            "PRIMARY KEY(id));");
    auto merged = conn->query("UNWIND [1, 1] AS i MERGE (a:A {stuff: i}) RETURN DISTINCT a.id;");
    ASSERT_TRUE(merged->isSuccess()) << "[check: query] " << merged->getErrorMessage();
    EXPECT_EQ(merged->getNumTuples(), 1u)
        << "[check: merge-returns-stored-node] the repeated key returns another default";
    mustRun("CREATE NODE TABLE B(id SERIAL, stuff INT64, PRIMARY KEY(id));");
    // Les deux lignes de stuff = 1 doivent rendre le même nœud, celui qui est stocké.
    EXPECT_EQ(queryInt("UNWIND [1, 2, 1] AS i MERGE (b:B {stuff: i}) WITH i, "
                       "collect(DISTINCT b.id) AS ids WHERE size(ids) > 1 RETURN count(*);"),
        0)
        << "[check: merge-returns-stored-node] the repeated key returns another node";
    // Un SET qui suit un MERGE sur une propriété qui n'est pas la clé.
    mustRun("CREATE NODE TABLE C(id SERIAL, stuff INT64, x INT64, PRIMARY KEY(id));");
    mustRun("UNWIND [1, 2, 1] AS i MERGE (c:C {stuff: i}) SET c.x = i * 10;");
    EXPECT_EQ(queryInt("MATCH (c:C {stuff: 2}) RETURN c.x;"), 20)
        << "[check: merge-returns-stored-node] the SET after the repeated key";
    EXPECT_EQ(queryInt("MATCH (c:C {stuff: 1}) RETURN c.x;"), 10)
        << "[check: merge-returns-stored-node] the SET after the repeated key";
}

// La forme de batch_link de rag3weaver (dialect.rs, unwind_par_cle puis MERGE de la relation
// et SET) quand un lot porte deux fois la même relation, créée par le lot lui-même, avec une
// autre relation créée entre les deux : le SET de la seconde occurrence est perdu, sans erreur.
// Adjacents, ou sur une relation qui existait déjà, les doublons sont justes ; aucune autre
// relation n'est touchée (sonde du 4 octobre). Même famille que Ladybug c5a3c7385 : un MERGE qui
// retrouve un motif créé plus tôt dans le lot ne repose pas l'identifiant de ce qu'il a trouvé.
TEST_F(UpstreamFixes, MergeOfARelationRepeatedInTheBatchKeepsTheLastSet) {
    mustRun("CREATE NODE TABLE A(_uuid STRING PRIMARY KEY);");
    mustRun("CREATE NODE TABLE B(_uuid STRING PRIMARY KEY);");
    mustRun("CREATE REL TABLE R(FROM A TO B, t STRING, u INT64);");
    mustRun("CREATE (:A {_uuid: 'a'}), (:B {_uuid: 'b'}), (:B {_uuid: 'c'});");
    mustRun("UNWIND [{from_uuid: 'a', to_uuid: 'b', t: 'x', u: 1}, "
            "{from_uuid: 'a', to_uuid: 'c', t: 'w', u: 5}, "
            "{from_uuid: 'a', to_uuid: 'b', t: 'y', u: 2}] AS item "
            "WITH item, item.from_uuid AS __cle_0 MATCH (a:A {_uuid: __cle_0}) "
            "WITH a, item, item.to_uuid AS __cle_1 MATCH (b:B {_uuid: __cle_1}) "
            "MERGE (a)-[r:R]->(b) SET r.t = item.t, r.u = item.u;");
    EXPECT_EQ(queryInt("MATCH (:A)-[r:R]->(:B {_uuid: 'b'}) RETURN r.u;"), 2)
        << "[check: merge-set-applied] the second occurrence of a->b lost its SET";
    EXPECT_EQ(queryInt("MATCH (:A)-[r:R]->(:B {_uuid: 'c'}) RETURN r.u;"), 5)
        << "[check: merge-set-applied] a->c";
}

// Ladybug a8814a143. Le balayage de plusieurs tables de relations partage un état : les
// relations locales de R1 sont relues sous R2.
TEST_F(UpstreamFixes, ScanOfSeveralRelationTablesInATransaction) {
    mustRun("CREATE NODE TABLE P(id INT64 PRIMARY KEY);");
    mustRun("CREATE REL TABLE R1(FROM P TO P);");
    mustRun("CREATE REL TABLE R2(FROM P TO P);");
    mustRun("CREATE (:P {id: 1}), (:P {id: 2});");
    mustRun("BEGIN TRANSACTION;");
    mustRun("MATCH (a:P {id: 1}), (b:P {id: 2}) CREATE (a)-[:R1]->(b);");
    EXPECT_EQ(queryInt("MATCH (a:P)-[r:R1|R2]->(b:P) RETURN count(*);"), 1)
        << "[check: transaction-reads-its-relations] ";
    mustRun("COMMIT;");
}

// Défaut d'origine, même code chez Vela et Ladybug (relevé par la session cœur C++, ticket
// 2026-10-04-copy-apres-des-insertions-dans-la-meme-transaction). Les lignes insérées dans
// une transaction portent des décalages provisoires qui commencent au nombre de lignes de
// la table ; un COPY dans la même table et la même transaction écrit ses lignes aux mêmes
// décalages, et la transaction prend les unes pour les autres : la clé 5 rend la ligne
// 1005, le SET et la relation tombent sur les lignes locales ; avec une seule ligne locale,
// la recherche par clé plante.
//
// Le COPY était refusé par un nom depuis 0f4a54b2c ; depuis f1d8c7190, les lignes locales sont
// versées dans la table avant le COPY, et le refus est retiré. Le témoin exige désormais le
// COPY accepté, et chaque lecture et chaque écriture sur la bonne ligne, avant comme après
// la validation.
class CopyAfterLocalInserts : public UpstreamFixes {
public:
    // Codes du fils : 0 invariant tenu, 5 clé rendue fausse, 6 SET mal placé, 7 relation
    // mal reliée, 8 état validé faux, 9 COPY refusé, 4 requête échouée.
    std::string run(int localRows) {
        const auto csv = databasePath + ".copy-after-local.csv";
        {
            std::ofstream out(csv);
            for (int i = 0; i < 100; i++) {
                out << i << ",copie " << i << "\n";
            }
        }
        mustRun("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
        mustRun("CREATE REL TABLE L(FROM Doc TO Doc);");
        return inChild([&](rag3db::main::Connection& connection) {
            if (auto code = runOrFail(connection, "BEGIN TRANSACTION;")) {
                return code;
            }
            if (auto code = runOrFail(connection,
                    "UNWIND range(1000, " + std::to_string(1000 + localRows - 1) +
                        ") AS i CREATE (:Doc {id: i, name: 'locale ' + CAST(i AS STRING)});")) {
                return code;
            }
            auto copy = connection.query("COPY Doc FROM '" + csv + "' (header=false);");
            if (!copy->isSuccess()) {
                std::cerr << "  copy: " << copy->getErrorMessage() << "\n";
                connection.query("ROLLBACK;");
                return 9;
            }
            auto key = connection.query("MATCH (n:Doc {id: 5}) RETURN n.id;");
            if (!key->isSuccess() || !key->hasNext() ||
                key->getNext()->getValue(0)->getValue<int64_t>() != 5) {
                std::cerr << "  key 5: "
                          << (key->isSuccess() ? key->toString() : key->getErrorMessage())
                          << "\n";
                return 5;
            }
            if (auto code = runOrFail(connection, "MATCH (n:Doc {id: 5}) SET n.name = 'X';")) {
                return code;
            }
            if (auto code = runOrFail(connection,
                    "MATCH (a:Doc {id: 7}), (b:Doc {id: 8}) CREATE (a)-[:L]->(b);")) {
                return code;
            }
            if (auto code = runOrFail(connection, "COMMIT;")) {
                return code;
            }
            const auto count = [&](const std::string& query) -> int64_t {
                auto result = connection.query(query);
                return result->isSuccess() && result->hasNext() ?
                           result->getNext()->getValue(0)->getValue<int64_t>() :
                           -1;
            };
            if (count("MATCH (n:Doc) WHERE n.name = 'X' RETURN count(*);") != 1 ||
                count("MATCH (n:Doc {id: 5}) WHERE n.name = 'X' RETURN count(*);") != 1) {
                return 6;
            }
            if (count("MATCH (a:Doc)-[:L]->(b:Doc) WHERE a.id = 7 AND b.id = 8 RETURN "
                      "count(*);") != 1 ||
                count("MATCH ()-[r:L]->() RETURN count(r);") != 1) {
                return 7;
            }
            if (count("MATCH (n:Doc) RETURN count(n);") != 100 + localRows ||
                count("MATCH (n:Doc) WHERE n.id >= 1000 AND n.name = 'locale ' + CAST(n.id "
                      "AS STRING) RETURN count(*);") != localRows) {
                return 8;
            }
            return 0;
        });
    }
};

std::string copyAfterLocalInsertsCheck(const std::string& outcome) {
    if (outcome == "exit code 5") {
        return "[check: copy-after-local-inserts-key] the key 5 does not give the row 5";
    }
    if (outcome == "exit code 6") {
        return "[check: copy-after-local-inserts-set] the SET of the row 5 landed elsewhere";
    }
    if (outcome == "exit code 7") {
        return "[check: copy-after-local-inserts-relation] the relation does not join 7 and 8";
    }
    if (outcome == "exit code 8") {
        return "[check: copy-after-local-inserts-committed] the committed rows are wrong";
    }
    if (outcome == "exit code 9") {
        return "[check: copy-after-local-inserts-accepted] the COPY was refused";
    }
    return "[check: copy-after-local-inserts-runs] " + outcome;
}

TEST_F(CopyAfterLocalInserts, TwentyLocalRowsThenACopyInTheSameTable) {
    const auto outcome = run(20);
    EXPECT_EQ(outcome, "") << copyAfterLocalInsertsCheck(outcome);
}

TEST_F(CopyAfterLocalInserts, OneLocalRowThenACopyInTheSameTable) {
    const auto outcome = run(1);
    EXPECT_EQ(outcome, "") << copyAfterLocalInsertsCheck(outcome);
}

// Ladybug 2fc419036. Un champ CSV vide entre guillemets devient NULL.
TEST_F(UpstreamFixes, QuotedEmptyCsvFieldIsNotNull) {
    const auto csv = databasePath + ".quoted.csv";
    std::ofstream(csv) << "1,\"\"\n";
    mustRun("CREATE NODE TABLE T(id INT64 PRIMARY KEY, s STRING);");
    mustRun("COPY T FROM '" + csv + "' (header=false);");
    EXPECT_EQ(queryInt("MATCH (t:T) WHERE t.s IS NULL RETURN count(*);"), 0)
        << "[check: quoted-empty-is-not-null] ";
}

// Ladybug a8c619830. Un CSV qui finit par un champ vide sans saut de ligne est refusé.
TEST_F(UpstreamFixes, CsvEndingWithAnEmptyFieldAndNoNewline) {
    const auto csv = databasePath + ".trailing.csv";
    std::ofstream(csv) << "1,x,";
    mustRun("CREATE NODE TABLE U(id INT64 PRIMARY KEY, a STRING, b STRING);");
    auto copy = conn->query("COPY U FROM '" + csv + "' (header=false);");
    EXPECT_TRUE(copy->isSuccess())
        << "[check: copy-accepts-trailing-empty-field] " << copy->getErrorMessage();
    EXPECT_EQ(queryInt("MATCH (u:U) WHERE u.id = 1 AND u.a = 'x' AND u.b IS NULL RETURN count(*);"),
        1)
        << "[check: copy-accepts-trailing-empty-field] the row read back";
}

// Ladybug f03139f95. Un accent grave doublé dans un nom n'est pas réduit.
TEST_F(UpstreamFixes, EscapedBacktickInAName) {
    mustRun("CREATE NODE TABLE `a``b`(id INT64 PRIMARY KEY);");
    auto tables = conn->query("CALL show_tables() RETURN name;");
    ASSERT_TRUE(tables->isSuccess()) << "[check: query] " << tables->getErrorMessage();
    EXPECT_EQ(tables->getNext()->getValue(0)->toString(), "a`b")
        << "[check: escaped-backtick-collapsed] ";
}

// Ladybug ef1e0e3cc. MaterializedQueryResult::toString avance le curseur du résultat.
TEST_F(UpstreamFixes, ResultToStringKeepsTheCursor) {
    mustRun("CREATE NODE TABLE P(id INT64 PRIMARY KEY);");
    mustRun("UNWIND range(1, 3) AS i CREATE (:P {id: i});");
    auto result = conn->query("MATCH (p:P) RETURN p.id;");
    result->toString();
    EXPECT_TRUE(result->hasNext()) << "[check: to-string-keeps-cursor] ";
}

} // namespace

#endif
