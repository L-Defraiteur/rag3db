// Marche « index vectoriel juste en service » : un SET d'un vecteur vers un autre perd des
// lignes dans l'index HNSW (session cœur C++, 3 octobre au soir). Un seul écrivain, une
// seule connexion, ni arrêt ni rejeu : ces témoins sont hors des verrous.
//
// L'attendu, partout : toute ligne qui porte un vecteur est joignable par une recherche
// exhaustive (k = efs = nombre de lignes), et se retrouve première — à égalité de
// distance près — quand on cherche son propre vecteur. Une recherche ne rend aucune ligne
// supprimée.
//
// Le compte des pertes n'est pas stable d'une passe à l'autre (tirage des niveaux du
// graphe) : un seul nombre ne prouve rien. Chaque cas se joue donc plusieurs fois, sur
// une table neuve à chaque fois, et rougit si un seul essai perd une ligne ; l'étiquette
// vector-index-exact couvre les deux contrôles, les comptes sont dans le message.
//
// Recette du cœur C++ : vecteurs d'origine [i % 17, i % 23, i % 29, i], index l2 bâti
// après le remplissage, puis MATCH … WHERE id dans une tranche SET vec = [id % 13 + 0.5,
// id % 19, id % 31, id + 0.25], une instruction par tranche, en auto-commit. Ses chiffres
// d'une passe (1000 lignes) : depuis NULL, 1000 ; ligne à ligne, 969 ; par lots de 512,
// 532 ; vingt lignes vers le même vecteur en une instruction, 998 ; vingt SET distincts
// ou vers le vecteur d'une autre ligne, 1000.
//
// État au 4 octobre, après c8fdaf196 (les anciens voisins d'une ligne mise à jour restent
// joignables ; l'index relit juste ses arêtes) : vingt lignes vers le même vecteur passe
// au vert ; les autres mises à jour perdent encore des lignes, un essai sur deux environ
// (marche « mise à jour de l'index : contrôle en fin d'instruction ») ; le chemin du
// produit (depuis NULL, et le remplacement qui repasse par NULL ligne à ligne) est vert.
// Un défaut sans aucune mise à jour : une ligne lointaine dans un nuage serré, index bâti
// d'un coup.
//
// Sur master du 3 octobre, mille SET de vecteur en une instruction peuvent échouer sur
// « bitset::set: __position (which is 2049) >= _Nb (which is 2048) » ; les lots restent
// donc à 512 lignes au plus.

#ifndef __SINGLE_THREADED__

#include <sys/wait.h>
#include <unistd.h>

#include <cmath>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <chrono>
#include <map>
#include <unordered_map>
#include <optional>
#include <random>
#include <algorithm>
#include <set>

#include "bench_harness.h"
#include "catalog/catalog.h"
#include "catalog/catalog_entry/table_catalog_entry.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/client_context.h"
#include "processor/result/flat_tuple.h"
#include "storage/index/index.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
#include "transaction/transaction.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr auto ORIGIN_VECTOR = "[n.id % 17, n.id % 23, n.id % 29, n.id]";
constexpr auto OTHER_VECTOR = "[n.id % 13 + 0.5, n.id % 19, n.id % 31, n.id + 0.25]";

class VectorIndexUpdate : public EmptyDBTest {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        systemConfig->bufferPoolSize = 1024 * 1024 * 1024;
        createDBAndConn();
        mustRun("CALL auto_checkpoint=false;");
        concurrency::loadVectorExtension(*conn);
        if (const auto* runs = std::getenv("CONCURRENCE_VECTOR_RUNS")) {
            forcedRuns = std::max(1, std::atoi(runs));
        }
    }

    void mustRun(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << "[check: query] " << query.substr(0, 200) << "\n"
                                         << result->getErrorMessage();
    }

    // Une table Doc<run> de numRows lignes de dimension 4 à la recette du cœur C++ (ou à
    // NULL), indexée après le remplissage.
    void createDocs(int run, int64_t numRows, bool nullVectors = false) {
        table = stringFormat("Doc{}", run);
        mustRun(stringFormat("CREATE NODE TABLE {}(id INT64 PRIMARY KEY, vec FLOAT[4], "
                             "marker STRING, vec2 FLOAT[4]);",
            table));
        mustRun(stringFormat("UNWIND range(0, {}) AS i CREATE (n:{} {id: i}) SET n.vec = {};",
            numRows - 1, table, nullVectors ? "NULL" : ORIGIN_VECTOR));
        createIndex();
    }

    void createIndex(const std::string& index = "doc_index", const std::string& column = "vec") {
        mustRun(stringFormat("CALL CREATE_VECTOR_INDEX('{}', '{}', '{}', metric := 'l2');", table,
            index, column));
    }

    // Une instruction par tranche [begin, end) de batch lignes.
    void setInSlices(int64_t begin, int64_t end, int64_t batch, const std::string& vector) {
        for (auto low = begin; low < end; low += batch) {
            mustRun(stringFormat("MATCH (n:{}) WHERE n.id >= {} AND n.id < {} SET n.vec = {};",
                table, low, std::min(end, low + batch), vector));
        }
    }

    // L'invariant : rend la liste des étiquettes en défaut, et imprime les comptes.
    // ownStep : ne chercher son propre vecteur que pour une ligne sur ownStep.
    std::set<std::string> check(const std::string& label, int64_t ownStep = 1,
        const std::string& index = "doc_index", const std::string& column = "vec") {
        std::set<std::string> failed;
        std::vector<std::pair<int64_t, std::string>> rows;
        auto scan = conn->query(stringFormat("MATCH (n:{}) WHERE n.{} IS NOT NULL RETURN n.id, n.{};",
            table, column, column));
        while (scan->hasNext()) {
            auto tuple = scan->getNext();
            rows.emplace_back(tuple->getValue(0)->getValue<int64_t>(), tuple->getValue(1)->toString());
        }
        if (rows.empty()) {
            return failed;
        }
        std::set<int64_t> alive;
        for (const auto& row : rows) {
            alive.insert(row.first);
        }
        const auto numRows = rows.size();
        std::set<int64_t> reached;
        int64_t dead = 0;
        auto exhaustive = conn->query(stringFormat(
            "CALL QUERY_VECTOR_INDEX('{}', '{}', {}, {}, efs := {}) RETURN node.id;", table, index,
            rows.front().second, numRows, numRows));
        if (!exhaustive->isSuccess()) {
            std::cerr << label << ": " << exhaustive->getErrorMessage() << "\n";
            failed.insert("query");
            return failed;
        }
        while (exhaustive->hasNext()) {
            const auto id = exhaustive->getNext()->getValue(0)->getValue<int64_t>();
            if (alive.contains(id)) {
                reached.insert(id);
            } else {
                dead++;
            }
        }
        int64_t asked = 0;
        int64_t foundFirst = 0;
        const auto efs = std::min<uint64_t>(numRows, 1000);
        for (auto i = 0u; i < numRows; i += ownStep) {
            const auto& [id, vector] = rows[i];
            asked++;
            auto own = conn->query(stringFormat(
                "CALL QUERY_VECTOR_INDEX('{}', '{}', {}, 30, efs := {}) RETURN node.id, "
                "distance;",
                table, index, vector, efs));
            // Les résultats ne sont pas triés : la ligne doit être à la plus petite distance.
            std::vector<std::pair<int64_t, double>> results;
            double best = INFINITY;
            while (own->hasNext()) {
                auto tuple = own->getNext();
                const auto distance = std::stod(tuple->getValue(1)->toString());
                results.emplace_back(tuple->getValue(0)->getValue<int64_t>(), distance);
                best = std::min(best, distance);
                if (!alive.contains(results.back().first)) {
                    dead++;
                }
            }
            auto found = false;
            for (const auto& [resultId, distance] : results) {
                if (resultId == id && distance <= best + 1e-6) {
                    found = true;
                    break;
                }
            }
            foundFirst += found;
            if (!found && asked - foundFirst <= 3) {
                std::cerr << "  row " << id << " " << vector << " not first; nearest:";
                for (auto r = 0u; r < std::min<size_t>(results.size(), 5); ++r) {
                    std::cerr << " " << results[r].first << "@" << results[r].second;
                }
                std::cerr << " (" << results.size() << " results)\n";
            }
        }
        std::cerr << label << ": " << reached.size() << "/" << numRows
                  << " reachable, " << foundFirst << "/" << asked << " find themselves, " << dead
                  << " dead rows returned\n";
        // Une seule étiquette pour les deux contrôles : d'une passe à l'autre, la même
        // perte se voit par l'un, par l'autre ou par les deux.
        if (reached.size() != numRows || foundFirst != asked) {
            failed.insert("vector-index-exact");
        }
        if (dead != 0) {
            failed.insert("vector-search-no-dead-row");
        }
        return failed;
    }

    // Joue scenario runs fois ; l'échec porte l'union des étiquettes de toutes les passes.
    // runs : choisi par cas. Un rouge perd un essai sur deux environ (4 octobre, après
    // c8fdaf196) : assez d'essais pour qu'une passe entière ne le manque pas ; les
    // garde-fous verts les plus lents en ont moins. CONCURRENCE_VECTOR_RUNS les remplace tous.
    template<typename Scenario>
    void runRepeatedly(Scenario scenario, int64_t ownStep = 1, int runs = 5) {
        std::set<std::string> failed;
        int failedRuns = 0;
        if (forcedRuns > 0) {
            runs = forcedRuns;
        }
        for (auto run = 0; run < runs; ++run) {
            scenario(run);
            if (HasFatalFailure()) {
                return;
            }
            auto runFailed = check(stringFormat("run {}", run), ownStep);
            if (checkSecondIndex) {
                const auto second =
                    check(stringFormat("run {}, second index", run), ownStep, "doc_index2", "vec2");
                runFailed.insert(second.begin(), second.end());
            }
            failedRuns += !runFailed.empty();
            failed.insert(runFailed.begin(), runFailed.end());
        }
        std::string checks;
        for (const auto& name : failed) {
            checks += "[check: " + name + "] ";
        }
        EXPECT_TRUE(failed.empty()) << checks << failedRuns << " of " << runs
                                    << " runs lose rows in the vector index";
    }

    int forcedRuns = 0;
    bool checkSecondIndex = false;
    std::string table;
};

