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

#include <filesystem>
#include <fstream>
#include <functional>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/table/node_table.h"
#include "storage/storage_utils.h"

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

// Défaut d'origine trouvé en écrivant le cas précédent : dans une transaction, des insertions
// ordinaires puis un COPY dans la même table. Les lignes locales portent des décalages
// provisoires qui commencent au nombre de lignes de la table ; le COPY écrit les siennes aux
// mêmes décalages. Ensuite, par la clé d'une ligne du COPY, on atteignait une ligne locale :
// avec vingt lignes insérées puis cent copiées, MATCH (n {id: 5}) rendait la ligne 1005, un SET
// par la clé 5 modifiait la ligne 1005, une relation de 7 vers 8 était attachée à 1007 et
// 1008 — validés en silence ; avec moins de lignes locales, un plantage (NodeGroup::lookup).
// Refusé par son nom tant que le cas n'est pas rendu juste ; l'ordre inverse est juste, et le
// refus ne laisse rien.
TEST_F(JournaledCopyTest, ACopyAfterUncommittedInsertsInTheSameTableIsRefusedByName) {
    createDBAndConn();
    ASSERT_TRUE(conn->query("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING, score "
                            "DOUBLE);")
                    ->isSuccess());
    writeCsv(0, 300);
    for (const auto* setting : {"CALL force_checkpoint_on_copy=true;",
             "CALL force_checkpoint_on_copy=false;"}) {
        ASSERT_TRUE(conn->query(setting)->isSuccess());
        ASSERT_TRUE(conn->query("BEGIN TRANSACTION;")->isSuccess());
        ASSERT_TRUE(conn->query("CREATE (:Doc {id: 100000, name: 'before', score: 0.0});")
                        ->isSuccess());
        auto refused = conn->query(copyStatement());
        ASSERT_FALSE(refused->isSuccess());
        EXPECT_NE(refused->getErrorMessage().find(
                      rag3db::storage::NodeTable::COPY_AFTER_UNCOMMITTED_INSERTS),
            std::string::npos)
            << refused->getErrorMessage();
        ASSERT_TRUE(conn->query("ROLLBACK;")->isSuccess());
        EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 0);
    }
    // L'ordre inverse : le COPY d'abord, puis l'insertion, puis les recherches par clé.
    ASSERT_TRUE(conn->query("BEGIN TRANSACTION;")->isSuccess());
    ASSERT_TRUE(conn->query(copyStatement())->isSuccess());
    ASSERT_TRUE(
        conn->query("CREATE (:Doc {id: 100000, name: 'after', score: 0.0});")->isSuccess());
    EXPECT_EQ(text("MATCH (n:Doc {id: 10}) RETURN n.name;"), "name 10");
    EXPECT_EQ(text("MATCH (n:Doc {id: 100000}) RETURN n.name;"), "after");
    ASSERT_TRUE(conn->query("COMMIT;")->isSuccess());
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), 301);
    EXPECT_EQ(text("MATCH (n:Doc {id: 100000}) RETURN n.name;"), "after");
    expectRows(0, 300);
}

} // namespace

#endif
