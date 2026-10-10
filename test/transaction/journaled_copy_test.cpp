// Le chargement en masse journalisé, les nœuds puis les relations (étapes 2 et 3 ; page de conception
// extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/04-le-chargement-en-masse-journalise.md).
//
// Avec CALL force_checkpoint_on_copy=false, un COPY écrit ses lignes au journal de sa
// transaction au lieu de forcer un point de reprise. Chaque cas fait écrire un processus
// enfant, qui se tue (SIGKILL) base ouverte ; le journal est vérifié non vide, donc rien n'a
// passé de point de reprise ; puis ce processus-ci, qui n'a jamais ouvert la base, la rouvre :
// ce qu'il lit vient du rejeu.

#ifndef __SINGLE_THREADED__

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <functional>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/table/node_table.h"
#include "test_runner/fsm_leak_checker.h"
#include "storage/storage_utils.h"
#include "storage/wal/local_wal.h"
#include "transaction/transaction.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class JournaledCopyTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "needs an on-disk database: the subject is its journal";
        }
        csvPath = (std::filesystem::temp_directory_path() /
                   ("journaled_copy_" + std::to_string(::getpid()) + ".csv"))
                      .string();
        relCsvPath = (std::filesystem::temp_directory_path() /
                      ("journaled_copy_rel_" + std::to_string(::getpid()) + ".csv"))
                         .string();
    }

    void TearDown() override {
        std::filesystem::remove(relCsvPath);
        std::filesystem::remove(csvPath);
        EmptyDBTest::TearDown();
    }

    // Écrit les lignes id, name, score pour id dans [first, first + count).
    void writeCsv(int64_t first, int64_t count, const std::string& extraLine = "") const {
        std::ofstream csv(csvPath);
        for (int64_t id = first; id < first + count; id++) {
            csv << id << ",name " << id << "," << id << ".5\n";
        }
        csv << extraLine;
    }

    // Écrit les relations i -> i + step, poids i, pour i dans [first, first + count).
    void writeRelCsv(int64_t first, int64_t count, int64_t step) const {
        std::ofstream csv(relCsvPath);
        for (int64_t i = first; i < first + count; i++) {
            csv << i << "," << i + step << "," << i << "\n";
        }
    }

    std::string relCopyStatement(const std::string& options = "") const {
        return "COPY Link FROM '" + relCsvPath + "'" + options + ";";
    }

    // `count` relations dans chaque sens, chacune de i vers i + step avec le poids i.
    void expectRels(int64_t count, int64_t step) {
        EXPECT_EQ(single("MATCH (:Doc)-[l:Link]->(:Doc) RETURN count(l);"), count);
        EXPECT_EQ(single("MATCH (:Doc)<-[l:Link]-(:Doc) RETURN count(l);"), count);
        EXPECT_EQ(single("MATCH (a:Doc)-[l:Link]->(b:Doc) WHERE b.id <> a.id + " +
                         std::to_string(step) + " OR l.weight <> a.id RETURN count(l);"),
            0);
        EXPECT_EQ(single("MATCH (b:Doc)<-[l:Link]-(a:Doc) WHERE b.id <> a.id + " +
                         std::to_string(step) + " RETURN count(l);"),
            0);
    }

    std::string copyStatement(const std::string& options = "") const {
        return "COPY Doc FROM '" + csvPath + "'" + options + ";";
    }

    // Le processus enfant : ouvre la base, règle, exécute `body`, et se tue base ouverte. Une
    // instruction qui échoue le fait sortir par un code, que le parent voit.
    void writeThenDie(const std::function<void(Connection&)>& body, bool journaled = true) {
        const auto child = fork();
        ASSERT_GE(child, 0);
        if (child == 0) {
            auto status = 3;
            try {
                Database childDatabase(databasePath, *systemConfig);
                Connection childConn(&childDatabase);
                must(childConn, "CALL auto_checkpoint=false;");
                if (journaled) {
                    must(childConn, "CALL force_checkpoint_on_copy=false;");
                }
                body(childConn);
                kill(getpid(), SIGKILL);
            } catch (const std::exception& e) {
                fprintf(stderr, "[child] %s\n", e.what());
                status = 4;
            }
            _exit(status);
        }
        int status = 0;
        ASSERT_EQ(waitpid(child, &status, 0), child);
        ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
            << "the child must die by SIGKILL with the database open; exit code "
            << (WIFEXITED(status) ? WEXITSTATUS(status) : -1);
    }

    static void must(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        if (!result->isSuccess()) {
            fprintf(stderr, "[child] %s : %s\n", query.c_str(), result->getErrorMessage().c_str());
            _exit(5);
        }
    }

    static void schema(Connection& connection) {
        must(connection, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING, score DOUBLE);");
        must(connection, "CREATE REL TABLE Link(FROM Doc TO Doc, weight INT64);");
        // Le schéma passe son point de reprise ; ce qui suit n'en passera aucun.
        must(connection, "CHECKPOINT;");
    }

    uint64_t journalSize() const {
        std::error_code ec;
        const auto size = std::filesystem::file_size(
            rag3db::storage::StorageUtils::getWALFilePath(databasePath), ec);
        return ec ? 0 : size;
    }

    int64_t single(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    std::string text(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<std::string>() :
                   "<none>";
    }

    // Chaque ligne de [first, first + count) est là, se retrouve par sa clé, et porte ses
    // valeurs.
    void expectRows(int64_t first, int64_t count) {
        EXPECT_EQ(single("MATCH (n:Doc) WHERE n.id >= " + std::to_string(first) + " AND n.id < " +
                         std::to_string(first + count) + " RETURN count(*);"),
            count);
        for (const auto id : {first, first + count / 2, first + count - 1}) {
            EXPECT_EQ(single("MATCH (n:Doc {id: " + std::to_string(id) + "}) RETURN count(*);"), 1)
                << id;
            EXPECT_EQ(text("MATCH (n:Doc {id: " + std::to_string(id) + "}) RETURN n.name;"),
                "name " + std::to_string(id));
        }
        EXPECT_EQ(single("MATCH (n:Doc) WHERE n.id >= " + std::to_string(first) + " AND n.id < " +
                         std::to_string(first + count) +
                         " AND n.score = CAST(n.id AS DOUBLE) + 0.5 RETURN count(*);"),
            count);
    }

    std::string csvPath;
    std::string relCsvPath;
};

// Mort juste après un COPY validé : toutes ses lignes, par le journal seul.
TEST_F(JournaledCopyTest, ACommittedCopySurvivesADeathWithoutAnyCheckpoint) {
    writeCsv(0, 5000);
    writeThenDie([&](Connection& child) {
        schema(child);
        must(child, copyStatement());
    });
    ASSERT_GT(journalSize(), 0u) << "the COPY must not have checkpointed";
    createDBAndConn();
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 5000);
    expectRows(0, 5000);
    // La table reste utilisable, et une réouverture de plus rend la même chose.
    ASSERT_TRUE(conn->query("CREATE (:Doc {id: 5000, name: 'name 5000', score: 5000.5});")
                    ->isSuccess());
    createDBAndConn();
    expectRows(0, 5001);
}

// Plus d'un groupe de nœuds (131 072 lignes) : les décalages se rejouent au-delà du premier.
TEST_F(JournaledCopyTest, ACopyLargerThanOneNodeGroupSurvives) {
    writeCsv(0, 150000);
    writeThenDie([&](Connection& child) {
        schema(child);
        must(child, copyStatement());
    });
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 150000);
    expectRows(0, 150000);
    EXPECT_EQ(single("MATCH (n:Doc) RETURN max(offset(id(n)));"), 149999);
}

// Mort avant la validation : rien de ce COPY, tout de ce qui était validé avant.
TEST_F(JournaledCopyTest, AnUncommittedCopyLeavesNothing) {
    writeThenDie([&](Connection& child) {
        schema(child);
        writeCsv(0, 1000);
        must(child, copyStatement());
        writeCsv(1000, 2000);
        must(child, "BEGIN TRANSACTION;");
        must(child, copyStatement());
    });
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 1000);
    expectRows(0, 1000);
    EXPECT_EQ(single("MATCH (n:Doc {id: 1500}) RETURN count(*);"), 0);
    // Les clés du COPY perdu sont libres.
    writeCsv(1000, 2000);
    ASSERT_TRUE(conn->query(copyStatement())->isSuccess());
    expectRows(0, 3000);
}

// Des écritures ordinaires qui se réfèrent aux lignes d'un COPY, dans la même transaction et
// dans les suivantes : mise à jour, suppression, relations. Les relations sont journalisées
// par le décalage de leurs extrémités : elles ne sont justes que si le rejeu redonne aux
// lignes du COPY les mêmes décalages.
TEST_F(JournaledCopyTest, OrdinaryWritesReferringToCopiedRowsAreReplayedOnTheSameRows) {
    writeThenDie([&](Connection& child) {
        schema(child);
        must(child, "BEGIN TRANSACTION;");
        writeCsv(0, 3000);
        must(child, copyStatement());
        writeCsv(3000, 3000);
        must(child, copyStatement());
        // Les insertions ordinaires viennent après les COPY de la transaction (l'ordre inverse
        // est refusé, voir plus bas).
        must(child, "CREATE (:Doc {id: 100000, name: 'after the copies', score: 0.0});");
        must(child, "CREATE (:Doc {id: 100001, name: 'between', score: 0.0});");
        must(child, "MATCH (n:Doc {id: 10}) SET n.name = 'changed in the transaction';");
        must(child, "COMMIT;");
        must(child, "MATCH (n:Doc {id: 4000}) SET n.name = 'changed afterwards';");
        must(child, "MATCH (n:Doc {id: 11}) DELETE n;");
        must(child, "MATCH (a:Doc {id: 20}), (b:Doc {id: 5999}) CREATE (a)-[:Link {weight: 1}]->(b);");
        must(child, "MATCH (a:Doc {id: 100000}), (b:Doc {id: 3000}) CREATE "
                    "(a)-[:Link {weight: 2}]->(b);");
        must(child, "MATCH (a:Doc {id: 2999}), (b:Doc {id: 100001}) CREATE "
                    "(a)-[:Link {weight: 3}]->(b);");
    });
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 6001);
    EXPECT_EQ(text("MATCH (n:Doc {id: 100000}) RETURN n.name;"), "after the copies");
    EXPECT_EQ(text("MATCH (n:Doc {id: 10}) RETURN n.name;"), "changed in the transaction");
    EXPECT_EQ(text("MATCH (n:Doc {id: 4000}) RETURN n.name;"), "changed afterwards");
    EXPECT_EQ(single("MATCH (n:Doc {id: 11}) RETURN count(*);"), 0);
    EXPECT_EQ(text("MATCH (n:Doc {id: 12}) RETURN n.name;"), "name 12");
    EXPECT_EQ(text("MATCH (n:Doc {id: 100001}) RETURN n.name;"), "between");
    EXPECT_EQ(single("MATCH (:Doc)-[l:Link]->(:Doc) RETURN count(l);"), 3);
    EXPECT_EQ(single("MATCH (a:Doc {id: 20})-[l:Link]->(b:Doc) RETURN b.id;"), 5999);
    EXPECT_EQ(single("MATCH (a:Doc {id: 100000})-[l:Link]->(b:Doc) RETURN b.id;"), 3000);
    EXPECT_EQ(single("MATCH (a:Doc)-[l:Link]->(b:Doc {id: 100001}) RETURN a.id;"), 2999);
    EXPECT_EQ(single("MATCH (a:Doc)-[l:Link {weight: 1}]->(b:Doc) RETURN a.id;"), 20);
}

// Un COPY annulé, un COPY refusé pour clé en double, puis un COPY validé : seul le dernier.
TEST_F(JournaledCopyTest, OnlyTheCommittedCopyIsReplayedAfterARolledBackAndARefusedOne) {
    writeThenDie([&](Connection& child) {
        schema(child);
        writeCsv(0, 500);
        must(child, copyStatement());
        writeCsv(500, 500);
        must(child, "BEGIN TRANSACTION;");
        must(child, copyStatement());
        must(child, "ROLLBACK;");
        writeCsv(1000, 500, "7,duplicate,0.0\n");
        if (child.query(copyStatement())->isSuccess()) {
            _exit(6); // la clé 7 existe : ce COPY doit être refusé
        }
        writeCsv(2000, 500);
        must(child, copyStatement());
    });
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 1000);
    expectRows(0, 500);
    expectRows(2000, 500);
    EXPECT_EQ(single("MATCH (n:Doc {id: 600}) RETURN count(*);"), 0);
    EXPECT_EQ(single("MATCH (n:Doc {id: 1200}) RETURN count(*);"), 0);
    EXPECT_EQ(text("MATCH (n:Doc {id: 7}) RETURN n.name;"), "name 7");
}