// Garde-fou vert : le chemin principal de rag3weaver, une ligne créée sans vecteur puis
// remplie.
TEST_F(VectorIndexUpdate, SetFromNullLineByLine) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000, true /* nullVectors */);
        setInSlices(0, 1000, 1, OTHER_VECTOR);
    }, 1 /* ownStep */, 3 /* runs */);
}

TEST_F(VectorIndexUpdate, SetToAnotherVectorLineByLine) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        setInSlices(0, 1000, 1, OTHER_VECTOR);
    });
}

TEST_F(VectorIndexUpdate, SetToAnotherVectorInBatchesOf512) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        setInSlices(0, 1000, 512, OTHER_VECTOR);
    }, 1 /* ownStep */, 10 /* runs */);
}

// Vingt lignes vers le même vecteur, en une instruction (998 dans la passe du cœur C++).
TEST_F(VectorIndexUpdate, TwentyRowsToTheSameVectorInOneStatement) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        setInSlices(100, 120, 20, "[7, 7, 7, 7]");
    });
}

// Garde-fous verts du cœur C++ : vingt SET distincts, ou vers le vecteur d'une autre ligne.
TEST_F(VectorIndexUpdate, TwentyRowsToDistinctVectorsLineByLine) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        setInSlices(100, 120, 1, OTHER_VECTOR);
    }, 1 /* ownStep */, 20 /* runs */);
}

TEST_F(VectorIndexUpdate, TwentyRowsToAnotherRowsVector) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        mustRun(stringFormat("MATCH (n:{}), (m:{}) WHERE n.id >= 100 AND n.id < 120 AND "
                             "m.id = n.id + 500 SET n.vec = m.vec;",
            table, table));
    });
}

// La dimension réelle des plongements de rag3weaver.
TEST_F(VectorIndexUpdate, RealDimension768InBatchesOf100) {
    const auto vector = [](int64_t id, int64_t shift) {
        std::string text = "[";
        for (auto d = 0; d < 768; ++d) {
            text += (d ? "," : "") +
                    std::to_string(std::sin(0.37 * static_cast<double>(id * 768 + d) + shift));
        }
        return text + "]";
    };
    const auto fill = [&](int64_t shift, bool create) {
        for (auto low = 0; low < 1000; low += 100) {
            std::string list = "[";
            for (auto id = low; id < low + 100; ++id) {
                list += (id > low ? "," : "") + stringFormat("{id: {}, vec: {}}", id, vector(id, shift));
            }
            list += "]";
            mustRun(create ?
                        stringFormat("UNWIND {} AS x CREATE (:{} {id: x.id, vec: x.vec});", list, table) :
                        stringFormat("UNWIND {} AS x MATCH (n:{} {id: x.id}) SET n.vec = x.vec;", list,
                            table));
        }
    };
    runRepeatedly([&](int run) {
        table = stringFormat("Doc{}", run);
        mustRun(stringFormat("CREATE NODE TABLE {}(id INT64 PRIMARY KEY, vec FLOAT[768]);", table));
        fill(0, true);
        createIndex();
        // Par UNWIND de cent lignes, comme rag3weaver écrit ses plongements.
        fill(1, false);
    }, 1 /* ownStep */, 1 /* runs */);
}

// Lourd : 25 s avant c8fdaf196, 12 à 27 min après (la mise à jour s'est beaucoup ralentie,
// signalé au cœur C++ le 4 octobre). Probabiliste (probabilistic.txt) : rouge six essais sur
// six, puis vert une fois.
TEST_F(VectorIndexUpdate, TenThousandRowsInBatchesOf512) {
    runRepeatedly(
        [&](int run) {
            createDocs(run, 10000);
            setInSlices(0, 10000, 512, OTHER_VECTOR);
        },
        10 /* ownStep */, 2 /* runs */);
}

// Une même ligne mise à jour cinquante fois, chaque fois vers un autre vecteur.
TEST_F(VectorIndexUpdate, OneRowUpdatedManyTimes) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        for (auto k = 1; k <= 50; ++k) {
            mustRun(stringFormat("MATCH (n:{}) WHERE n.id = 500 SET n.vec = [{}, {}, {}, {}];",
                table, 500 % 13 + 0.5 * k, k % 19, k % 31, 500 + 0.25 * k));
        }
    }, 1 /* ownStep */, 20 /* runs */);
}

// Mise à jour puis suppression d'une ligne sur trois : les restantes joignables, aucune
// supprimée rendue.
TEST_F(VectorIndexUpdate, UpdateThenDelete) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        setInSlices(0, 1000, 512, OTHER_VECTOR);
        mustRun(stringFormat("MATCH (n:{}) WHERE n.id % 3 = 0 DELETE n;", table));
    }, 1 /* ownStep */, 10 /* runs */);
}

