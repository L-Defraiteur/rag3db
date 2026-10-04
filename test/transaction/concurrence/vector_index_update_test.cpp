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

#include <cmath>
#include <cstdlib>
#include <iostream>
#include <map>
#include <set>

#include "bench_harness.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "processor/result/flat_tuple.h"

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

// Lourd : 25 s avant c8fdaf196, 27 min après (la mise à jour s'est beaucoup ralentie,
// signalé au cœur C++ le 4 octobre). Ne tourne que sur demande, CONCURRENCE_VECTOR_HEAVY=1 ;
// rouge à chaque passe avant comme après.
TEST_F(VectorIndexUpdate, TenThousandRowsInBatchesOf512) {
    if (!std::getenv("CONCURRENCE_VECTOR_HEAVY")) {
        GTEST_SKIP() << "heavy: set CONCURRENCE_VECTOR_HEAVY=1";
    }
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

} // namespace

#endif