// ── Les relations (étape 3) ──────────────────────────────────────────────────────────────────

// Mort juste après un COPY de relations validé : toutes ses relations, par le journal seul,
// dans les deux sens, avec leurs extrémités et leurs propriétés.
TEST_F(JournaledCopyTest, ACommittedRelationCopySurvivesADeathWithoutAnyCheckpoint) {
    writeCsv(0, 3000);
    writeRelCsv(0, 2500, 1);
    writeThenDie([&](Connection& child) {
        schema(child);
        must(child, copyStatement());
        must(child, relCopyStatement());
    });
    ASSERT_GT(journalSize(), 0u) << "neither COPY must have checkpointed";
    createDBAndConn();
    expectRels(2500, 1);
    EXPECT_EQ(single("MATCH (a:Doc {id: 1200})-[l:Link]->(b:Doc) RETURN b.id;"), 1201);
    EXPECT_EQ(single("MATCH (b:Doc {id: 1200})<-[l:Link]-(a:Doc) RETURN a.id;"), 1199);
    createDBAndConn(); // une réouverture de plus rend la même chose
    expectRels(2500, 1);
}

// Mort avant la validation d'un COPY de relations : aucune de ses relations.
TEST_F(JournaledCopyTest, AnUncommittedRelationCopyLeavesNothing) {
    writeCsv(0, 1000);
    writeRelCsv(0, 900, 1);
    writeThenDie([&](Connection& child) {
        schema(child);
        must(child, copyStatement());
        must(child, "BEGIN TRANSACTION;");
        must(child, relCopyStatement());
    });
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 1000);
    EXPECT_EQ(single("MATCH (:Doc)-[l:Link]->(:Doc) RETURN count(l);"), 0);
    ASSERT_TRUE(conn->query(relCopyStatement())->isSuccess());
    expectRels(900, 1);
}

// Des écritures ordinaires qui visent des relations d'un COPY : mise à jour, suppression. Elles
// sont journalisées par l'identité de la relation : elles ne sont justes au rejeu que si le
// COPY rejoué redonne les mêmes identités, quel que soit le fil qui a écrit quel lot.
TEST_F(JournaledCopyTest, OrdinaryWritesReferringToCopiedRelationsAreReplayedOnTheSameOnes) {
    // Assez de relations pour que plusieurs fils se partagent le COPY : 236 000, quatre par
    // nœud d'origine, de i vers i + k avec le poids 10 i + k.
    static constexpr int64_t NUM_SOURCES = 59000;
    writeCsv(0, 60000);
    {
        std::ofstream csv(relCsvPath);
        for (int64_t i = 0; i < NUM_SOURCES; i++) {
            for (int64_t k = 1; k <= 4; k++) {
                csv << i << "," << i + k << "," << 10 * i + k << "\n";
            }
        }
    }
    writeThenDie([&](Connection& child) {
        schema(child);
        must(child, copyStatement());
        must(child, relCopyStatement());
        must(child, "MATCH (a:Doc {id: 10})-[l:Link {weight: 102}]->(:Doc) SET l.weight = 777;");
        must(child,
            "MATCH (a:Doc {id: 40000})-[l:Link {weight: 400003}]->(:Doc) SET l.weight = 888;");
        must(child, "MATCH (a:Doc {id: 11})-[l:Link {weight: 114}]->(:Doc) DELETE l;");
        must(child, "MATCH (a:Doc {id: 58999})-[l:Link {weight: 589991}]->(:Doc) DELETE l;");
        must(child,
            "MATCH (a:Doc {id: 5}), (b:Doc {id: 50}) CREATE (a)-[:Link {weight: 9}]->(b);");
    });
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    const auto total = 4 * NUM_SOURCES - 2 + 1;
    EXPECT_EQ(single("MATCH (:Doc)-[l:Link]->(:Doc) RETURN count(l);"), total);
    EXPECT_EQ(single("MATCH (:Doc)<-[l:Link]-(:Doc) RETURN count(l);"), total);
    EXPECT_EQ(single("MATCH (a:Doc {id: 10})-[l:Link {weight: 777}]->(b:Doc) RETURN b.id;"), 12);
    EXPECT_EQ(
        single("MATCH (a:Doc {id: 40000})-[l:Link {weight: 888}]->(b:Doc) RETURN b.id;"), 40003);
    EXPECT_EQ(single("MATCH (a:Doc {id: 11})-[l:Link]->(b:Doc) RETURN sum(b.id);"), 12 + 13 + 14);
    EXPECT_EQ(single("MATCH (a:Doc {id: 58999})-[l:Link]->(:Doc) RETURN count(l);"), 3);
    EXPECT_EQ(single("MATCH (a:Doc {id: 5})-[l:Link {weight: 9}]->(b:Doc) RETURN b.id;"), 50);
    // Aucune autre relation n'a bougé : chacune relie i à i + k avec le poids 10 i + k, sauf
    // les deux modifiées et celle qui a été créée.
    EXPECT_EQ(single("MATCH (a:Doc)-[l:Link]->(b:Doc) WHERE l.weight <> 10 * a.id + (b.id - "
                     "a.id) OR b.id - a.id < 1 OR b.id - a.id > 4 RETURN count(l);"),
        3);
}

// La forme de la transaction par paquet : dans une transaction, un petit lot de relations par
// MERGE, puis un gros par COPY dans la même table, sur les mêmes nœuds d'origine.
TEST_F(JournaledCopyTest, MergedThenCopiedRelationsOfOneTransactionSurvive) {
    writeCsv(0, 1000);
    writeRelCsv(0, 900, 2);
    writeThenDie([&](Connection& child) {
        schema(child);
        must(child, copyStatement());
        must(child, "BEGIN TRANSACTION;");
        must(child, "UNWIND range(0, 19) AS i MATCH (a:Doc {id: i}) WITH a, i MATCH (b:Doc {id: i "
                    "+ 1}) MERGE (a)-[l:Link]->(b) ON CREATE SET l.weight = 10000 + i;");
        must(child, relCopyStatement());
        must(child, "COMMIT;");
    });
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    EXPECT_EQ(single("MATCH (:Doc)-[l:Link]->(:Doc) RETURN count(l);"), 920);
    EXPECT_EQ(single("MATCH (:Doc)<-[l:Link]-(:Doc) RETURN count(l);"), 920);
    EXPECT_EQ(single("MATCH (a:Doc)-[l:Link]->(b:Doc) WHERE b.id = a.id + 1 AND l.weight = 10000 "
                     "+ a.id RETURN count(l);"),
        20);
    EXPECT_EQ(single("MATCH (a:Doc)-[l:Link]->(b:Doc) WHERE b.id = a.id + 2 AND l.weight = a.id "
                     "RETURN count(l);"),
        900);
    EXPECT_EQ(single("MATCH (a:Doc {id: 5})-[l:Link]->(b:Doc) RETURN sum(b.id);"), 6 + 7);
    EXPECT_EQ(single("MATCH (b:Doc {id: 7})<-[l:Link]-(a:Doc) RETURN sum(a.id);"), 6 + 5);
}

// L'autre forme de la transaction par paquet : des nœuds créés par MERGE dans la transaction,
// puis des relations par COPY vers eux, les deux bouts pouvant être tout juste créés.
TEST_F(JournaledCopyTest, RelationsCopiedToNodesMergedInTheSameTransactionSurvive) {
    writeCsv(0, 300);
    writeRelCsv(100, 600, 1);
    writeThenDie([&](Connection& child) {
        schema(child);
        must(child, copyStatement());
        must(child, "BEGIN TRANSACTION;");
        must(child, "UNWIND range(250, 799) AS i MERGE (n:Doc {id: i}) ON CREATE SET n.name = "
                    "'merged ' + CAST(i AS STRING);");
        must(child, relCopyStatement());
        must(child, "COMMIT;");
    });
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 800);
    EXPECT_EQ(single("MATCH (:Doc)-[l:Link]->(:Doc) RETURN count(l);"), 600);
    EXPECT_EQ(single("MATCH (:Doc)<-[l:Link]-(:Doc) RETURN count(l);"), 600);
    EXPECT_EQ(single("MATCH (a:Doc)-[l:Link]->(b:Doc) WHERE b.id <> a.id + 1 OR l.weight <> a.id "
                     "RETURN count(l);"),
        0);
    EXPECT_EQ(text("MATCH (a:Doc {id: 299})-[l:Link]->(b:Doc) RETURN b.name;"), "merged 300");
    EXPECT_EQ(text("MATCH (b:Doc {id: 500})<-[l:Link]-(a:Doc) RETURN a.name;"), "merged 499");
    EXPECT_EQ(text("MATCH (a:Doc {id: 150})-[l:Link]->(b:Doc) RETURN b.name;"), "name 151");
}

// Un COPY de relations qui peut écarter des lignes (IGNORE_ERRORS) n'est pas journalisé : il
// force son point de reprise, et ses relations survivent par lui.
TEST_F(JournaledCopyTest, ARelationCopyThatMaySkipRowsStillForcesItsCheckpoint) {
    writeCsv(0, 100);
    writeRelCsv(0, 99, 1);
    writeThenDie([&](Connection& child) {
        schema(child);
        must(child, copyStatement());
        must(child, relCopyStatement("(IGNORE_ERRORS=true)"));
    });
    EXPECT_EQ(journalSize(), 0u) << "the relation COPY must have checkpointed";
    createDBAndConn();
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 100);
    expectRels(99, 1);
}

// Sans le réglage, rien ne change : un COPY force son point de reprise et laisse un journal
// vide.
TEST_F(JournaledCopyTest, ByDefaultACopyStillForcesItsCheckpoint) {
    writeCsv(0, 1000);
    writeThenDie(
        [&](Connection& child) {
            schema(child);
            must(child, copyStatement());
        },
        false /* journaled */);
    EXPECT_EQ(journalSize(), 0u);
    createDBAndConn();
    expectRows(0, 1000);
}

// Des lignes écartées (IGNORE_ERRORS) : les décalages ne se rejoueraient pas à l'identique, ce
// COPY-là garde son point de reprise.
TEST_F(JournaledCopyTest, ACopyThatSkipsRowsStillForcesItsCheckpoint) {
    writeThenDie([&](Connection& child) {
        schema(child);
        writeCsv(0, 200);
        must(child, copyStatement());
        writeCsv(200, 300, "7,duplicate,0.0\n");
        must(child, copyStatement("(IGNORE_ERRORS=true)"));
    });
    EXPECT_EQ(journalSize(), 0u) << "a COPY with skipped rows must have checkpointed";
    createDBAndConn();
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 500);
    expectRows(0, 500);
}

// Après ALTER TABLE … DROP puis ADD d'une colonne. Le stockage garde l'emplacement de la
// colonne retirée ; le journal, lui, porte les propriétés du catalogue dans leur ordre. Un COPY
// de relations journalisé comptait les colonnes du stockage et lisait après la fin de son plan
// (plantage, trouvé par le test Cypher d'origine ddl_empty.CopyRelAfterDropAddColNewGroup dès
// que le chargement journalisé a été essayé par défaut) ; un COPY de nœuds écrivait les
// colonnes du stockage au lieu des propriétés.
// (La colonne retirée est déclarée APRÈS la clé : retirer une colonne déclarée avant elle
// casse l'insertion ordinaire elle-même — défaut d'origine, ticket du 5 octobre 2026.)
class JournaledCopyAfterAlterTest : public JournaledCopyTest {
protected:
    // Une table dont une colonne est retirée, puis une colonne ajoutée ; une table de relations
    // de même.
    static void alteredSchema(Connection& connection) {
        must(connection, "CREATE NODE TABLE Item(id INT64, extra STRING, name STRING, PRIMARY KEY "
                         "(id));");
        must(connection, "CREATE REL TABLE Tie(FROM Item TO Item, comment STRING, weight INT64);");
        must(connection, "ALTER TABLE Item DROP extra;");
        must(connection, "ALTER TABLE Item ADD age INT64;");
        must(connection, "ALTER TABLE Tie DROP comment;");
        must(connection, "ALTER TABLE Tie ADD since INT64;");
        must(connection, "CHECKPOINT;");
    }

    // id, name, age pour id dans [0, count).
    void writeItems(int64_t count) const {
        std::ofstream csv(csvPath);
        for (int64_t id = 0; id < count; id++) {
            csv << id << ",item " << id << "," << id + 20 << "\n";
        }
    }