// Une mise à jour annulée : l'index doit revenir à ce qu'il était, chaque ligne joignable
// par son vecteur d'origine.
TEST_F(VectorIndexUpdate, UpdateInARolledBackTransaction) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        mustRun("BEGIN TRANSACTION;");
        setInSlices(0, 300, 100, OTHER_VECTOR);
        // Le cas n'éprouve rien si la mise à jour n'a pas eu lieu avant l'annulation.
        auto updated = conn->query(stringFormat(
            "MATCH (n:{}) WHERE n.vec <> {} RETURN count(*);", table, ORIGIN_VECTOR));
        ASSERT_TRUE(updated->isSuccess()) << "[check: query] " << updated->getErrorMessage();
        ASSERT_EQ(updated->getNext()->getValue(0)->getValue<int64_t>(), 300)
            << "[check: update-before-rollback] ";
        mustRun("ROLLBACK;");
        auto changed = conn->query(stringFormat(
            "MATCH (n:{}) WHERE n.vec <> {} RETURN count(*);", table, ORIGIN_VECTOR));
        ASSERT_TRUE(changed->isSuccess()) << "[check: query] " << changed->getErrorMessage();
        ASSERT_EQ(changed->getNext()->getValue(0)->getValue<int64_t>(), 0)
            << "[check: rollback-restores-table] ";
    });
}

// Ce que rag3weaver fait réellement au moteur (session des embarquements, 4 octobre,
// extension/rag3weaver/tests/e2e_invariant_des_vecteurs.rs).

// Le chemin principal : des lignes créées sans vecteur, remplies par lots de 32 avec leur
// marqueur.
TEST_F(VectorIndexUpdate, SetFromNullInBatchesOf32WithAMarker) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000, true /* nullVectors */);
        for (auto low = 0; low < 1000; low += 32) {
            mustRun(stringFormat("UNWIND range({}, {}) AS i MATCH (n:{} {id: i}) SET n.vec = [i % 13 "
                                 "+ 0.5, i % 19, i % 31, i + 0.25], n.marker = concat('h', CAST(i AS STRING));",
                low, std::min(999, low + 31), table));
        }
    });
}

// Le remplacement d'un vecteur tel que rag3weaver l'écrit depuis le 3 octobre au soir
// (write_vectors) : une ligne par instruction, en repassant par NULL.
void replaceThroughNull(VectorIndexUpdate& test, int64_t id, const std::string& vector) {
    test.mustRun(stringFormat("MATCH (n:{}) WHERE n.id = {} SET n.vec = NULL, n.marker = '';",
        test.table, id));
    test.mustRun(stringFormat("MATCH (n:{}) WHERE n.id = {} SET n.vec = {}, n.marker = 'h{}';",
        test.table, id, vector, id));
}

TEST_F(VectorIndexUpdate, ReplaceThroughNullEveryRow) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        for (auto id = 0; id < 1000; ++id) {
            replaceThroughNull(*this, id, OTHER_VECTOR);
        }
    }, 1 /* ownStep */, 1 /* runs */);
}

TEST_F(VectorIndexUpdate, ReplaceThroughNullEveryOtherRow) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        for (auto id = 0; id < 1000; id += 2) {
            replaceThroughNull(*this, id, OTHER_VECTOR);
        }
    }, 1 /* ownStep */, 1 /* runs */);
}

TEST_F(VectorIndexUpdate, ReplaceThroughNullEveryThirdRow) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        for (auto id = 0; id < 1000; id += 3) {
            replaceThroughNull(*this, id, OTHER_VECTOR);
        }
    }, 1 /* ownStep */, 2 /* runs */);
}

TEST_F(VectorIndexUpdate, ReplaceThroughNullTheSameRowManyTimes) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        for (auto k = 1; k <= 50; ++k) {
            replaceThroughNull(*this, 500,
                stringFormat("[{}, {}, {}, {}]", 500 % 13 + 0.5 * k, k % 19, k % 31, 500 + 0.25 * k));
        }
    });
}

// Un second index sur une seconde colonne de la même table (un autre modèle), rempli
// depuis NULL pendant que le premier existe : les deux restent justes.
TEST_F(VectorIndexUpdate, SecondIndexOnASecondColumnFilledFromNull) {
    checkSecondIndex = true;
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        createIndex("doc_index2", "vec2");
        for (auto low = 0; low < 1000; low += 32) {
            mustRun(stringFormat("UNWIND range({}, {}) AS i MATCH (n:{} {id: i}) SET n.vec2 = [i % 7, "
                                 "i % 11 + 0.5, i % 37, i * 2];",
                low, std::min(999, low + 31), table));
        }
    });
}

// Le gros rattrapage : l'index retiré puis recréé sur une table pleine, après des mises à
// jour.
TEST_F(VectorIndexUpdate, DropThenCreateTheIndexOnAFullTable) {
    runRepeatedly([&](int run) {
        createDocs(run, 1000);
        setInSlices(0, 1000, 512, OTHER_VECTOR);
        mustRun(stringFormat("CALL DROP_VECTOR_INDEX('{}', 'doc_index');", table));
        createIndex();
    }, 1 /* ownStep */, 2 /* runs */);
}

// Une ligne très éloignée des autres dans un index bâti d'un coup, sans aucune mise à
// jour (reproduction du cœur C++ : 500 lignes, une à [9000, 2, 3, 4]). Deux nuages : la
// recette du cœur C++, dont la dernière coordonnée va jusqu'à 499, et un nuage serré où
// toutes les coordonnées restent sous 31.
static void farRowScenario(VectorIndexUpdate& test, int run, const std::string& vector) {
    test.table = stringFormat("Doc{}", run);
    test.mustRun(
        stringFormat("CREATE NODE TABLE {}(id INT64 PRIMARY KEY, vec FLOAT[4]);", test.table));
    test.mustRun(stringFormat("UNWIND range(0, 499) AS i CREATE (n:{} {id: i}) SET n.vec = {};",
        test.table, vector));
    test.mustRun(
        stringFormat("MATCH (n:{}) WHERE n.id = 250 SET n.vec = [9000, 2, 3, 4];", test.table));
    test.createIndex();
}

TEST_F(VectorIndexUpdate, FarRowInAnIndexBuiltAtOnce) {
    runRepeatedly([&](int run) { farRowScenario(*this, run, ORIGIN_VECTOR); });
}

TEST_F(VectorIndexUpdate, FarRowFromATightCloudInAnIndexBuiltAtOnce) {
    runRepeatedly([&](int run) {
        farRowScenario(*this, run, "[n.id % 17, n.id % 23, n.id % 29, n.id % 31]");
    }, 1 /* ownStep */, 30 /* runs */);
}

// Un COPY refusé (une clé en double) laisse la cardinalité de la table gonflée des lignes
// qu'il avait ajoutées : l'annulation retire les lignes, pas les statistiques fusionnées
// (NodeBatchInsert, node_batch_insert.cpp). Avant 1ea49837f, CREATE_VECTOR_INDEX bornait
// son parcours par cette cardinalité alors que son graphe en mémoire est dimensionné par
// les vraies lignes : il écrivait hors de ses tableaux, et le processus mourait (SIGSEGV).
// Trouvé le 4 octobre par l'essai déterministe de la corruption d'e2e_code (ticket
// 2026-10-04-memoire-corrompue-dans-e2e-code.md) ; le lien avec ce rapport reste à prouver.
static void fillThenRefuseACopy(VectorIndexUpdate& test) {
    test.table = "DocCopy";
    test.mustRun("CREATE NODE TABLE DocCopy(id INT64 PRIMARY KEY, vec FLOAT[4], s STRING);");
    test.mustRun("UNWIND range(0, 199) AS i CREATE (:DocCopy {id: i, vec: [i % 17, i % 23, "
                 "i % 29, i], s: lpad(CAST(i % 7 AS STRING), 4096 + (i % 7) * 4000, 'x')});");
    test.createIndex();
    test.mustRun("CHECKPOINT;");
    const auto csv = test.databasePath + ".refused.csv";
    {
        std::ofstream out(csv);
        for (auto i = 200; i < 400; ++i) {
            out << i << ",\"[1,2,3," << i << "]\"," << std::string(5000, 'y') << "\n";
        }
        out << "5,\"[1,2,3,4]\",dup\n";
    }
    auto copy = test.conn->query("COPY DocCopy FROM '" + csv + "' (header=false);");
    ASSERT_FALSE(copy->isSuccess()) << "[check: copy-refused] the COPY was meant to fail";
}