    // i -> i + 1, weight i, since 2000 + i, pour i dans [0, count).
    void writeTies(int64_t count) const {
        std::ofstream csv(relCsvPath);
        for (int64_t i = 0; i < count; i++) {
            csv << i << "," << i + 1 << "," << i << "," << 2000 + i << "\n";
        }
    }

    void expectItems(int64_t count) {
        EXPECT_EQ(single("MATCH (n:Item) RETURN count(*);"), count);
        EXPECT_EQ(single("MATCH (n:Item) WHERE n.age = n.id + 20 AND n.name = 'item ' + CAST(n.id "
                         "AS STRING) RETURN count(*);"),
            count);
        for (const auto id : {int64_t{0}, count / 2, count - 1}) {
            EXPECT_EQ(text("MATCH (n:Item {id: " + std::to_string(id) + "}) RETURN n.name;"),
                "item " + std::to_string(id));
        }
    }

    void expectTies(int64_t count) {
        EXPECT_EQ(single("MATCH (:Item)-[t:Tie]->(:Item) RETURN count(t);"), count);
        EXPECT_EQ(single("MATCH (:Item)<-[t:Tie]-(:Item) RETURN count(t);"), count);
        EXPECT_EQ(single("MATCH (a:Item)-[t:Tie]->(b:Item) WHERE b.id = a.id + 1 AND t.weight = "
                         "a.id AND t.since = 2000 + a.id RETURN count(t);"),
            count);
        EXPECT_EQ(single("MATCH (b:Item)<-[t:Tie]-(a:Item) WHERE b.id = a.id + 1 AND t.weight = "
                         "a.id AND t.since = 2000 + a.id RETURN count(t);"),
            count);
    }
};

// Des COPY de nœuds puis de relations, journalisés, après le DROP et l'ADD ; la mort ; le rejeu.
TEST_F(JournaledCopyAfterAlterTest, CopiesAfterADroppedAndAnAddedColumnAreReplayed) {
    writeItems(500);
    writeTies(499);
    writeThenDie([&](Connection& child) {
        alteredSchema(child);
        must(child, "COPY Item FROM '" + csvPath + "';");
        must(child, "COPY Tie FROM '" + relCsvPath + "';");
    });
    ASSERT_GT(journalSize(), 0u) << "nothing must have checkpointed";
    createDBAndConn();
    expectItems(500);
    expectTies(499);
    ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
    createDBAndConn();
    expectItems(500);
    expectTies(499);
}

// Les mêmes lignes par le chemin ordinaire : garde-fou, ce chemin journalise déjà les
// propriétés dans leur ordre.
TEST_F(JournaledCopyAfterAlterTest, OrdinaryInsertsAfterADroppedAndAnAddedColumnAreReplayed) {
    writeThenDie([&](Connection& child) {
        alteredSchema(child);
        must(child, "UNWIND range(0, 499) AS i CREATE (:Item {id: i, name: 'item ' + CAST(i AS "
                    "STRING), age: i + 20});");
        must(child, "UNWIND range(0, 498) AS i MATCH (a:Item {id: i}), (b:Item {id: i + 1}) "
                    "CREATE (a)-[:Tie {weight: i, since: 2000 + i}]->(b);");
    });
    ASSERT_GT(journalSize(), 0u) << "nothing must have checkpointed";
    createDBAndConn();
    expectItems(500);
    expectTies(499);
}

// Sans mort : les mêmes COPY, lus dans la session, après un point de reprise et une
// réouverture — sous les deux réglages.
TEST_F(JournaledCopyAfterAlterTest, CopiesAfterADroppedAndAnAddedColumnInOneSession) {
    writeItems(500);
    writeTies(499);
    for (const auto* setting : {"CALL force_checkpoint_on_copy=true;",
             "CALL force_checkpoint_on_copy=false;"}) {
        SCOPED_TRACE(setting);
        conn.reset();
        database.reset();
        std::filesystem::remove(databasePath);
        std::filesystem::remove(rag3db::storage::StorageUtils::getWALFilePath(databasePath));
        createDBAndConn();
        alteredSchema(*conn);
        ASSERT_TRUE(conn->query(setting)->isSuccess());
        auto copy = conn->query("COPY Item FROM '" + csvPath + "';");
        ASSERT_TRUE(copy->isSuccess()) << copy->getErrorMessage();
        copy = conn->query("COPY Tie FROM '" + relCsvPath + "';");
        ASSERT_TRUE(copy->isSuccess()) << copy->getErrorMessage();
        expectItems(500);
        expectTies(499);
        ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
        createDBAndConn();
        expectItems(500);
        expectTies(499);
    }
}

// Dans une transaction, des insertions ordinaires puis un COPY dans la même table (défaut
// d'origine : les lignes locales portent des décalages provisoires qui commencent au nombre de
// lignes de la table, et le COPY écrivait les siennes aux mêmes décalages — par la clé d'une
// ligne du COPY on atteignait une ligne locale ; d'abord refusé par un nom, 0f4a54b2c). Au
// début du COPY, les lignes locales de la table sont maintenant versées dans la table, comme le
// ferait le commit : elles deviennent des lignes non validées ordinaires, et le COPY ajoute à
// la suite. Chaque cas est joué sous les deux réglages du chargement journalisé.
class CopyAfterInsertsTest : public JournaledCopyTest {
protected:
    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    void openWithSchema() {
        createDBAndConn();
        ok("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING, score DOUBLE);");
        ok("CREATE REL TABLE Link(FROM Doc TO Doc, weight INT64);");
    }

    // Sous chaque réglage, sur une base neuve.
    void underBothSettings(const std::function<void()>& body) {
        for (const auto* setting : {"CALL force_checkpoint_on_copy=true;",
                 "CALL force_checkpoint_on_copy=false;"}) {
            SCOPED_TRACE(setting);
            conn.reset();
            database.reset();
            std::filesystem::remove(databasePath);
            std::filesystem::remove(rag3db::storage::StorageUtils::getWALFilePath(databasePath));
            openWithSchema();
            ok(setting);
            body();
        }
    }

    // Les lignes locales 1000 … 1000 + count - 1, nommées « local <id> ».
    void insertLocals(int64_t first, int64_t count) {
        ok("UNWIND range(" + std::to_string(first) + ", " + std::to_string(first + count - 1) +
            ") AS i CREATE (:Doc {id: i, name: 'local ' + CAST(i AS STRING), score: 0.0});");
    }

    void expectLocal(int64_t id) {
        EXPECT_EQ(text("MATCH (n:Doc {id: " + std::to_string(id) + "}) RETURN n.name;"),
            "local " + std::to_string(id));
    }
};

// La forme du défaut : vingt lignes insérées, cent copiées, puis une recherche, une mise à
// jour et une relation par les clés des lignes du COPY, et des lignes locales — dans la
// transaction, après la validation, après la réouverture.
TEST_F(CopyAfterInsertsTest, TwentyInsertsThenACopyAreAllReachedByTheirOwnKeys) {
    writeCsv(0, 100);
    underBothSettings([&] {
        ok("BEGIN TRANSACTION;");
        insertLocals(1000, 20);
        // Une relation entre deux lignes locales, créée avant le COPY.
        ok("MATCH (a:Doc {id: 1003}), (b:Doc {id: 1004}) CREATE (a)-[:Link {weight: 34}]->(b);");
        ok(copyStatement());
        const auto check = [&] {
            EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 120);
            expectRows(0, 100);
            EXPECT_EQ(text("MATCH (n:Doc {id: 5}) RETURN n.name;"), "name 5");
            expectLocal(1000);
            expectLocal(1005);
            expectLocal(1019);
            EXPECT_EQ(single("MATCH (a:Doc)-[l:Link]->(b:Doc) WHERE l.weight = 34 "
                             "RETURN a.id * 10000 + b.id;"),
                1003 * 10000 + 1004);
        };
        check();
        ok("MATCH (n:Doc {id: 5}) SET n.name = 'five';");
        ok("MATCH (a:Doc {id: 7}), (b:Doc {id: 8}) CREATE (a)-[:Link {weight: 78}]->(b);");
        ok("MATCH (a:Doc {id: 1007}), (b:Doc {id: 9}) CREATE (a)-[:Link {weight: 79}]->(b);");
        const auto checkWrites = [&] {
            EXPECT_EQ(text("MATCH (n:Doc {id: 5}) RETURN n.name;"), "five");
            expectLocal(1005);
            EXPECT_EQ(single("MATCH (a:Doc)-[l:Link]->(b:Doc) WHERE l.weight = 78 "
                             "RETURN a.id * 10000 + b.id;"),
                7 * 10000 + 8);
            EXPECT_EQ(single("MATCH (b:Doc)<-[l:Link]-(a:Doc) WHERE l.weight = 79 "
                             "RETURN a.id * 10000 + b.id;"),
                1007 * 10000 + 9);
            EXPECT_EQ(single("MATCH (:Doc)-[l:Link]->(:Doc) RETURN count(l);"), 3);
            EXPECT_EQ(single("MATCH (:Doc)<-[l:Link]-(:Doc) RETURN count(l);"), 3);
        };
        checkWrites();
        ok("COMMIT;");
        EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 120);
        checkWrites();
        createDBAndConn();
        EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 120);
        checkWrites();
        expectLocal(1019);
    });
}

// Une seule ligne locale : avant, un plantage dans NodeGroup::lookup.
TEST_F(CopyAfterInsertsTest, OneInsertThenACopy) {
    writeCsv(0, 300);
    underBothSettings([&] {
        ok("BEGIN TRANSACTION;");
        insertLocals(100000, 1);
        ok(copyStatement());
        EXPECT_EQ(text("MATCH (n:Doc {id: 10}) RETURN n.name;"), "name 10");
        expectLocal(100000);
        ok("COMMIT;");
        EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 301);
        expectRows(0, 300);
        expectLocal(100000);
    });
}

// Une clé du COPY est déjà celle d'une ligne locale : le COPY est refusé pour clé en double,
// la transaction s'annule, rien ne reste et les clés sont libres.
TEST_F(CopyAfterInsertsTest, ACopyKeyAlreadyInsertedByTheTransactionIsRefused) {
    writeCsv(0, 100);
    underBothSettings([&] {
        ok("BEGIN TRANSACTION;");
        insertLocals(1000, 3);
        ok("CREATE (:Doc {id: 42, name: 'mine', score: 0.0});");
        auto refused = conn->query(copyStatement());
        ASSERT_FALSE(refused->isSuccess());
        EXPECT_NE(refused->getErrorMessage().find("duplicated primary key value 42"),
            std::string::npos)
            << refused->getErrorMessage();
        ok("ROLLBACK;");
        EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 0);
        // Les clés des lignes versées puis annulées sont libres, celles du COPY aussi.
        ok("CREATE (:Doc {id: 42, name: 'again', score: 0.0});");
        ok("CREATE (:Doc {id: 1001, name: 'again', score: 0.0});");
        ok("MATCH (n:Doc) DELETE n;");
        ok(copyStatement());
        EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 100);
        expectRows(0, 100);
        createDBAndConn();
        EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 100);
    });
}

// Une ligne locale modifiée, une autre supprimée, avant le COPY : la première garde sa valeur,
// la seconde reste supprimée et sa clé est libre.
TEST_F(CopyAfterInsertsTest, ALocalRowUpdatedOrDeletedBeforeTheCopy) {
    writeCsv(0, 100);
    underBothSettings([&] {
        ok("BEGIN TRANSACTION;");
        insertLocals(1000, 5);
        ok("MATCH (n:Doc {id: 1001}) SET n.name = 'changed';");
        ok("MATCH (n:Doc {id: 1002}) DELETE n;");
        ok(copyStatement());
        const auto check = [&] {
            EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 104);
            EXPECT_EQ(text("MATCH (n:Doc {id: 1001}) RETURN n.name;"), "changed");
            EXPECT_EQ(single("MATCH (n:Doc {id: 1002}) RETURN count(*);"), 0);
            expectLocal(1000);
            expectLocal(1004);
            expectRows(0, 100);
        };
        check();
        ok("COMMIT;");
        check();
        createDBAndConn();
        check();
        ok("CREATE (:Doc {id: 1002, name: 'local 1002', score: 0.0});");
        expectLocal(1002);
    });
}

// Des insertions après le COPY, un second COPY après elles, puis d'autres insertions : chaque
// ligne à sa clé.
TEST_F(CopyAfterInsertsTest, InsertsAfterTheCopyThenAnotherCopy) {
    underBothSettings([&] {
        ok("BEGIN TRANSACTION;");
        insertLocals(1000, 2);
        writeCsv(0, 100);
        ok(copyStatement());
        insertLocals(2000, 3);
        expectLocal(2001);
        EXPECT_EQ(text("MATCH (n:Doc {id: 50}) RETURN n.name;"), "name 50");
        ok("MATCH (a:Doc {id: 2001}), (b:Doc {id: 50}) CREATE (a)-[:Link {weight: 1}]->(b);");
        writeCsv(100, 100);
        ok(copyStatement());
        insertLocals(3000, 1);
        const auto check = [&] {
            EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 206);
            expectRows(0, 200);
            for (const auto id : {1000, 1001, 2000, 2001, 2002, 3000}) {
                expectLocal(id);
            }
            EXPECT_EQ(single("MATCH (a:Doc)-[l:Link]->(b:Doc) RETURN a.id * 10000 + b.id;"),
                2001 * 10000 + 50);
            EXPECT_EQ(single("MATCH (b:Doc)<-[l:Link]-(a:Doc) RETURN a.id * 10000 + b.id;"),
                2001 * 10000 + 50);
        };
        check();
        ok("COMMIT;");
        check();
        createDBAndConn();
        check();
    });
}

// ROLLBACK, puis la même suite rejouée et validée : la première n'a rien laissé.
TEST_F(CopyAfterInsertsTest, RolledBackThenReplayed) {
    writeCsv(0, 100);
    underBothSettings([&] {
        for (const auto* end : {"ROLLBACK;", "COMMIT;"}) {
            ok("BEGIN TRANSACTION;");
            insertLocals(1000, 20);
            ok("MATCH (n:Doc {id: 1002}) DELETE n;");
            ok(copyStatement());
            insertLocals(2000, 2);
            EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 121);
            ok(end);
            if (std::string(end) == "ROLLBACK;") {
                EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 0);
                EXPECT_EQ(single("MATCH (n:Doc {id: 5}) RETURN count(*);"), 0);
                EXPECT_EQ(single("MATCH (n:Doc {id: 1005}) RETURN count(*);"), 0);
            }
        }
        const auto check = [&] {
            EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 121);
            expectRows(0, 100);
            expectLocal(1005);
            expectLocal(2001);
            EXPECT_EQ(single("MATCH (n:Doc {id: 1002}) RETURN count(*);"), 0);
        };
        check();
        ok("CHECKPOINT;");
        check();
        createDBAndConn();
        check();
    });
}

// Une clé insérée, versée par le COPY, supprimée puis réinsérée (relecture du banc) : l'index
// porte alors deux entrées pour la clé. Validée, la clé mène à la nouvelle ligne ; annulée,
// les deux entrées partent et la clé est libre.
TEST_F(CopyAfterInsertsTest, AFlushedKeyDeletedThenInsertedAgain) {
    writeCsv(0, 100);
    underBothSettings([&] {
        for (const auto* end : {"ROLLBACK;", "COMMIT;"}) {
            ok("BEGIN TRANSACTION;");
            ok("CREATE (:Doc {id: 1000, name: 'first', score: 0.0});");
            ok(copyStatement());
            ok("MATCH (n:Doc {id: 1000}) DELETE n;");
            ok("CREATE (:Doc {id: 1000, name: 'second', score: 0.0});");
            EXPECT_EQ(text("MATCH (n:Doc {id: 1000}) RETURN n.name;"), "second");
            EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 101);
            ok(end);
            if (std::string(end) == "ROLLBACK;") {
                EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 0);
                ok("CREATE (:Doc {id: 1000, name: 'between', score: 0.0});");
                EXPECT_EQ(text("MATCH (n:Doc {id: 1000}) RETURN n.name;"), "between");
                ok("MATCH (n:Doc) DELETE n;");
            }
        }
        const auto check = [&] {
            EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 101);
            EXPECT_EQ(single("MATCH (n:Doc {id: 1000}) RETURN count(*);"), 1);
            EXPECT_EQ(text("MATCH (n:Doc {id: 1000}) RETURN n.name;"), "second");
            expectRows(0, 100);
        };
        check();
        ok("CHECKPOINT;");
        check();
        createDBAndConn();
        check();
    });
}

// Une ligne insérée avant le COPY, modifiée après lui (relecture du banc) : versée, elle est
// une ligne de la table, et sa mise à jour est journalisée comme telle, après son insertion.
// Le rejeu doit la retrouver.
TEST_F(CopyAfterInsertsTest, AnUpdateAfterTheCopyOfARowInsertedBeforeItIsReplayed) {
    writeCsv(0, 500);
    writeThenDie([&](Connection& child) {
        schema(child);
        must(child, "BEGIN TRANSACTION;");
        must(child, "UNWIND range(100000, 100004) AS i CREATE (:Doc {id: i, name: 'local ' + "
                    "CAST(i AS STRING), score: 0.0});");
        must(child, copyStatement());
        must(child, "MATCH (n:Doc {id: 100002}) SET n.name = 'after the copy';");
        must(child, "MATCH (n:Doc {id: 100003}) DELETE n;");
        must(child, "MATCH (n:Doc {id: 7}) SET n.name = 'seven';");
        must(child, "COMMIT;");
    });
    ASSERT_GT(journalSize(), 0u) << "nothing must have checkpointed";
    createDBAndConn();
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 504);
    EXPECT_EQ(text("MATCH (n:Doc {id: 100002}) RETURN n.name;"), "after the copy");
    EXPECT_EQ(text("MATCH (n:Doc {id: 100001}) RETURN n.name;"), "local 100001");
    EXPECT_EQ(single("MATCH (n:Doc {id: 100003}) RETURN count(*);"), 0);
    EXPECT_EQ(text("MATCH (n:Doc {id: 7}) RETURN n.name;"), "seven");
    EXPECT_EQ(text("MATCH (n:Doc {id: 8}) RETURN n.name;"), "name 8");
}

// Par le journal seul : des insertions, un COPY, d'autres insertions, des relations, validés,
// puis la mort base ouverte. Le rejeu remet chaque ligne à sa clé.
TEST_F(CopyAfterInsertsTest, SurvivesADeathWithoutAnyCheckpoint) {
    writeCsv(0, 3000);
    writeThenDie([&](Connection& child) {
        schema(child);
        must(child, "BEGIN TRANSACTION;");
        must(child, "UNWIND range(100000, 100019) AS i CREATE (:Doc {id: i, name: 'local ' + "
                    "CAST(i AS STRING), score: 0.0});");
        must(child, "MATCH (n:Doc {id: 100002}) DELETE n;");
        must(child, "MATCH (n:Doc {id: 100003}) SET n.name = 'changed';");
        must(child, copyStatement());
        must(child, "CREATE (:Doc {id: 200000, name: 'local 200000', score: 0.0});");
        must(child, "MATCH (a:Doc {id: 100005}), (b:Doc {id: 7}) CREATE (a)-[:Link {weight: "
                    "57}]->(b);");
        must(child, "MATCH (a:Doc {id: 8}), (b:Doc {id: 200000}) CREATE (a)-[:Link {weight: "
                    "82}]->(b);");
        must(child, "COMMIT;");
    });
    ASSERT_GT(journalSize(), 0u) << "nothing must have checkpointed";
    createDBAndConn();
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 3020);
    expectRows(0, 3000);
    EXPECT_EQ(text("MATCH (n:Doc {id: 100005}) RETURN n.name;"), "local 100005");
    EXPECT_EQ(text("MATCH (n:Doc {id: 100003}) RETURN n.name;"), "changed");
    EXPECT_EQ(single("MATCH (n:Doc {id: 100002}) RETURN count(*);"), 0);
    EXPECT_EQ(text("MATCH (n:Doc {id: 200000}) RETURN n.name;"), "local 200000");
    EXPECT_EQ(single("MATCH (a:Doc)-[l:Link]->(b:Doc) WHERE l.weight = 57 RETURN a.id * 1000000 "
                     "+ b.id;"),
        100005ll * 1000000 + 7);
    EXPECT_EQ(single("MATCH (b:Doc)<-[l:Link]-(a:Doc) WHERE l.weight = 82 RETURN a.id * 1000000 "
                     "+ b.id;"),
        8ll * 1000000 + 200000);
}

// Les statistiques d'une table (STATS_INFO : la cardinalité, les comptes de distincts) suivent
// la transaction d'un COPY comme elles suivent celle d'un CREATE : vues par elle, fusionnées à
// sa validation, jetées à son annulation. Le COPY les fusionnait dans la table pendant son
// exécution — avant la validation, et avant le refus pour une clé en double ; et le versement
// des lignes locales qui précède un COPY fusionnait celles des CREATE, puis les comptait une
// seconde fois. Les comptes de distincts sont des estimations (HyperLogLog) : ils sont comparés
// à ce qu'ils valaient, pas à un nombre.
class CopyStatisticsTest : public CopyAfterInsertsTest {
protected:
    int64_t cardinality() { return single("CALL STATS_INFO('Doc') RETURN cardinality;"); }
    int64_t distinctNames() {
        return single("CALL STATS_INFO('Doc') RETURN name_distinct_count;");
    }
};

TEST_F(CopyStatisticsTest, ARolledBackCopyLeavesThemAsTheyWere) {
    underBothSettings([&] {
        writeCsv(0, 100);
        ok(copyStatement());
        ASSERT_EQ(cardinality(), 100);
        const auto namesBefore = distinctNames();
        writeCsv(100, 5000);
        ok("BEGIN TRANSACTION;");
        ok(copyStatement());
        EXPECT_EQ(cardinality(), 5100) << "the transaction sees its own copy";
        EXPECT_GT(distinctNames(), namesBefore);
        ok("ROLLBACK;");
        EXPECT_EQ(cardinality(), 100);
        EXPECT_EQ(distinctNames(), namesBefore);
    });
}

TEST_F(CopyStatisticsTest, ARefusedCopyLeavesThemAsTheyWere) {
    underBothSettings([&] {
        writeCsv(0, 100);
        ok(copyStatement());
        const auto namesBefore = distinctNames();
        writeCsv(100, 5000, "7,again,0.5\n");
        auto refused = conn->query(copyStatement());
        ASSERT_FALSE(refused->isSuccess());
        EXPECT_EQ(cardinality(), 100);
        EXPECT_EQ(distinctNames(), namesBefore);
    });
}

TEST_F(CopyStatisticsTest, ACommittedCopyIsCountedOnce) {
    underBothSettings([&] {
        writeCsv(0, 5000);
        ok("BEGIN TRANSACTION;");
        ok(copyStatement());
        EXPECT_EQ(cardinality(), 5000);
        ok("COMMIT;");
        EXPECT_EQ(cardinality(), 5000);
        createDBAndConn();
        EXPECT_EQ(cardinality(), 5000);
    });
}

TEST_F(CopyStatisticsTest, InsertsThenACopyAreCountedOnce) {
    underBothSettings([&] {
        writeCsv(0, 100);
        ok("BEGIN TRANSACTION;");
        insertLocals(1000, 20);
        EXPECT_EQ(cardinality(), 20);
        ok(copyStatement());
        EXPECT_EQ(cardinality(), 120) << "in the transaction, after the copy";
        insertLocals(2000, 5);
        EXPECT_EQ(cardinality(), 125) << "in the transaction, after more inserts";
        ok("COMMIT;");
        EXPECT_EQ(cardinality(), 125) << "after the commit";
    });
}

TEST_F(CopyStatisticsTest, InsertsThenACopyRolledBackLeaveNothing) {
    underBothSettings([&] {
        writeCsv(0, 100);
        ok("BEGIN TRANSACTION;");
        insertLocals(1000, 20);
        ok(copyStatement());
        insertLocals(2000, 5);
        ok("ROLLBACK;");
        EXPECT_EQ(cardinality(), 0);
        EXPECT_EQ(distinctNames(), 0);
    });
}

// La forme compacte des tableaux au journal : un tableau de taille fixe de numériques
// (FLOAT[N], DOUBLE[N], entiers) y est écrit en octets bruts, sous trois numéros
// d'enregistrement neufs (insertion, mise à jour d'un nœud, mise à jour d'une relation). Avant,
// chaque élément y était une valeur avec son type : un FLOAT[768] de 3 Ko pesait 9,2 Ko.
//
// La preuve est une parité au bit près : les mêmes instructions jouées dans une base de
// référence, qui ne passe pas par le journal, et dans une base dont le processus meurt base
// ouverte, relue par le rejeu — chaque élément comparé par ses octets.
class JournalRawArraysTest : public JournaledCopyTest {
protected:
    static constexpr int64_t DIMENSION = 768;
    static constexpr int64_t NUM_COPIED = 300;