TEST_F(VectorIndexUpdate, CreateIndexAfterARefusedCopy) {
    fillThenRefuseACopy(*this);
    if (HasFatalFailure()) {
        return;
    }
    mustRun("CALL DROP_VECTOR_INDEX('DocCopy', 'doc_index');");
    conn.reset();
    database.reset();
    // Dans un fils : avant 1ea49837f, la création tuait le processus.
    const auto pid = fork();
    if (pid == 0) {
        concurrency::disableCoreDumps();
        createDBAndConn();
        concurrency::loadVectorExtension(*conn);
        auto created =
            conn->query("CALL CREATE_VECTOR_INDEX('DocCopy', 'doc_index', 'vec', metric := 'l2');");
        _exit(created->isSuccess() ? 0 : 5);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    ASSERT_TRUE(WIFEXITED(status) && WEXITSTATUS(status) == 0)
        << "[check: create-after-refused-copy] "
        << (WIFSIGNALED(status) ? "killed by signal " + std::to_string(WTERMSIG(status)) :
                                  "exit code " + std::to_string(WEXITSTATUS(status)));
    createDBAndConn();
    concurrency::loadVectorExtension(*conn);
    const auto failed = check("after a refused COPY");
    std::string checks;
    for (const auto& name : failed) {
        checks += "[check: " + name + "] ";
    }
    EXPECT_TRUE(failed.empty()) << checks;
    auto strings = conn->query("MATCH (d:DocCopy) WHERE size(d.s) <> 4096 + (d.id % 7) * 4000 "
                               "RETURN count(*);");
    EXPECT_EQ(strings->getNext()->getValue(0)->getValue<int64_t>(), 0)
        << "[check: long-strings-intact] ";
}

// La ligne d'origine d'une clé que le COPY refusé portait en double (id 5) doit rester
// trouvable par sa clé. L'annulation (RollbackPKDeleter) retire les clés des lignes annulées
// de la partie en mémoire de l'index de clé primaire, celle de la ligne d'origine comprise ;
// une clé déjà passée par un point de reprise survit. D'où la recette de la session cœur
// C++ : pas d'index vectoriel (sa création écrit un point de reprise), pas de CHECKPOINT
// avant le COPY. La table garde la ligne ; c'est la recherche par clé qui la perd, et un
// MERGE de la clé créerait un doublon.
static void insertThenRefuseACopy(VectorIndexUpdate& test) {
    test.mustRun("CREATE NODE TABLE KeyCopy(id INT64 PRIMARY KEY, name STRING);");
    test.mustRun("UNWIND range(0, 199) AS i CREATE (:KeyCopy {id: i, name: 'row ' + "
                 "CAST(i AS STRING)});");
    const auto csv = test.databasePath + ".refused-keys.csv";
    {
        std::ofstream out(csv);
        for (auto i = 200; i < 400; ++i) {
            out << i << ",new " << i << "\n";
        }
        out << "5,duplicate\n";
    }
    auto copy = test.conn->query("COPY KeyCopy FROM '" + csv + "' (header=false);");
    ASSERT_FALSE(copy->isSuccess()) << "[check: copy-refused] the COPY was meant to fail";
}

TEST_F(VectorIndexUpdate, RefusedCopyKeepsTheOriginalKey) {
    insertThenRefuseACopy(*this);
    if (HasFatalFailure()) {
        return;
    }
    auto count = [&](const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << "[check: query] " << result->getErrorMessage();
        return result->isSuccess() ? result->getNext()->getValue(0)->getValue<int64_t>() : -1;
    };
    // « d.id + 0 » force un balayage : « WHERE d.id = 5 » devient une recherche par clé.
    // UNWIND … MATCH par clé balaie aussi la table (planificateur) : il retrouve la ligne.
    EXPECT_EQ(count("MATCH (d:KeyCopy) WHERE d.id + 0 = 5 RETURN count(*);"), 1)
        << "[check: original-row-scanned] the row itself";
    EXPECT_EQ(count("MATCH (d:KeyCopy {id: 5}) RETURN count(*);"), 1)
        << "[check: original-key-found] MATCH by key";
    EXPECT_EQ(count("MATCH (d:KeyCopy) WHERE d.id = 5 RETURN count(*);"), 1)
        << "[check: original-key-found] WHERE on the key";
    mustRun("MERGE (d:KeyCopy {id: 5}) SET d.name = 'merged';");
    EXPECT_EQ(count("MATCH (d:KeyCopy) WHERE d.id + 0 = 5 RETURN count(*);"), 1)
        << "[check: merge-no-duplicate] MERGE of the original key";
}

// Le CHECKPOINT qui suit un COPY refusé, sans point de reprise avant : l'annulation ramène
// le compte de lignes du groupe mais pas ses colonnes, et le point de reprise écrivait hors
// de son bloc (ASan : NullMask::copyNullMask sous NodeGroup::checkpointInMemOnly, session
// cœur C++). Dans un fils : le tas corrompu peut tuer le processus.
TEST_F(VectorIndexUpdate, CheckpointAfterARefusedCopy) {
    conn.reset();
    database.reset();
    const auto pid = fork();
    if (pid == 0) {
        concurrency::disableCoreDumps();
        createDBAndConn();
        conn->query("CALL auto_checkpoint=false;");
        insertThenRefuseACopy(*this);
        auto checkpoint = conn->query("CHECKPOINT;");
        if (!checkpoint->isSuccess()) {
            _exit(5);
        }
        auto rows = conn->query("MATCH (d:KeyCopy) RETURN count(*), sum(size(d.name));");
        auto tuple = rows->getNext();
        _exit(tuple->getValue(0)->getValue<int64_t>() == 200 ? 0 : 6);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    EXPECT_TRUE(WIFEXITED(status) && WEXITSTATUS(status) == 0)
        << "[check: checkpoint-after-refused-copy] "
        << (WIFSIGNALED(status) ? "killed by signal " + std::to_string(WTERMSIG(status)) :
                                  "exit code " + std::to_string(WEXITSTATUS(status)));
    createDBAndConn();
}

// Le défaut lui-même, qui demeure après 1ea49837f : la cardinalité que STATS_INFO rend (et que
// le planificateur lit) compte les lignes d'un COPY refusé. C'est une estimation : le COPY
// fusionne ses statistiques dans la table avant sa validation, et rien ne les en retire. Le
// recalage du 5 octobre (voie (c), comme le reltuples de PostgreSQL) la remet sur le nombre de
// lignes au point de reprise et à la lecture : juste après le point de reprise, et après la
// réouverture.
TEST_F(VectorIndexUpdate, RefusedCopyLeavesTheCardinalityTrueAfterACheckpoint) {
    fillThenRefuseACopy(*this);
    if (HasFatalFailure()) {
        return;
    }
    mustRun("CHECKPOINT;");
    for (const auto* moment : {"after a checkpoint", "after reopening"}) {
        auto card = conn->query("CALL STATS_INFO('DocCopy') RETURN cardinality;");
        ASSERT_TRUE(card->isSuccess()) << "[check: query] " << card->getErrorMessage();
        EXPECT_EQ(card->getNext()->getValue(0)->toString(), "200")
            << "[check: cardinality-matches-rows] " << moment;
        card.reset();
        conn.reset();
        database.reset();
        createDBAndConn();
    }
}

// Avant tout point de reprise, l'estimation reste gonflée : il faut que le COPY garde ses
// statistiques dans sa transaction (voie (a), avec l'étape 4 du chargement journalisé).
TEST_F(VectorIndexUpdate, RefusedCopyLeavesTheCardinalityTrueAtOnce) {
    fillThenRefuseACopy(*this);
    if (HasFatalFailure()) {
        return;
    }
    auto card = conn->query("CALL STATS_INFO('DocCopy') RETURN cardinality;");
    ASSERT_TRUE(card->isSuccess()) << "[check: query] " << card->getErrorMessage();
    EXPECT_EQ(card->getNext()->getValue(0)->toString(), "200")
        << "[check: cardinality-matches-rows-at-once] ";
}

// Le fabricant de la base gardée de InflatedCardinalityIsRecalibratedAtOpening : sauté, sauf si
// BANC_FABRIQUER donne un dossier. À jouer sur un moteur d'AVANT le recalage, qui écrit sur
// disque une cardinalité gonflée par un COPY annulé.
TEST_F(VectorIndexUpdate, FabricateADatabaseWithAnInflatedCardinality) {
    const char* target = std::getenv("BANC_FABRIQUER");
    if (target == nullptr) {
        GTEST_SKIP() << "only run to fabricate the kept database";
    }
    const auto csv = databasePath + ".inflate.csv";
    {
        std::ofstream out(csv);
        for (auto i = 200; i < 400; ++i) {
            out << i << ",n" << i << "\n";
        }
    }
    for (const auto& query : std::vector<std::string>{
             "CREATE NODE TABLE T(id INT64 PRIMARY KEY, s STRING);",
             "UNWIND range(0, 199) AS i CREATE (:T {id: i, s: 'r'});", "CHECKPOINT;",
             "BEGIN TRANSACTION;", "COPY T FROM '" + csv + "' (header=false);", "ROLLBACK;",
             "CHECKPOINT;"}) {
        mustRun(query);
    }
    auto card = conn->query("CALL STATS_INFO('T') RETURN cardinality;");
    std::cerr << "  fabricated with STATS_INFO " << card->getNext()->getValue(0)->toString()
              << "\n";
    card.reset();
    conn.reset();
    database.reset();
    std::filesystem::create_directories(target);
    std::filesystem::copy(databasePath, std::string(target) + "/db.kz",
        std::filesystem::copy_options::overwrite_existing);
    std::filesystem::remove(csv);
}

// Une base dont la cardinalité gonflée est déjà écrite sur disque (fabriquée par le moteur
// d'avant le recalage, dataset/databases/inflated-cardinality) se recale à la lecture.
TEST_F(VectorIndexUpdate, InflatedCardinalityIsRecalibratedAtOpening) {
    conn.reset();
    database.reset();
    std::filesystem::remove_all(databasePath);
    const auto fixture =
        TestHelper::appendRag3dbRootPath("dataset/databases/inflated-cardinality/db.kz.gz");
    ASSERT_EQ(std::system(("gzip -dc '" + fixture + "' > '" + databasePath + "'").c_str()), 0)
        << "[check: setup] the kept database could not be unpacked";
    createDBAndConn();
    auto card = conn->query("CALL STATS_INFO('T') RETURN cardinality;");
    ASSERT_TRUE(card->isSuccess()) << "[check: query] " << card->getErrorMessage();
    EXPECT_EQ(card->getNext()->getValue(0)->toString(), "200")
        << "[check: cardinality-recalibrated-at-opening] ";
}

// Le miroir de HNSWStorageInfo (extension/vector/src/include/index/hnsw_index.h), pour écrire
// son compte sans tirer les en-têtes de l'extension dans le banc. Sa disposition est vérifiée
// avant toute écriture : le compte lu doit égaler le nombre de lignes reliées.
struct HNSWStorageInfoMirror : rag3db::storage::IndexStorageInfo {
    table_id_t upperRelTableID;
    table_id_t lowerRelTableID;
    offset_t upperEntryPoint;
    offset_t lowerEntryPoint;
    offset_t numCheckpointedNodes;
};

// Le compte des lignes reliées de l'index, lu ou, avec newCount, écrit par les internes.
offset_t indexedRowCount(rag3db::main::Connection& connection, const std::string& table,
    const std::string& index, std::optional<offset_t> newCount = std::nullopt) {
    EXPECT_TRUE(connection.query("BEGIN TRANSACTION READ ONLY;")->isSuccess());
    auto* context = connection.getClientContext();
    auto* transaction = rag3db::transaction::Transaction::Get(*context);
    const auto* entry =
        rag3db::catalog::Catalog::Get(*context)->getTableCatalogEntry(transaction, table);
    auto& nodeTable = rag3db::storage::StorageManager::Get(*context)
                          ->getTable(entry->getTableID())
                          ->cast<rag3db::storage::NodeTable>();
    auto* found = nodeTable.getIndex(index).value();
    auto& info = static_cast<HNSWStorageInfoMirror&>(
        const_cast<rag3db::storage::IndexStorageInfo&>(found->getStorageInfo()));
    if (newCount) {
        info.numCheckpointedNodes = *newCount;
    }
    const auto count = info.numCheckpointedNodes;
    EXPECT_TRUE(connection.query("COMMIT;")->isSuccess());
    return count;
}

// Le nombre d'arêtes de la couche basse de l'index : sa table de relations cachée, que Cypher ne
// voit pas, lue par les internes.
int64_t lowerGraphEdgeCount(rag3db::main::Connection& connection, const std::string& table,
    const std::string& index) {
    EXPECT_TRUE(connection.query("BEGIN TRANSACTION READ ONLY;")->isSuccess());
    auto* context = connection.getClientContext();
    auto* transaction = rag3db::transaction::Transaction::Get(*context);
    const auto* entry =
        rag3db::catalog::Catalog::Get(*context)->getTableCatalogEntry(transaction, table);
    auto* storage = rag3db::storage::StorageManager::Get(*context);
    auto& nodeTable = storage->getTable(entry->getTableID())->cast<rag3db::storage::NodeTable>();
    const auto& info = static_cast<const HNSWStorageInfoMirror&>(
        nodeTable.getIndex(index).value()->getStorageInfo());
    const auto count =
        static_cast<int64_t>(storage->getTable(info.lowerRelTableID)->getNumTotalRows(transaction));
    EXPECT_TRUE(connection.query("COMMIT;")->isSuccess());
    return count;
}

// Le filet de e1049934e : un index qui compte plus de lignes reliées que sa table n'en contient
// (une base écrite avant le crochet d'annulation) refuse sous le nom « is behind its table », que
// rag3weaver reconnaît pour retirer l'index et le rebâtir. Aucune base abîmée ne le déclenche
// sur le moteur corrigé : le compte est écrit par les internes, à 210 pour 200 lignes. La
// recherche, la mise à jour d'un vecteur et la fin d'un COPY de 5 lignes (205 < 210) doivent
// refuser par ce nom ; le COPY refusé est annulé ; DROP puis CREATE rendent un index juste.
TEST_F(VectorIndexUpdate, IndexCountingMoreRowsThanItsTableIsRefusedByName) {
    table = "Lag";
    mustRun("CREATE NODE TABLE Lag(id INT64 PRIMARY KEY, vec FLOAT[4]);");
    mustRun(stringFormat("UNWIND range(0, 199) AS i CREATE (n:Lag {id: i}) SET n.vec = {};",
        ORIGIN_VECTOR));
    createIndex();
    mustRun("CHECKPOINT;");
    ASSERT_EQ(indexedRowCount(*conn, table, "doc_index"), 200u)
        << "[check: setup] the mirror of HNSWStorageInfo does not read the index's row count";
    indexedRowCount(*conn, table, "doc_index", 210);
    const auto refusedByName = [&](const std::string& query, const char* check) {
        auto result = conn->query(query);
        const auto message = result->isSuccess() ? std::string("<success>") :
                                                   result->getErrorMessage();
        std::cerr << "  " << check << ": " << message.substr(0, 300) << "\n";
        EXPECT_NE(message.find("is behind its table"), std::string::npos)
            << "[check: " << check << "] " << message.substr(0, 300);
    };
    refusedByName("CALL QUERY_VECTOR_INDEX('Lag', 'doc_index', [1.0, 2.0, 3.0, 4.0], 5) RETURN "
                  "node.id;",
        "search-refused");
    refusedByName("MATCH (n:Lag {id: 3}) SET n.vec = [9.0, 9.0, 9.0, 9.0];", "update-refused");
    const auto csv = databasePath + ".lag.csv";
    {
        std::ofstream out(csv);
        for (auto i = 1000; i < 1005; ++i) {
            out << i << ",\"[1.0,2.0,3.0," << i << ".0]\"\n";
        }
    }
    refusedByName("COPY Lag FROM '" + csv + "' (header=false);", "copy-refused");
    auto rows = conn->query("MATCH (n:Lag) RETURN count(n);");
    EXPECT_EQ(rows->getNext()->getValue(0)->getValue<int64_t>(), 200)
        << "[check: refused-copy-rolled-back] ";
    rows.reset();
    mustRun("CALL DROP_VECTOR_INDEX('Lag', 'doc_index');");
    createIndex();
    const auto failed = check("after the index is built again");
    std::string checks;
    for (const auto& name : failed) {
        checks += "[check: " + name + "] ";
    }
    EXPECT_TRUE(failed.empty()) << checks;
    std::filesystem::remove(csv);
}

// SONDE, non commitée : la joignabilité sur les vrais vecteurs de l'arbre principal
// (~/.cache/rag3weaver-build/joignabilite-export/Scope_Chunk.csv : uuid, décalage, 768
// composantes), « masse » (COPY des vecteurs, puis l'index) contre « fond » (COPY sans
// vecteur, une réclamation par lots de 512, les vecteurs par lots de 32, puis l'index).
// Des amas de quasi-copies et des groupes de copies exactes (banc, 5 octobre). Sur les vrais
// vecteurs, les introuvables étaient surtout des quasi-doublons : joignables par une recherche
// exhaustive, pas par la leur. La règle d'élagage était inversée : le plus lointain d'un amas
// serré restait, le plus proche partait. La règle classique laisse les copies exactes
// s'accumuler (deux copies ne se couvrent jamais) : un groupe plus grand que le degré n'aurait
// plus de sortie, d'où le groupe de 49 (ml = 60, mu = 30).
//
// Données tirées d'une graine fixe : 2 000 points au hasard, 40 amas de 10 quasi-copies à
// environ 1e-5 l'une de l'autre en cosinus, trois groupes exacts de chaque taille de 2 à 9, et
// un groupe exact de 49. La moitié des lignes est chargée par COPY avant l'index (le graphe
// bâti en mémoire), l'autre insérée après (l'élagage sur disque). L'attendu : chaque ligne est
// joignable par une recherche exhaustive, et sa propre recherche (k = 10) la rend, ou rend une
// de ses copies exactes à distance nulle.
TEST_F(VectorIndexUpdate, NearCopiesAndExactCopiesAreAllFound) {
    constexpr int DIMENSION = 64;
    std::mt19937 random{20261005};
    std::normal_distribution<double> normal{0.0, 1.0};
    const auto unit = [&](std::vector<double> v) {
        double norm = 0;
        for (const auto x : v) {
            norm += x * x;
        }
        for (auto& x : v) {
            x /= std::sqrt(norm);
        }
        return v;
    };
    const auto draw = [&] {
        std::vector<double> v(DIMENSION);
        for (auto& x : v) {
            x = normal(random);
        }
        return unit(v);
    };
    // Une ligne : son vecteur, et le numéro de son groupe exact (-1 si seule).
    std::vector<std::pair<std::vector<double>, int>> rows;
    for (auto i = 0; i < 2000; i++) {
        rows.emplace_back(draw(), -1);
    }
    for (auto cluster = 0; cluster < 40; cluster++) {
        const auto center = draw();
        for (auto member = 0; member < 10; member++) {
            // Un écart de norme ~4,5e-3 : une distance cosinus de l'ordre de 1e-5.
            auto v = center;
            for (auto& x : v) {
                x += normal(random) * 4.5e-3 / std::sqrt(DIMENSION);
            }
            rows.emplace_back(unit(v), -1);
        }
    }
    auto group = 0;
    std::vector<int> sizes;
    for (auto size = 2; size <= 9; size++) {
        sizes.insert(sizes.end(), {size, size, size});
    }
    sizes.push_back(49);
    for (const auto size : sizes) {
        const auto v = draw();
        for (auto copy = 0; copy < size; copy++) {
            rows.emplace_back(v, group);
        }
        group++;
    }
    std::shuffle(rows.begin(), rows.end(), random);
    std::vector<std::string> texts;
    for (const auto& [v, g] : rows) {
        std::string text = "[";
        for (auto d = 0; d < DIMENSION; d++) {
            text += (d ? "," : "") + stringFormat("{}", static_cast<float>(v[d]));
        }
        texts.push_back(text + "]");
    }
    const auto numRows = static_cast<int64_t>(rows.size());
    const auto half = numRows / 2;
    // Le contrôle commun (check) cherche la ligne première à égalité près parmi trente, ce que
    // le groupe de 49 ne permet pas : ce témoin a le sien.
    const auto checkCopies = [&]() -> std::string {
        std::set<int64_t> reached;
        auto exhaustive = conn->query(stringFormat(
            "CALL QUERY_VECTOR_INDEX('{}', 'doc_index', {}, {}, efs := {}) RETURN node.id;",
            table, texts[0], numRows, numRows));
        if (!exhaustive->isSuccess()) {
            return exhaustive->getErrorMessage();
        }
        while (exhaustive->hasNext()) {
            reached.insert(exhaustive->getNext()->getValue(0)->getValue<int64_t>());
        }
        int64_t notFound = 0;
        std::string firstMisses;
        for (auto i = 0; i < numRows; i++) {
            auto own = conn->query(stringFormat(
                "CALL QUERY_VECTOR_INDEX('{}', 'doc_index', {}, 10) RETURN node.id, distance;",
                table, texts[i]));
            auto found = false;
            while (own->isSuccess() && own->hasNext()) {
                auto tuple = own->getNext();
                const auto id = tuple->getValue(0)->getValue<int64_t>();
                found |= id == i || (rows[i].second >= 0 && rows[id].second == rows[i].second &&
                                        std::stod(tuple->getValue(1)->toString()) < 1e-6);
            }
            if (!found && ++notFound <= 5) {
                firstMisses += stringFormat(" {} (group {})", i, rows[i].second);
            }
        }
        if (reached.size() == static_cast<size_t>(numRows) && notFound == 0) {
            return "";
        }
        return stringFormat("{}/{} reachable, {} not found by their own vector;{}",
            reached.size(), numRows, notFound, firstMisses);
    };
    std::string failures;
    const auto runs = forcedRuns > 0 ? forcedRuns : 3;
    for (auto run = 0; run < runs; run++) {
        table = stringFormat("Copies{}", run);
        mustRun(stringFormat("CREATE NODE TABLE {}(id INT64 PRIMARY KEY, vec FLOAT[{}]);", table,
            DIMENSION));
        const auto csv = databasePath + stringFormat(".copies{}.csv", run);
        {
            std::ofstream out(csv);
            for (auto i = 0; i < half; i++) {
                out << i << ",\"" << texts[i] << "\"\n";
            }
        }
        mustRun(stringFormat("COPY {} FROM '{}' (header=false);", table, csv));
        std::filesystem::remove(csv);
        mustRun(stringFormat(
            "CALL CREATE_VECTOR_INDEX('{}', 'doc_index', 'vec', metric := 'cosine');", table));
        for (auto first = half; first < numRows; first += 64) {
            std::string items;
            for (auto i = first; i < std::min(numRows, first + 64); i++) {
                items +=
                    (items.empty() ? "" : ", ") + stringFormat("{id: {}, v: {}}", i, texts[i]);
            }
            mustRun(stringFormat("UNWIND [{}] AS item CREATE (:{} {id: item.id, vec: item.v});",
                items, table));
        }
        if (HasFatalFailure()) {
            return;
        }
        const auto failure = checkCopies();
        std::cerr << "  run " << run << ": " << (failure.empty() ? "all found" : failure) << "\n";
        if (!failure.empty()) {
            failures += stringFormat(" run {}: {}", run, failure);
        }
    }
    EXPECT_TRUE(failures.empty()) << "[check: copies-all-found]" << failures;
}

// La recette de l'arbre principal (10 octobre) : une note d'embedder.rs, vue une fois le 25 août
// 2026, disait que l'index HNSW plantait (shrinkForNode → computeDistance) sur quelques centaines
// de vecteurs identiques — 1 402 vecteurs NULS du MockEmbedder. Rejouée sur le moteur d'aujourd'hui,
// dans un fils (un plantage n'emporte pas la passe) : le même vecteur pour toutes les lignes, nul
// (une norme nulle : le cosinus divise par zéro) ou non nul (des doublons seuls) ; l'index créé
// avant ou après les lignes ; les lignes une à une ou par COPY. L'attendu : ni plantage ni erreur,
// la recherche rend, et chaque ligne est joignable par la recherche exhaustive.
struct SameVectorCase {
    bool zero;
    bool indexFirst;
    bool copy;
};

class SameVectorForEveryRow : public VectorIndexUpdate,
                              public ::testing::WithParamInterface<SameVectorCase> {};

TEST_P(SameVectorForEveryRow, NeitherCrashesNorLosesRows) {
    constexpr int NUM_ROWS = 1402;
    constexpr int DIMENSION = 384;
    const auto [zero, indexFirst, copy] = GetParam();
    std::string vector = "[";
    for (auto d = 0; d < DIMENSION; d++) {
        vector += (d ? "," : "") + std::string(zero ? "0" : (d % 3 == 0 ? "0.5" : "-0.25"));
    }
    vector += "]";
    const auto csv = databasePath + ".same.csv";
    if (copy) {
        std::ofstream out(csv);
        for (auto i = 0; i < NUM_ROWS; i++) {
            out << i << ",\"" << vector << "\"\n";
        }
    }
    const auto report = databasePath + ".same.report";
    conn.reset();
    database.reset();
    const auto pid = fork();
    if (pid == 0) {
        concurrency::disableCoreDumps();
        alarm(300);
        int code = 0;
        try {
            rag3db::main::Database childDatabase(databasePath, *systemConfig);
            rag3db::main::Connection connection(&childDatabase);
            concurrency::loadVectorExtension(connection);
            const auto run = [&](const std::string& query) {
                auto result = connection.query(query);
                if (!result->isSuccess()) {
                    std::ofstream(report) << "query failed: " << query.substr(0, 120) << ": "
                                          << result->getErrorMessage();
                    _exit(4);
                }
                return result;
            };
            run(stringFormat(
                "CREATE NODE TABLE T(id INT64, v FLOAT[{}], PRIMARY KEY(id));", DIMENSION));
            const auto createIndex = [&] {
                run("CALL CREATE_VECTOR_INDEX('T', 'idx', 'v', metric := 'cosine');");
            };
            if (indexFirst) {
                createIndex();
            }
            if (copy) {
                run("COPY T FROM '" + csv + "' (header=false);");
            } else {
                for (auto i = 0; i < NUM_ROWS; i++) {
                    run(stringFormat("CREATE (:T {id: {}, v: {}});", i, vector));
                }
            }
            if (!indexFirst) {
                createIndex();
            }
            auto top = run("CALL QUERY_VECTOR_INDEX('T', 'idx', " + vector +
                           ", 10) RETURN node.id;");
            const auto numTop = top->getNumTuples();
            auto all = run(stringFormat(
                "CALL QUERY_VECTOR_INDEX('T', 'idx', {}, {}, efs := {}) RETURN node.id;", vector,
                NUM_ROWS, NUM_ROWS));
            std::set<int64_t> reached;
            while (all->hasNext()) {
                reached.insert(all->getNext()->getValue(0)->getValue<int64_t>());
            }
            std::ofstream(report) << "top " << numTop << ", reached " << reached.size() << "/"
                                  << NUM_ROWS;
            code = numTop == 10 && reached.size() == NUM_ROWS ? 0 : 5;
        } catch (const std::exception& e) {
            std::ofstream(report) << "exception: " << e.what();
            code = 3;
        }
        _exit(code);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    std::string detail;
    if (std::filesystem::exists(report)) {
        std::ifstream in(report);
        std::getline(in, detail);
    }
    std::cerr << "  " << (zero ? "zero" : "same") << (indexFirst ? ", index first" : ", rows first")
              << (copy ? ", COPY" : ", line by line") << ": " << detail << "\n";
    EXPECT_FALSE(WIFSIGNALED(status))
        << "[check: same-vector-no-crash] killed by signal " << WTERMSIG(status) << "; " << detail;
    if (WIFEXITED(status)) {
        EXPECT_EQ(WEXITSTATUS(status), 0) << "[check: same-vector-all-reachable] " << detail;
    }
    std::filesystem::remove(csv);
    std::filesystem::remove(report);
}

INSTANTIATE_TEST_SUITE_P(Cases, SameVectorForEveryRow,
    ::testing::Values(SameVectorCase{true, true, false}, SameVectorCase{true, true, true},
        SameVectorCase{true, false, false}, SameVectorCase{true, false, true},
        SameVectorCase{false, true, false}, SameVectorCase{false, true, true},
        SameVectorCase{false, false, false}, SameVectorCase{false, false, true}),
    [](const ::testing::TestParamInfo<SameVectorCase>& info) {
        return std::string(info.param.zero ? "Zero" : "Same") +
               (info.param.indexFirst ? "IndexFirst" : "RowsFirst") +
               (info.param.copy ? "Copy" : "LineByLine");
    });

class RealVectorsReachability : public VectorIndexUpdate,
                                public ::testing::WithParamInterface<std::string> {};

TEST_P(RealVectorsReachability, RowsNotFoundByTheirOwnVector) {
    const std::string source = std::string(std::getenv("HOME")) +
                               "/.cache/rag3weaver-build/joignabilite-export/Scope_Chunk.csv";
    if (!std::filesystem::exists(source)) {
        GTEST_SKIP() << "no export";
    }
    std::vector<std::string> uuids;
    std::unordered_map<std::string, std::string> vectors;
    const auto withVectors = databasePath + ".with.csv";
    const auto withoutVectors = databasePath + ".without.csv";
    {
        std::ifstream in(source);
        std::ofstream with(withVectors);
        std::ofstream without(withoutVectors);
        std::string line;
        while (std::getline(in, line)) {
            const auto first = line.find(',');
            const auto second = line.find(',', first + 1);
            const auto uuid = line.substr(0, first);
            const auto vector = "[" + line.substr(second + 1) + "]";
            uuids.push_back(uuid);
            vectors[uuid] = vector;
            with << uuid << ",\"" << vector << "\"\n";
            without << uuid << "\n";
        }
    }
    table = "T";
    mustRun("CREATE NODE TABLE T(uuid STRING PRIMARY KEY, claim STRING, emb FLOAT[768]);");
    const auto start = std::chrono::steady_clock::now();
    if (GetParam() == "Masse") {
        mustRun("COPY T(uuid, emb) FROM '" + withVectors + "' (header=false);");
    } else {
        mustRun("COPY T(uuid) FROM '" + withoutVectors + "' (header=false);");
        while (true) {
            auto claim = conn->query("MATCH (n:T) WHERE n.claim IS NULL WITH n LIMIT 512 SET "
                                     "n.claim = 'c' RETURN n.uuid;");
            ASSERT_TRUE(claim->isSuccess()) << claim->getErrorMessage();
            std::vector<std::string> claimed;
            while (claim->hasNext()) {
                claimed.push_back(claim->getNext()->getValue(0)->toString());
            }
            if (claimed.empty()) {
                break;
            }
            for (auto first = 0u; first < claimed.size(); first += 32) {
                std::string items;
                for (auto i = first; i < std::min<size_t>(claimed.size(), first + 32); i++) {
                    items += (items.empty() ? "" : ", ") + std::string("{u: '") + claimed[i] +
                             "', v: " + vectors[claimed[i]] + "}";
                }
                mustRun("UNWIND [" + items +
                        "] AS item MATCH (n:T {uuid: item.u}) SET n.emb = item.v;");
            }
        }
    }
    const auto indexStart = std::chrono::steady_clock::now();
    mustRun("CALL CREATE_VECTOR_INDEX('T', 'idx', 'emb', metric := 'cosine');");
    const auto built = std::chrono::steady_clock::now();
    std::cerr << "  " << GetParam() << ": index built in "
              << std::chrono::duration_cast<std::chrono::milliseconds>(built - indexStart).count()
              << " ms\n";
    int64_t missed = 0;
    std::string firstMisses;
    std::vector<std::string> missedUuids;
    for (const auto& uuid : uuids) {
        // Retrouvée : parmi les dix, ou la recherche atteint un vecteur identique au sien (des
        // groupes de vecteurs identiques, jusqu'à 49, ne tiennent pas dans dix).
        auto result = conn->query("CALL QUERY_VECTOR_INDEX('T', 'idx', " + vectors[uuid] +
                                  ", 10) RETURN node.uuid, distance;");
        auto found = false;
        while (result->isSuccess() && result->hasNext()) {
            auto tuple = result->getNext();
            found |= tuple->getValue(0)->toString() == uuid ||
                     std::stod(tuple->getValue(1)->toString()) < 1e-6;
        }
        if (!found) {
            missedUuids.push_back(uuid);
        }
        if (!found && ++missed <= 5) {
            firstMisses += " " + uuid;
        }
    }
    std::cerr << "  " << GetParam() << ": " << missed << " of " << uuids.size()
              << " not found by their own vector (loaded and indexed in "
              << std::chrono::duration_cast<std::chrono::seconds>(built - start).count()
              << " s);" << firstMisses << "\n";
    {
        // Le rappel@10 moyen sur 300 requêtes tirées d'une graine fixe, contre la force brute
        // (cosinus en double) : un rendu compte s'il est à une distance au plus égale à la
        // dixième vraie, à 1e-6 près (les copies exactes sont interchangeables).
        std::vector<std::vector<float>> parsed;
        parsed.reserve(uuids.size());
        for (const auto& uuid : uuids) {
            std::vector<float> v;
            const auto& text = vectors[uuid];
            const char* p = text.c_str() + 1;
            char* end = nullptr;
            while (true) {
                v.push_back(std::strtof(p, &end));
                if (*end != ',') {
                    break;
                }
                p = end + 1;
            }
            parsed.push_back(std::move(v));
        }
        const auto cosine = [](const std::vector<float>& a, const std::vector<float>& b) {
            double ab = 0, a2 = 0, b2 = 0;
            for (auto d = 0u; d < a.size(); d++) {
                ab += double(a[d]) * b[d];
                a2 += double(a[d]) * a[d];
                b2 += double(b[d]) * b[d];
            }
            return 1 - ab / std::sqrt(a2 * b2);
        };
        std::unordered_map<std::string, size_t> index;
        for (auto i = 0u; i < uuids.size(); i++) {
            index[uuids[i]] = i;
        }
        std::mt19937 random{20261005};
        double recallSum = 0;
        constexpr int QUERIES = 300;
        for (auto q = 0; q < QUERIES; q++) {
            const auto row = random() % uuids.size();
            std::vector<double> dists;
            dists.reserve(uuids.size());
            for (const auto& v : parsed) {
                dists.push_back(cosine(parsed[row], v));
            }
            auto sorted = dists;
            std::nth_element(sorted.begin(), sorted.begin() + 9, sorted.end());
            const auto tenth = sorted[9];
            auto result = conn->query("CALL QUERY_VECTOR_INDEX('T', 'idx', " +
                                      vectors[uuids[row]] + ", 10) RETURN node.uuid;");
            int hits = 0;
            while (result->isSuccess() && result->hasNext()) {
                hits += dists[index[result->getNext()->getValue(0)->toString()]] <= tenth + 1e-6;
            }
            recallSum += std::min(hits, 10) / 10.0;
        }
        const auto numEdges = lowerGraphEdgeCount(*conn, "T", "idx");
        std::cerr << "  " << GetParam() << ": recall@10 " << recallSum / QUERIES
                  << " over " << QUERIES << " queries; lower degree "
                  << (numEdges < 0 ? -1.0 : double(numEdges) / uuids.size()) << "\n";
    }
    if (std::getenv("BANC_EXHAUSTIF") && !missedUuids.empty()) {
        // Sonde : un introuvable est-il hors du graphe (îlot) ou seulement hors du chemin
        // de la recherche ? Une recherche exhaustive depuis le premier vecteur le dit.
        std::set<std::string> reached;
        auto all = conn->query("CALL QUERY_VECTOR_INDEX('T', 'idx', " + vectors[uuids.front()] +
                               ", " + std::to_string(uuids.size()) + ", efs := " +
                               std::to_string(uuids.size()) + ") RETURN node.uuid;");
        while (all->isSuccess() && all->hasNext()) {
            reached.insert(all->getNext()->getValue(0)->toString());
        }
        int64_t inGraph = 0;
        for (const auto& uuid : missedUuids) {
            inGraph += reached.contains(uuid);
        }
        std::cerr << "  exhaustive: " << reached.size() << "/" << uuids.size()
                  << " reached; of the " << missedUuids.size() << " misses, " << inGraph
                  << " are reached by the exhaustive search\n";
        for (const auto& uuid : missedUuids) {
            auto wide = conn->query("CALL QUERY_VECTOR_INDEX('T', 'idx', " + vectors[uuid] +
                                    ", 10, efs := 2000) RETURN node.uuid, distance;");
            auto found = false;
            while (wide->isSuccess() && wide->hasNext()) {
                auto tuple = wide->getNext();
                found |= tuple->getValue(0)->toString() == uuid ||
                         std::stod(tuple->getValue(1)->toString()) < 1e-6;
            }
            std::cerr << "    " << uuid << (reached.contains(uuid) ? " in graph" : " ISLAND")
                      << (found ? ", found at efs 2000" : ", not found at efs 2000") << "\n";
        }
    }
    if (const char* dir = std::getenv("BANC_MANQUES")) {
        static int pass = 0;
        std::ofstream out(std::string(dir) + "/" + GetParam() + "-" + std::to_string(++pass) +
                          ".txt");
        for (const auto& uuid : missedUuids) {
            out << uuid << "\n";
        }
    }
    std::filesystem::remove(withVectors);
    std::filesystem::remove(withoutVectors);
}

INSTANTIATE_TEST_SUITE_P(Paths, RealVectorsReachability, ::testing::Values("Masse", "Fond"),
    [](const ::testing::TestParamInfo<std::string>& info) { return info.param; });

} // namespace

#endif