    static std::string element(int64_t row, int64_t position) {
        // Des décimales que le flottant n'a pas : l'arrondi est celui de l'analyse du texte,
        // la même des deux côtés.
        return std::to_string((row * 7919 + position * 104729) % 200003 - 100001) + "." +
               std::to_string((row * 31 + position * 17) % 9973);
    }

    static std::string list(int64_t row, int64_t size, int64_t nullAt = -1) {
        std::string result = "[";
        for (int64_t i = 0; i < size; i++) {
            result += (i ? "," : "") + (i == nullAt ? std::string("NULL") : element(row, i));
        }
        return result + "]";
    }

    static std::string intList(int64_t row, int64_t size, int64_t nullAt = -1) {
        std::string result = "[";
        for (int64_t i = 0; i < size; i++) {
            result += (i ? "," : "") +
                      (i == nullAt ? std::string("NULL") : std::to_string(row * 1000 + i - 500));
        }
        return result + "]";
    }

    void writeVectorCsv() const {
        std::ofstream csv(csvPath);
        for (int64_t id = 1000; id < 1000 + NUM_COPIED; id++) {
            csv << id << ",\"" << list(id, DIMENSION) << "\",\"" << list(id + 1, 3) << "\",\""
                << intList(id, 5) << "\"\n";
        }
    }

    // Les mêmes écritures pour les deux bases : insertions, COPY, mises à jour, tableaux nuls,
    // éléments nuls, propriété de relation.
    void writes(const std::function<void(const std::string&)>& run) const {
        // Une seule transaction : ses lignes vont au journal dans un même enregistrement, où se
        // côtoient des tableaux pleins (octets bruts), un tableau nul (rien) et des tableaux à
        // élément nul (la forme d'avant).
        run("BEGIN TRANSACTION;");
        for (int64_t id = 0; id < 5; id++) {
            run("CREATE (:Doc {id: " + std::to_string(id) + ", v: " + list(id, DIMENSION) +
                ", w: " + list(id + 2, 3) + ", n: " + intList(id, 5) + "});");
        }
        run("CREATE (:Doc {id: 100});");
        run("CREATE (:Doc {id: 101, v: " + list(101, DIMENSION, 400) + ", w: " + list(101, 3, 0) +
            ", n: " + intList(101, 5, 4) + "});");
        // Les valeurs qu'une comparaison de flottants ne distingue pas : pas un nombre, le zéro
        // négatif, un dénormalisé, les infinis. La parité se juge sur leurs octets.
        run("CREATE (:Doc {id: 102, w: [CAST('NaN' AS DOUBLE), -0.0, 4.9e-324], n: " +
            intList(102, 5) + "});");
        run("CREATE (:Doc {id: 103, w: [CAST('Infinity' AS DOUBLE), CAST('-Infinity' AS DOUBLE), "
            "2.2250738585072014e-308], n: [2147483647, -2147483648, 0, -1, 1]});");
        run("COMMIT;");
        run("MATCH (d:Doc {id: 102}) SET d.w = [-0.0, CAST('NaN' AS DOUBLE), 0.0];");
        run("COPY Doc FROM '" + csvPath + "' (header=false);");
        run("MATCH (d:Doc {id: 3}) SET d.v = " + list(9003, DIMENSION) + ";");
        run("MATCH (d:Doc {id: 1007}) SET d.v = " + list(9007, DIMENSION) + ", d.n = " +
            intList(77, 5) + ";");
        run("MATCH (d:Doc {id: 1010}) SET d.w = " + list(9010, 3, 1) + ";");
        run("MATCH (d:Doc {id: 4}) SET d.v = NULL;");
        run("MATCH (d:Doc {id: 100}) SET d.w = " + list(9100, 3) + ";");
        run("MATCH (a:Doc {id: 1}), (b:Doc {id: 1001}) CREATE (a)-[:Link {p: " + list(1, 4) +
            "}]->(b);");
        run("MATCH (a:Doc {id: 2}), (b:Doc {id: 1002}) CREATE (a)-[:Link {p: " + list(2, 4) +
            "}]->(b);");
        run("MATCH (a:Doc {id: 2})-[l:Link]->(b:Doc) SET l.p = " + list(9202, 4) + ";");
    }

    static constexpr const char* SCHEMA_DOC =
        "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, v FLOAT[768], w DOUBLE[3], n INT32[5]);";
    static constexpr const char* SCHEMA_LINK = "CREATE REL TABLE Link(FROM Doc TO Doc, p FLOAT[4]);";

    // Chaque tableau de chaque ligne, élément par élément : nul des deux côtés ou les mêmes
    // octets. Rend le nombre d'éléments comparés ; les écarts sont comptés, pas listés.
    static uint64_t expectSameArrays(Connection& actual, Connection& reference,
        const std::string& query, const std::string& what) {
        auto left = actual.query(query);
        auto right = reference.query(query);
        EXPECT_TRUE(left->isSuccess()) << what << " : " << left->getErrorMessage();
        EXPECT_TRUE(right->isSuccess()) << what << " : " << right->getErrorMessage();
        if (!left->isSuccess() || !right->isSuccess()) {
            return 0;
        }
        EXPECT_EQ(left->getNumTuples(), right->getNumTuples()) << what;
        uint64_t numCompared = 0;
        uint64_t numDifferent = 0;
        while (left->hasNext() && right->hasNext()) {
            auto leftRow = left->getNext();
            auto rightRow = right->getNext();
            for (uint32_t column = 0; column < leftRow->len(); column++) {
                auto* a = leftRow->getValue(column);
                auto* b = rightRow->getValue(column);
                if (a->isNull() || b->isNull()) {
                    numDifferent += a->isNull() != b->isNull();
                    numCompared++;
                    continue;
                }
                if (a->getDataType().getLogicalTypeID() != rag3db::common::LogicalTypeID::ARRAY) {
                    numDifferent += a->toString() != b->toString();
                    numCompared++;
                    continue;
                }
                const auto size = rag3db::common::NestedVal::getChildrenSize(a);
                numDifferent += size != rag3db::common::NestedVal::getChildrenSize(b);
                for (uint32_t i = 0;
                     i < size && i < rag3db::common::NestedVal::getChildrenSize(b); i++) {
                    auto* x = rag3db::common::NestedVal::getChildVal(a, i);
                    auto* y = rag3db::common::NestedVal::getChildVal(b, i);
                    numCompared++;
                    if (x->isNull() || y->isNull()) {
                        numDifferent += x->isNull() != y->isNull();
                        continue;
                    }
                    switch (x->getDataType().getLogicalTypeID()) {
                    case rag3db::common::LogicalTypeID::FLOAT: {
                        const auto f = x->getValue<float>();
                        const auto g = y->getValue<float>();
                        numDifferent += memcmp(&f, &g, sizeof(f)) != 0;
                    } break;
                    case rag3db::common::LogicalTypeID::DOUBLE: {
                        const auto f = x->getValue<double>();
                        const auto g = y->getValue<double>();
                        numDifferent += memcmp(&f, &g, sizeof(f)) != 0;
                    } break;
                    default: {
                        numDifferent += x->getValue<int32_t>() != y->getValue<int32_t>();
                    } break;
                    }
                }
            }
        }
        EXPECT_EQ(numDifferent, 0u) << what << " : elements that differ, of " << numCompared;
        return numCompared;
    }

    static void expectSameDatabase(Connection& actual, Connection& reference,
        const std::string& what) {
        const auto numNodeElements = expectSameArrays(actual, reference,
            "MATCH (d:Doc) RETURN d.id, d.v, d.w, d.n ORDER BY d.id;", what + ", nodes");
        // 309 lignes ; un tableau nul compte pour un élément.
        EXPECT_GT(numNodeElements, 230000u) << what;
        const auto numRelElements = expectSameArrays(actual, reference,
            "MATCH (a:Doc)-[l:Link]->(b:Doc) RETURN a.id, b.id, l.p ORDER BY a.id;",
            what + ", relationships");
        EXPECT_EQ(numRelElements, 2u * (2 + 4)) << what;
    }
};

TEST_F(JournalRawArraysTest, ArraysAreReplayedBitForBitAndWeighWhatTheyHold) {
    writeVectorCsv();
    // La référence : les mêmes écritures, sans journal à rejouer.
    const auto referencePath = databasePath + ".reference";
    std::filesystem::remove(referencePath);
    auto referenceDatabase = std::make_unique<Database>(referencePath, *systemConfig);
    auto reference = std::make_unique<Connection>(referenceDatabase.get());
    const auto runReference = [&](const std::string& query) {
        auto result = reference->query(query);
        ASSERT_TRUE(result->isSuccess()) << query.substr(0, 120) << " : "
                                         << result->getErrorMessage();
    };
    runReference(SCHEMA_DOC);
    runReference(SCHEMA_LINK);
    writes(runReference);
    runReference("CHECKPOINT;");

    writeThenDie([&](Connection& child) {
        must(child, SCHEMA_DOC);
        must(child, SCHEMA_LINK);
        must(child, "CHECKPOINT;");
        writes([&](const std::string& query) { must(child, query); });
    });
    const auto journal = journalSize();
    ASSERT_GT(journal, 0u) << "nothing must have checkpointed";
    // Les 300 lignes du COPY, 5 insérées et 3 mises à jour portent 309 FLOAT[768] : 949 Ko
    // bruts. La forme d'avant en écrivait le triple.
    EXPECT_LT(journal, 1300u * 1024) << "the journal weighs " << journal << " bytes";
    EXPECT_GT(journal, 949u * 1024);

    // En lecture seule d'abord : le journal est rejoué en mémoire et reste où il est.
    {
        auto readOnlyConfig = *systemConfig;
        readOnlyConfig.readOnly = true;
        Database readOnlyDatabase(databasePath, readOnlyConfig);
        Connection readOnly(&readOnlyDatabase);
        expectSameDatabase(readOnly, *reference, "read-only open");
    }
    EXPECT_EQ(journalSize(), journal) << "a read-only open must leave the journal as it is";

    createDBAndConn();
    expectSameDatabase(*conn, *reference, "after the replay");
    // Et après un point de reprise et une réouverture.
    ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
    createDBAndConn();
    expectSameDatabase(*conn, *reference, "after a checkpoint and a reopening");

    reference.reset();
    referenceDatabase.reset();
    std::filesystem::remove(referencePath);
    std::filesystem::remove(rag3db::storage::StorageUtils::getWALFilePath(referencePath));
}

// Un journal écrit par le moteur d'avant la forme compacte (1177f5794) se rejoue : il porte les
// numéros d'enregistrement d'origine, relus par l'ancien décodage. La base et son journal sont
// au dépôt, avec le programme qui les a fabriqués (journal_before_raw_arrays/fabrique.cpp) :
// dix insertions, un tableau nul, un élément nul, un COPY journalisé de cinquante lignes, trois
// mises à jour ; le processus est mort base ouverte.
TEST_F(JournalRawArraysTest, AJournalWrittenBeforeTheRawFormIsStillReplayed) {
    const auto fixture = TestHelper::appendRag3dbRootPath(
        "test/transaction/journal_before_raw_arrays/base.rag3db");
    const auto walPath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    std::filesystem::remove(databasePath);
    std::filesystem::remove(walPath);
    std::filesystem::copy_file(fixture, databasePath);
    std::filesystem::copy_file(rag3db::storage::StorageUtils::getWALFilePath(fixture), walPath);
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 62);
    EXPECT_EQ(text("MATCH (d:Doc {id: 1049}) RETURN d.name;"), "copied 1049");
    // Une insertion, une ligne du COPY, les trois mises à jour, le tableau nul, l'élément nul.
    EXPECT_EQ(single("MATCH (d:Doc {id: 7}) WHERE d.v[1] = CAST(7.5 AS FLOAT) AND d.v[3] = "
                     "CAST(-1.0 AS FLOAT) AND d.v[4] = CAST(7.125 AS FLOAT) AND d.w[1] = "
                     "7.000001 AND d.w[2] = -2.5 AND d.n[1] = 7 AND d.n[3] = 100000 RETURN "
                     "count(*);"),
        1);
    EXPECT_EQ(single("MATCH (d:Doc {id: 1023}) WHERE d.v[1] = CAST(1023.5 AS FLOAT) AND d.v[4] "
                     "= CAST(7.75 AS FLOAT) AND d.w[1] = 1023.25 AND d.n[1] = 1023 AND d.n[3] = "
                     "3 RETURN count(*);"),
        1);
    EXPECT_EQ(single("MATCH (d:Doc {id: 3}) WHERE d.v[1] = CAST(9.5 AS FLOAT) AND d.v[2] = "
                     "CAST(8.25 AS FLOAT) AND d.v[3] = CAST(7.125 AS FLOAT) AND d.v[4] = "
                     "CAST(6.0 AS FLOAT) RETURN count(*);"),
        1);
    EXPECT_EQ(single("MATCH (d:Doc {id: 1001}) WHERE d.w[1] = 0.1 AND d.w[2] = 0.2 RETURN "
                     "count(*);"),
        1);
    EXPECT_EQ(single("MATCH (d:Doc {id: 4}) WHERE d.v IS NULL AND d.w[2] = -2.5 RETURN "
                     "count(*);"),
        1);
    EXPECT_EQ(single("MATCH (d:Doc {id: 100}) WHERE d.v IS NULL AND d.w IS NULL AND d.n IS NULL "
                     "RETURN count(*);"),
        1);
    EXPECT_EQ(single("MATCH (d:Doc {id: 101}) WHERE d.v[1] = CAST(1.5 AS FLOAT) AND "
                     "list_extract(d.v, 2) IS NULL AND d.v[3] = CAST(3.5 AS FLOAT) AND "
                     "list_extract(d.w, 1) IS NULL AND d.w[2] = 1.0 AND d.n[2] = 2 AND "
                     "list_extract(d.n, 3) IS NULL RETURN count(*);"),
        1);
}

// Le seuil du journal d'un COPY (étape 4, page 05) : le journal d'une transaction vit en
// mémoire jusqu'à sa validation. Quand un COPY journalisé le porte au-delà de
// copy_journal_threshold, la transaction se replie sur le point de reprise forcé — son journal
// est vidé, plus rien n'y est écrit, et elle est durable par son seul point de reprise, entière
// ou pas du tout. Sous le seuil, rien ne change : le COPY est durable par le journal.
class CopyJournalThresholdTest : public JournaledCopyTest {
protected:
    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    void openWithSchema() {
        createDBAndConn();
        ok("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING, score DOUBLE);");
        ok("CREATE REL TABLE Link(FROM Doc TO Doc, weight INT64);");
        ok("CHECKPOINT;");
        ok("CALL auto_checkpoint=false;");
        ok("CALL force_checkpoint_on_copy=false;");
    }

    // Les lignes id, name, score de [first, first + count), dans un fichier au choix.
    static void writeDocs(const std::string& path, int64_t first, int64_t count) {
        std::ofstream csv(path);
        for (int64_t id = first; id < first + count; id++) {
            csv << id << ",name " << id << "," << id << ".5\n";
        }
    }

    static std::string copyFrom(const std::string& path) {
        return "COPY Doc FROM '" + path + "';";
    }

    int64_t fallbacks() {
        auto result = conn->query("CALL current_setting('copy_journal_fallbacks') RETURN *;");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   std::stoll(result->getNext()->getValue(0)->toString()) :
                   -1;
    }

    // Ce que 3 000 lignes pèsent au journal, mesuré : les seuils des cas s'en déduisent.
    uint64_t journalOfThreeThousandRows() {
        openWithSchema();
        writeDocs(csvPath, 0, 3000);
        EXPECT_TRUE(conn->query(copyFrom(csvPath))->isSuccess());
        EXPECT_EQ(fallbacks(), 0);
        const auto size = journalSize();
        EXPECT_GT(size, 50000u);
        conn.reset();
        database.reset();
        std::filesystem::remove(databasePath);
        std::filesystem::remove(rag3db::storage::StorageUtils::getWALFilePath(databasePath));
        return size;
    }

    static constexpr const char* SCHEMA_AND_SETTINGS[] = {
        "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING, score DOUBLE);",
        "CREATE REL TABLE Link(FROM Doc TO Doc, weight INT64);", "CHECKPOINT;"};

    static void childSchema(Connection& child) {
        for (const auto* statement : SCHEMA_AND_SETTINGS) {
            must(child, statement);
        }
    }
};

TEST_F(CopyJournalThresholdTest, UnderTheThresholdACopyStaysInTheJournal) {
    const auto size = journalOfThreeThousandRows();
    writeDocs(csvPath, 0, 3000);
    writeThenDie([&](Connection& child) {
        childSchema(child);
        must(child, "CALL copy_journal_threshold=" + std::to_string(size * 2) + ";");
        must(child, copyFrom(csvPath));
    });
    ASSERT_GT(journalSize(), 0u) << "the copy must not have checkpointed";
    createDBAndConn();
    expectRows(0, 3000);
}

TEST_F(CopyJournalThresholdTest, AboveTheThresholdACopyFallsBackToItsCheckpoint) {
    const auto size = journalOfThreeThousandRows();
    openWithSchema();
    writeDocs(csvPath, 0, 3000);
    ok("CALL copy_journal_threshold=" + std::to_string(size / 3) + ";");
    ok(copyFrom(csvPath));
    EXPECT_EQ(fallbacks(), 1);
    EXPECT_EQ(journalSize(), 0u) << "a forced transaction writes nothing to the journal";
    expectRows(0, 3000);
    // Le suivant, plus petit, est de nouveau journalisé : le repli est celui d'une transaction.
    writeDocs(csvPath, 3000, 100);
    ok(copyFrom(csvPath));
    EXPECT_EQ(fallbacks(), 1);
    EXPECT_GT(journalSize(), 0u);
    expectRows(3000, 100);
}

// Mort juste après la validation d'un COPY replié : tout est là, par le point de reprise seul.
TEST_F(CopyJournalThresholdTest, AFallenBackCopySurvivesADeathAfterItsCommit) {
    const auto size = journalOfThreeThousandRows();
    writeDocs(csvPath, 0, 3000);
    writeThenDie([&](Connection& child) {
        childSchema(child);
        must(child, "CALL copy_journal_threshold=" + std::to_string(size / 3) + ";");
        must(child, copyFrom(csvPath));
    });
    EXPECT_EQ(journalSize(), 0u);
    createDBAndConn();
    expectRows(0, 3000);
}

// Le franchissement en cours de transaction : ce qu'elle avait déjà au journal avant le seuil —
// une insertion, une colonne ajoutée, un premier petit COPY — puis le gros.
class CopyJournalThresholdCrossedTest : public CopyJournalThresholdTest {
protected:
    // Dans le fils : la transaction jusqu'au gros COPY compris, sans la finir.
    void crossingTransaction(Connection& child, uint64_t size) const {
        childSchema(child);
        must(child, "CALL copy_journal_threshold=" + std::to_string(size / 2) + ";");
        must(child, "BEGIN TRANSACTION;");
        must(child, "CREATE (:Doc {id: 900000, name: 'created', score: 1.5});");
        must(child, "ALTER TABLE Doc ADD extra INT64 DEFAULT 7;");
        must(child, "COPY Doc(id, name, score) FROM '" + relCsvPath + "';");
        must(child, "MATCH (d:Doc {id: 100005}) SET d.name = 'changed';");
        must(child, "COPY Doc(id, name, score) FROM '" + csvPath + "';");
        must(child, "CREATE (:Doc {id: 900001, name: 'created after', score: 2.5});");
    }

    void writeBothCsv() const {
        writeDocs(relCsvPath, 100000, 20); // le petit
        writeDocs(csvPath, 0, 3000);       // le gros
    }

    void expectEverything() {
        EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 3022);
        expectRows(0, 3000);
        EXPECT_EQ(text("MATCH (d:Doc {id: 900000}) RETURN d.name;"), "created");
        EXPECT_EQ(text("MATCH (d:Doc {id: 900001}) RETURN d.name;"), "created after");
        EXPECT_EQ(text("MATCH (d:Doc {id: 100005}) RETURN d.name;"), "changed");
        EXPECT_EQ(text("MATCH (d:Doc {id: 100019}) RETURN d.name;"), "name 100019");
        EXPECT_EQ(single("MATCH (d:Doc) WHERE d.extra = 7 RETURN count(*);"), 3022);
    }

    void expectNothing() {
        EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 0);
        auto extra = conn->query("MATCH (d:Doc) RETURN d.extra;");
        EXPECT_FALSE(extra->isSuccess()) << "the added column must not have come back";
    }
};

TEST_F(CopyJournalThresholdCrossedTest, CommittedThenDeadEverythingIsThereByTheCheckpointAlone) {
    const auto size = journalOfThreeThousandRows();
    writeBothCsv();
    writeThenDie([&](Connection& child) {
        crossingTransaction(child, size);
        must(child, "COMMIT;");
    });
    EXPECT_EQ(journalSize(), 0u);
    createDBAndConn();
    expectEverything();
    createDBAndConn();
    expectEverything();
}

TEST_F(CopyJournalThresholdCrossedTest, DeadBeforeTheCommitNothingComesBackNotEvenHalf) {
    const auto size = journalOfThreeThousandRows();
    writeBothCsv();
    writeThenDie([&](Connection& child) { crossingTransaction(child, size); });
    createDBAndConn();
    expectNothing();
    // Et la base sert : la même transaction, cette fois validée.
    ASSERT_TRUE(conn->query("CALL force_checkpoint_on_copy=false;")->isSuccess());
    ok("BEGIN TRANSACTION;");
    ok("CREATE (:Doc {id: 900000, name: 'created', score: 1.5});");
    ok("COPY Doc FROM '" + csvPath + "';");
    ok("COMMIT;");
    expectRows(0, 3000);
}

TEST_F(CopyJournalThresholdCrossedTest, RolledBackNothingIsLeftAndTheSessionGoesOn) {
    const auto size = journalOfThreeThousandRows();
    writeBothCsv();
    openWithSchema();
    ok("CALL copy_journal_threshold=" + std::to_string(size / 2) + ";");
    ok("BEGIN TRANSACTION;");
    ok("CREATE (:Doc {id: 900000, name: 'created', score: 1.5});");
    ok("ALTER TABLE Doc ADD extra INT64 DEFAULT 7;");
    ok("COPY Doc(id, name, score) FROM '" + relCsvPath + "';");
    EXPECT_EQ(fallbacks(), 0) << "the small copy stays under the threshold";
    ok("COPY Doc(id, name, score) FROM '" + csvPath + "';");
    EXPECT_EQ(fallbacks(), 1);
    ok("ROLLBACK;");
    expectNothing();
    EXPECT_EQ(journalSize(), 0u);
    // La session continue, et un COPY sous le seuil y est de nouveau journalisé.
    ok(copyFrom(relCsvPath));
    EXPECT_EQ(fallbacks(), 1);
    EXPECT_GT(journalSize(), 0u);
    createDBAndConn();
    EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 20);
}

// Le seuil se compte sur la transaction : deux COPY chacun dessous, ensemble dessus.
TEST_F(CopyJournalThresholdTest, TwoCopiesUnderTheThresholdTogetherAboveIt) {
    const auto size = journalOfThreeThousandRows();
    writeDocs(csvPath, 0, 3000);
    writeDocs(relCsvPath, 3000, 3000);
    openWithSchema();
    ok("CALL copy_journal_threshold=" + std::to_string(size * 3 / 2) + ";");
    ok("BEGIN TRANSACTION;");
    ok(copyFrom(csvPath));
    EXPECT_EQ(fallbacks(), 0);
    ok(copyFrom(relCsvPath));
    EXPECT_EQ(fallbacks(), 1);
    ok("COMMIT;");
    EXPECT_EQ(journalSize(), 0u);
    expectRows(0, 6000);
    createDBAndConn();
    expectRows(0, 6000);
}

// Un COPY de relations qui franchit le seuil au milieu de ses paquets.
TEST_F(CopyJournalThresholdTest, ARelationCopyCrossingTheThresholdMidway) {
    writeDocs(csvPath, 0, 30001);
    writeRelCsv(0, 30000, 1);
    openWithSchema();
    ok("CALL force_checkpoint_on_copy=true;");
    ok(copyFrom(csvPath));
    ok("CALL force_checkpoint_on_copy=false;");
    // 30 000 relations pèsent plus de 2 Mo au journal : le seuil tombe dans le COPY.
    ok("CALL copy_journal_threshold=400000;");
    ok(relCopyStatement());
    EXPECT_EQ(fallbacks(), 1);
    EXPECT_EQ(journalSize(), 0u);
    expectRels(30000, 1);
    createDBAndConn();
    expectRels(30000, 1);
}

TEST_F(CopyJournalThresholdTest, AFallenBackRelationCopyDiesAfterAndBeforeItsCommit) {
    writeDocs(csvPath, 0, 30001);
    writeRelCsv(0, 30000, 1);
    writeThenDie([&](Connection& child) {
        childSchema(child);
        must(child, "CALL force_checkpoint_on_copy=true;");
        must(child, copyFrom(csvPath));
        must(child, "CALL force_checkpoint_on_copy=false;");
        must(child, "CALL copy_journal_threshold=400000;");
        must(child, "BEGIN TRANSACTION;");
        must(child, relCopyStatement());
    });
    createDBAndConn();
    EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 30001);
    EXPECT_EQ(single("MATCH (:Doc)-[l:Link]->(:Doc) RETURN count(l);"), 0);
    conn.reset();
    database.reset();
    writeThenDie([&](Connection& child) {
        must(child, "CALL copy_journal_threshold=400000;");
        must(child, relCopyStatement());
    });
    EXPECT_EQ(journalSize(), 0u);
    createDBAndConn();
    expectRels(30000, 1);
}

// Une transaction forcée — durable par son seul point de reprise : un COPY sous le réglage
// d'aujourd'hui, un COPY qui écarte des lignes, la création d'un index — n'écrit rien au fichier
// du journal (37608cf4b). Elle tenait pourtant son journal en mémoire jusqu'à sa validation,
// pour le jeter : ses écritures ordinaires y étaient sérialisées pour rien, en mémoire que le
// tampon ne peut pas évincer. Dès qu'elle devient forcée, son journal est vidé et plus rien n'y
// est écrit.
class ForcedTransactionJournalTest : public CopyJournalThresholdTest {
protected:
    // Comme aujourd'hui : un COPY force son point de reprise.
    void openForced() {
        createDBAndConn();
        ok("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING, score DOUBLE);");
        ok("CREATE REL TABLE Link(FROM Doc TO Doc, weight INT64);");
        ok("UNWIND range(500000, 500199) AS i CREATE (:Doc {id: i, name: 'old', score: 0.5});");
        ok("CHECKPOINT;");
        ok("CALL auto_checkpoint=false;");
    }

    uint64_t journalInMemory() {
        return rag3db::transaction::Transaction::Get(*conn->getClientContext())
            ->getLocalWAL()
            .getSize();
    }

    // Deux cents mises à jour d'une chaîne de 2 000 caractères : 400 Ko de journal.
    void bigUpdates(const std::string& letter) {
        ok("MATCH (d:Doc) WHERE d.id >= 500000 SET d.name = repeat('" + letter + "', 2000);");
    }

    static void childForcedSchema(Connection& child) {
        must(child, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING, score DOUBLE);");
        must(child, "CREATE REL TABLE Link(FROM Doc TO Doc, weight INT64);");
        must(child, "UNWIND range(500000, 500199) AS i CREATE (:Doc {id: i, name: 'old', "
                    "score: 0.5});");
        must(child, "CHECKPOINT;");
    }

    // Des écritures ordinaires avant le COPY forcé, et après lui.
    static void mixedTransaction(Connection& child, const std::string& copy) {
        must(child, "BEGIN TRANSACTION;");
        must(child, "CREATE (:Doc {id: 900000, name: 'created', score: 1.5});");
        must(child, "MATCH (d:Doc) WHERE d.id >= 500000 AND d.id < 500200 SET d.name = "
                    "repeat('x', 2000);");
        must(child, "MATCH (d:Doc {id: 500007}) DELETE d;");
        must(child, copy);
        must(child, "CREATE (:Doc {id: 900001, name: 'created after', score: 2.5});");
        must(child, "MATCH (a:Doc {id: 900000}), (b:Doc {id: 12}) CREATE (a)-[:Link {weight: "
                    "9}]->(b);");
    }

    void expectTheMixedTransaction() {
        EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 3000 + 199 + 2);
        expectRows(0, 3000);
        EXPECT_EQ(text("MATCH (d:Doc {id: 900000}) RETURN d.name;"), "created");
        EXPECT_EQ(text("MATCH (d:Doc {id: 900001}) RETURN d.name;"), "created after");
        EXPECT_EQ(single("MATCH (d:Doc) WHERE d.id >= 500000 AND d.id < 500200 AND size(d.name) "
                         "= 2000 RETURN count(*);"),
            199);
        EXPECT_EQ(single("MATCH (d:Doc {id: 500007}) RETURN count(*);"), 0);
        EXPECT_EQ(single("MATCH (a:Doc {id: 900000})-[l:Link]->(b:Doc) RETURN b.id * 100 + "
                         "l.weight;"),
            1209);
    }

    void expectNothingOfIt() {
        EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 200);
        EXPECT_EQ(single("MATCH (d:Doc) WHERE d.name = 'old' RETURN count(*);"), 200);
        EXPECT_EQ(single("MATCH (:Doc)-[l:Link]->(:Doc) RETURN count(l);"), 0);
    }
};

// La mémoire : ce que la transaction avait au journal avant de devenir forcée est rendu, et ce
// qu'elle écrit ensuite n'y va pas.
TEST_F(ForcedTransactionJournalTest, AForcedTransactionKeepsNoJournalInMemory) {
    openForced();
    writeDocs(csvPath, 0, 3000);
    ok("BEGIN TRANSACTION;");
    bigUpdates("a");
    EXPECT_GT(journalInMemory(), 400000u) << "not forced yet: its updates are in its journal";
    ok(copyFrom(csvPath));
    EXPECT_EQ(journalInMemory(), 0u) << "forced by its copy: the journal is given back";
    bigUpdates("b");
    ok("CREATE (:Doc {id: 900000, name: 'created', score: 1.5});");
    EXPECT_EQ(journalInMemory(), 0u) << "and nothing more is written to it";
    ok("COMMIT;");
    EXPECT_EQ(journalSize(), 0u);
    EXPECT_EQ(fallbacks(), 0) << "this is not a fallback of a journaled copy";
    expectRows(0, 3000);
    EXPECT_EQ(single("MATCH (d:Doc) WHERE d.id >= 500000 AND d.id < 500200 AND d.name = "
                     "repeat('b', 2000) RETURN count(*);"),
        200);
    createDBAndConn();
    expectRows(0, 3000);
    EXPECT_EQ(text("MATCH (d:Doc {id: 900000}) RETURN d.name;"), "created");
    EXPECT_EQ(single("MATCH (d:Doc) WHERE d.id >= 500000 AND d.id < 500200 AND d.name = "
                     "repeat('b', 2000) RETURN count(*);"),
        200);
}

TEST_F(ForcedTransactionJournalTest, OrdinaryWritesBeforeAForcedCopyCommittedThenDead) {
    writeDocs(csvPath, 0, 3000);
    writeThenDie(
        [&](Connection& child) {
            childForcedSchema(child);
            mixedTransaction(child, copyFrom(csvPath));
            must(child, "COMMIT;");
        },
        false /* journaled */);
    EXPECT_EQ(journalSize(), 0u);
    createDBAndConn();
    expectTheMixedTransaction();
}

TEST_F(ForcedTransactionJournalTest, OrdinaryWritesBeforeAForcedCopyDeadBeforeTheCommit) {
    writeDocs(csvPath, 0, 3000);
    writeThenDie(
        [&](Connection& child) {
            childForcedSchema(child);
            mixedTransaction(child, copyFrom(csvPath));
        },
        false /* journaled */);
    createDBAndConn();
    expectNothingOfIt();
}

// Annulée, rien n'en reste ; et la transaction suivante, ordinaire, écrit de nouveau au journal.
TEST_F(ForcedTransactionJournalTest, RolledBackThenTheNextTransactionIsJournaledAgain) {
    openForced();
    writeDocs(csvPath, 0, 3000);
    ok("BEGIN TRANSACTION;");
    bigUpdates("a");
    ok(copyFrom(csvPath));
    ok("CREATE (:Doc {id: 900000, name: 'created', score: 1.5});");
    ok("ROLLBACK;");
    expectNothingOfIt();
    EXPECT_EQ(journalSize(), 0u);
    ok("BEGIN TRANSACTION;");
    bigUpdates("c");
    EXPECT_GT(journalInMemory(), 400000u);
    ok("COMMIT;");
    EXPECT_GT(journalSize(), 400000u);
    conn.reset();
    database.reset();
    // Et ce journal se rejoue : le fils meurt sans point de reprise après une transaction
    // forcée annulée puis une ordinaire validée.
    std::filesystem::remove(databasePath);
    std::filesystem::remove(rag3db::storage::StorageUtils::getWALFilePath(databasePath));
    writeThenDie(
        [&](Connection& child) {
            childForcedSchema(child);
            mixedTransaction(child, copyFrom(csvPath));
            must(child, "ROLLBACK;");
            must(child, "MATCH (d:Doc) WHERE d.id >= 500000 SET d.name = 'again';");
        },
        false /* journaled */);
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 200);
    EXPECT_EQ(single("MATCH (d:Doc) WHERE d.name = 'again' RETURN count(*);"), 200);
}

// Le COPY qui écarte des lignes reste forcé sous le réglage du chargement journalisé.
TEST_F(ForcedTransactionJournalTest, OrdinaryWritesBeforeACopyThatSkipsRowsCommittedThenDead) {
    {
        std::ofstream csv(csvPath);
        for (int64_t id = 0; id < 3000; id++) {
            csv << id << ",name " << id << "," << id << ".5\n";
        }
        csv << "17,again,0.5\n";
    }
    writeThenDie([&](Connection& child) {
        childForcedSchema(child);
        mixedTransaction(child, "COPY Doc FROM '" + csvPath + "' (IGNORE_ERRORS=true);");
        must(child, "COMMIT;");
    });
    EXPECT_EQ(journalSize(), 0u);
    createDBAndConn();
    expectTheMixedTransaction();
}

// Les pages sans propriétaire après une réouverture (page coeur-cpp/06) : ce qu'un travail qui
// n'a pas atteint de point de reprise a ajouté à la fin du fichier y reste, compté et à
// personne — le fichier n'a pas d'étendue connue. La mesure est celle du contrôle de fuite de
// la suite Cypher : toutes les tables retirées, un point de reprise, puis les pages du fichier
// moins les pages libres, comparées à celles d'une base qui a fait le même travail sans mourir.
class OwnerlessPagesTest : public CopyJournalThresholdTest {
protected:
    // Plus qu'un groupe de nœuds : un COPY n'écrit ses pages avant la validation que par groupe
    // plein ; en deçà, tout reste en mémoire jusqu'au point de reprise, et rien ne fuit.
    static constexpr int64_t NUM_ROWS = 200000;

    // La base rouverte en écriture, puis le contrôle de fuite de la suite Cypher : toutes les
    // tables retirées, un point de reprise, et les pages occupées sont celles de l'en-tête, du
    // catalogue et des métadonnées. Rouge avant le correctif : 640 pages occupées pour 9.
    void expectNoOwnerlessPagesAfterReopening() {
        createDBAndConn();
        ASSERT_TRUE(conn->query("MATCH (d:Doc) RETURN count(*);")->isSuccess());
        FSMLeakChecker::checkForLeakedPages(conn.get());
    }
};

TEST_F(OwnerlessPagesTest, AJournaledCopyReplayedLeavesNoPageBehind) {
    writeDocs(csvPath, 0, NUM_ROWS);
    writeThenDie([&](Connection& child) {
        childSchema(child);
        must(child, "CALL copy_journal_threshold=1073741824;");
        must(child, copyFrom(csvPath));
    });
    ASSERT_GT(journalSize(), 0u);
    expectNoOwnerlessPagesAfterReopening();
}

TEST_F(OwnerlessPagesTest, AForcedCopyKilledBeforeItsCommitLeavesNoPageBehind) {
    writeDocs(csvPath, 0, NUM_ROWS);
    writeThenDie(
        [&](Connection& child) {
            childSchema(child);
            must(child, "BEGIN TRANSACTION;");
            must(child, copyFrom(csvPath));
        },
        false /* journaled */);
    expectNoOwnerlessPagesAfterReopening();
}

TEST_F(OwnerlessPagesTest, AFallenBackCopyKilledBeforeItsCommitLeavesNoPageBehind) {
    writeDocs(csvPath, 0, NUM_ROWS);
    writeThenDie([&](Connection& child) {
        childSchema(child);
        must(child, "CALL copy_journal_threshold=65536;");
        must(child, "BEGIN TRANSACTION;");
        must(child, copyFrom(csvPath));
    });
    expectNoOwnerlessPagesAfterReopening();
}

// Deux morts de suite sans point de reprise entre elles : la fuite ne s'additionne pas.
TEST_F(OwnerlessPagesTest, TwoDeathsInARowDoNotAddUp) {
    writeDocs(csvPath, 0, NUM_ROWS);
    writeThenDie(
        [&](Connection& child) {
            childSchema(child);
            must(child, "BEGIN TRANSACTION;");
            must(child, copyFrom(csvPath));
        },
        false /* journaled */);
    writeThenDie(
        [&](Connection& child) {
            must(child, "BEGIN TRANSACTION;");
            must(child, copyFrom(csvPath));
        },
        false /* journaled */);
    expectNoOwnerlessPagesAfterReopening();
}

// Entre la mort et la réouverture en écriture, une ouverture en lecture seule : elle ne rend
// rien, n'écrit rien, et lit juste ; c'est l'ouverture en écriture d'après qui rend l'excédent.
TEST_F(OwnerlessPagesTest, AReadOnlyOpenInBetweenChangesNothing) {
    writeDocs(csvPath, 0, NUM_ROWS);
    writeThenDie([&](Connection& child) {
        childSchema(child);
        must(child, "CALL copy_journal_threshold=1073741824;");
        must(child, copyFrom(csvPath));
    });
    const auto walPath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    const auto fileSizeBefore = std::filesystem::file_size(databasePath);
    const auto journalBefore = std::filesystem::file_size(walPath);
    {
        auto readOnlyConfig = *systemConfig;
        readOnlyConfig.readOnly = true;
        Database readOnlyDatabase(databasePath, readOnlyConfig);
        Connection readOnly(&readOnlyDatabase);
        auto count = readOnly.query("MATCH (d:Doc) RETURN count(*);");
        ASSERT_TRUE(count->isSuccess()) << count->getErrorMessage();
        EXPECT_EQ(count->getNext()->getValue(0)->getValue<int64_t>(), NUM_ROWS);
        auto freePages = readOnly.query("CALL fsm_info() RETURN sum(num_pages);");
        ASSERT_TRUE(freePages->isSuccess());
        auto* numFree = freePages->getNext()->getValue(0);
        EXPECT_TRUE(numFree->isNull() || numFree->toString() == "0")
            << "a read-only open must not give back the excess: " << numFree->toString();
    }
    EXPECT_EQ(std::filesystem::file_size(databasePath), fileSizeBefore);
    EXPECT_EQ(std::filesystem::file_size(walPath), journalBefore);
    expectNoOwnerlessPagesAfterReopening();
}

// Le rejeu ne lit rien au-delà de l'étendue : les lignes d'un COPY journalisé reviennent par
// leur valeur, depuis le journal. Preuve par la réutilisation : l'excédent rendu à l'ouverture
// est réécrit par le premier point de reprise, qui y range les lignes rejouées ; chaque ligne est
// ensuite comparée à celle d'une base témoin qui n'est pas morte — par sa clé, son texte et ses
// nombres, sommés et échantillonnés.
TEST_F(OwnerlessPagesTest, TheReplayReadsNothingBeyondTheExtent) {
    const auto rowsCsv = csvPath;
    {
        std::ofstream csv(rowsCsv);
        for (int64_t id = 0; id < NUM_ROWS; id++) {
            csv << id << ",name " << id << "," << id << ".5,\"[" << id % 97 << "." << id % 7
                << "," << id % 89 << ".25," << -(id % 11) << ".0," << id % 13 << ".5]\"\n";
        }
    }
    const auto schema = "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING, score DOUBLE, "
                        "vec FLOAT[4]);";
    const auto digest = [](Connection& connection) {
        auto result = connection.query(
            "MATCH (d:Doc) RETURN count(*), sum(d.id), sum(size(d.name)), sum(d.score), "
            "sum(d.vec[1] * 3.0 + d.vec[2] * 5.0 + d.vec[3] * 7.0 + d.vec[4] * 11.0), "
            "min(d.name), max(d.name);");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        return result->isSuccess() ? result->getNext()->toString() : std::string("<error>");
    };
    const auto sample = [](Connection& connection, int64_t id) {
        auto result = connection.query("MATCH (d:Doc {id: " + std::to_string(id) +
                                       "}) RETURN d.name, d.score, d.vec;");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ? result->getNext()->toString() :
                                                          std::string("<none>");
    };
    // La base témoin, qui ne meurt pas.
    const auto referencePath = databasePath + ".reference";
    std::filesystem::remove(referencePath);
    std::filesystem::remove(rag3db::storage::StorageUtils::getWALFilePath(referencePath));
    Database referenceDatabase(referencePath, *systemConfig);
    Connection reference(&referenceDatabase);
    ASSERT_TRUE(reference.query(schema)->isSuccess());
    ASSERT_TRUE(reference.query("COPY Doc FROM '" + rowsCsv + "';")->isSuccess());
    ASSERT_TRUE(reference.query("CHECKPOINT;")->isSuccess());
    const auto referenceDigest = digest(reference);

    writeThenDie([&](Connection& child) {
        must(child, schema);
        must(child, "CHECKPOINT;");
        must(child, "CALL copy_journal_threshold=1073741824;");
        must(child, "COPY Doc FROM '" + rowsCsv + "';");
    });
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    // Le point de reprise réécrit l'excédent rendu : ce que le rejeu aurait lu là est écrasé.
    ok("CHECKPOINT;");
    EXPECT_EQ(digest(*conn), referenceDigest) << "after the replay and a checkpoint";
    for (const int64_t id : {0, 1, 131071, 131072, 131073, 150000, 199999}) {
        EXPECT_EQ(sample(*conn, id), sample(reference, id)) << id;
    }
    createDBAndConn();
    EXPECT_EQ(digest(*conn), referenceDigest) << "after a reopening";
    std::filesystem::remove(referencePath);
    std::filesystem::remove(rag3db::storage::StorageUtils::getWALFilePath(referencePath));
}

// Une base écrite avant la version 40 du stockage, dont l'en-tête n'a pas d'étendue : elle
// s'ouvre, rien n'est rendu (l'étendue est inconnue), son contenu est intact, et son premier
// point de reprise lui donne une étendue ; une mort après est alors rattrapée comme pour les
// autres. Fabriquée par un moteur d'avant (test/transaction/database_before_extent/fabrique.cpp).
TEST_F(OwnerlessPagesTest, ADatabaseFromBeforeTheExtentOpensAndAcquiresIt) {
    // Gardée compressée (2,9 Mo de pages presque vides, 25 Ko compressés) ; gzip et un shell.
    const auto fixture = TestHelper::appendRag3dbRootPath(
        "test/transaction/database_before_extent/base.rag3db.gz");
    std::filesystem::remove(databasePath);
    std::filesystem::remove(rag3db::storage::StorageUtils::getWALFilePath(databasePath));
    ASSERT_EQ(std::system(("gzip -dc '" + fixture + "' > '" + databasePath + "'").c_str()), 0);
    ASSERT_EQ(std::filesystem::file_size(databasePath), 2912256u);
    createDBAndConn();
    EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 501);
    EXPECT_EQ(text("MATCH (d:Doc {id: 1000}) RETURN d.name;"), "created");
    EXPECT_EQ(text("MATCH (d:Doc {id: 499}) RETURN d.name;"), "name 499");
    auto freePages = conn->query("CALL fsm_info() RETURN sum(num_pages);");
    ASSERT_TRUE(freePages->isSuccess());
    EXPECT_TRUE(freePages->getNext()->getValue(0)->isNull())
        << "an unknown extent gives nothing back";
    ok("CREATE (:Doc {id: 1001, name: 'by the new engine'});");
    ok("CHECKPOINT;");
    createDBAndConn();
    EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 502);
    EXPECT_EQ(text("MATCH (d:Doc {id: 1001}) RETURN d.name;"), "by the new engine");
    // Depuis ce point de reprise l'étendue est connue : une mort pendant un gros COPY est
    // rattrapée.
    writeDocs(csvPath, 2000, NUM_ROWS);
    const auto occupiedBefore = [&] {
        auto fileInfo = conn->query("CALL file_info() RETURN num_pages;");
        return fileInfo->getNext()->getValue(0)->getValue<int64_t>();
    }();
    conn.reset();
    database.reset();
    writeThenDie(
        [&](Connection& child) {
            must(child, "BEGIN TRANSACTION;");
            must(child, copyFrom(csvPath));
        },
        false /* journaled */);
    createDBAndConn();
    ok("CHECKPOINT;");
    auto fileInfo = conn->query("CALL file_info() RETURN num_pages;");
    EXPECT_LE(fileInfo->getNext()->getValue(0)->getValue<int64_t>(), occupiedBefore + 2)
        << "the pages of the killed copy must have been given back";
    EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 502);
}

// La même preuve avec un index vectoriel sur la table : l'index est créé avant (il force son
// point de reprise), le COPY journalisé le remplit, la mort, le rejeu, le point de reprise qui
// réécrit l'excédent ; puis la recherche rend la bonne ligne pour des vecteurs choisis. Sauté si
// l'extension vector n'est pas bâtie.
TEST_F(OwnerlessPagesTest, TheReplayReadsNothingBeyondTheExtentWithAVectorIndex) {
    const auto extension =
        TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
    if (!std::filesystem::exists(extension)) {
        GTEST_SKIP() << "the vector extension is not built";
    }
    constexpr int64_t numRows = 140000;
    {
        std::ofstream csv(csvPath);
        for (int64_t id = 0; id < numRows; id++) {
            csv << id << ",name " << id << ",\"[" << id << ".0,0.0,0.0,0.0]\"\n";
        }
    }
    writeThenDie([&](Connection& child) {
        must(child, "LOAD EXTENSION '" + extension + "';");
        must(child, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING, vec FLOAT[4]);");
        must(child, "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
        must(child, "CALL copy_journal_threshold=1073741824;");
        must(child, "COPY Doc FROM '" + csvPath + "';");
    });
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    ok("CHECKPOINT;");
    ok("LOAD EXTENSION '" + extension + "';");
    EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), numRows);
    for (const auto id : {0, 77, 131071, 131072, 139999}) {
        EXPECT_EQ(single("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', CAST([" +
                         std::to_string(id) + ".0, 0.0, 0.0, 0.0] AS FLOAT[4]), 1) RETURN "
                         "node.id;"),
            id)
            << id;
        EXPECT_EQ(text("MATCH (d:Doc {id: " + std::to_string(id) + "}) RETURN d.name;"),
            "name " + std::to_string(id));
    }
    createDBAndConn();
    ok("LOAD EXTENSION '" + extension + "';");
    EXPECT_EQ(single("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', CAST([131072.0, 0.0, 0.0, "
                     "0.0] AS FLOAT[4]), 1) RETURN node.id;"),
        131072);
}

// La même preuve avec un index de plein texte : créé avant le COPY (il force son point de
// reprise), rempli par le COPY journalisé ; après la mort, le rejeu, et le point de reprise qui
// réécrit l'excédent, la recherche d'un mot propre à une ligne rend cette ligne. Sauté si
// l'extension fts n'est pas bâtie.
TEST_F(OwnerlessPagesTest, TheReplayReadsNothingBeyondTheExtentWithAFullTextIndex) {
    const auto extension =
        TestHelper::appendRag3dbRootPath("extension/fts/build/libfts.rag3db_extension");
    if (!std::filesystem::exists(extension)) {
        GTEST_SKIP() << "the fts extension is not built";
    }
    constexpr int64_t numRows = 150000;
    {
        std::ofstream csv(csvPath);
        for (int64_t id = 0; id < numRows; id++) {
            csv << id << ",row " << id << " says word" << id << " and common\\n";
        }
    }
    writeThenDie([&](Connection& child) {
        must(child, "LOAD EXTENSION '" + extension + "';");
        must(child, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, body STRING);");
        must(child, "CALL CREATE_FTS_INDEX('Doc', 'doc_fts', ['body']);");
        must(child, "CALL copy_journal_threshold=1073741824;");
        must(child, "COPY Doc FROM '" + csvPath + "';");
    });
    ASSERT_GT(journalSize(), 0u);
    createDBAndConn();
    ok("CHECKPOINT;");
    ok("LOAD EXTENSION '" + extension + "';");
    EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), numRows);
    for (const auto id : {0, 77, 131071, 131072, 149999}) {
        EXPECT_EQ(single("CALL QUERY_FTS_INDEX('Doc', 'doc_fts', 'word" + std::to_string(id) +
                         "') RETURN node.id;"),
            id)
            << id;
    }
    EXPECT_EQ(single("CALL QUERY_FTS_INDEX('Doc', 'doc_fts', 'common') RETURN count(*);"),
        numRows);
}

} // namespace

#endif
