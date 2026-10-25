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
#include <atomic>
#include <mutex>
#include <thread>
#include <future>
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
            lastCheckSummary = label + " : the exhaustive search failed: " +
                               exhaustive->getErrorMessage();
            std::cerr << lastCheckSummary << "\n";
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
        lastCheckSummary = stringFormat(
            "{} : {}/{} reachable by the exhaustive search, {}/{} find themselves by their own "
            "vector, {} dead rows returned",
            label, reached.size(), numRows, foundFirst, asked, dead);
        std::cerr << lastCheckSummary << "\n";
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
        std::string details;
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
            if (!runFailed.empty()) {
                details += "\n  " + lastCheckSummary;
            }
            if (checkSecondIndex) {
                const auto second =
                    check(stringFormat("run {}, second index", run), ownStep, "doc_index2", "vec2");
                if (!second.empty()) {
                    details += "\n  " + lastCheckSummary;
                }
                runFailed.insert(second.begin(), second.end());
            }
            failedRuns += !runFailed.empty();
            failed.insert(runFailed.begin(), runFailed.end());
        }
        std::string checks;
        for (const auto& name : failed) {
            checks += "[check: " + name + "] ";
        }
        // Le détail des essais perdants entre dans le message d'échec, donc dans le JSON de la
        // comparaison : un rouge probabiliste dit de lui-même si la ligne manque à la recherche
        // exhaustive (hors du graphe) ou seulement à la recherche de son propre vecteur.
        EXPECT_TRUE(failed.empty()) << checks << failedRuns << " of " << runs
                                    << " runs lose rows in the vector index:" << details;
    }

    // Ce qu'une autre connexion a validé pendant un CREATE_VECTOR_INDEX, avant qu'il rende la main.
    struct WritesDuringCreate {
        int64_t inserted = 0;
        int64_t deleted = 0;
        // Déplacées puis supprimées dans la même fenêtre : l'absence l'emporte.
        int64_t movedThenDeleted = 0;
        // Les lignes dont le vecteur a changé, avec leur nouveau vecteur.
        std::vector<std::pair<int64_t, std::string>> updated;
    };

    // Le vecteur de dimension 16 d'une ligne insérée pendant le bâti, et un vecteur lointain,
    // propre à chaque j, pour les SET.
    static std::string createRowVector(int64_t i) {
        return stringFormat("[{} % 17, {} % 23, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, {}]", i,
            i, i / 1000.0);
    }
    static std::string movedRowVector(int64_t j) {
        return stringFormat("[{}, {} % 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1]", 500 + j, j);
    }

    // La table C de numRows lignes, puis c_index bâti par une connexion pendant qu'une autre
    // valide en boucle, une instruction par écriture : une insertion et, si withChanges, la
    // suppression d'une ligne d'origine paire et le SET du vecteur d'une impaire (une sur cinq
    // ensuite supprimée elle aussi).
    // La fenêtre du bâti dépend de la charge du poste : un essai dont le CREATE a fini avant
    // qu'une écriture de chaque genre ait validé recommence sur une table neuve, jusqu'à cinq
    // fois. Un témoin ne passe jamais sur une mise en place vide, et n'échoue pas sur une mise en
    // place ratée. Un CREATE en échec n'est pas une mise en place ratée : il ne recommence pas.
    WritesDuringCreate createIndexUnderWrites(int64_t numRows, bool withChanges) {
        static constexpr int MAX_ATTEMPTS = 5;
        WritesDuringCreate writes;
        for (int attempt = 0; attempt < MAX_ATTEMPTS; attempt++) {
            if (attempt > 0) {
                std::cerr << "  setup attempt " << attempt << " again: " << writes.inserted
                          << " inserted, " << writes.deleted << " deleted, "
                          << writes.updated.size() << " moved, " << writes.movedThenDeleted
                          << " moved then deleted during the build\n";
                mustRun("CALL DROP_VECTOR_INDEX('C', 'c_index');");
                mustRun("DROP TABLE C;");
            }
            std::string outcome;
            writes = createIndexUnderWritesOnce(numRows, withChanges, outcome);
            if (outcome != "ok") {
                ADD_FAILURE() << "[check: setup] " << outcome;
                return writes;
            }
            const auto complete = writes.inserted > 0 &&
                                  (!withChanges || (writes.deleted > 0 && !writes.updated.empty() &&
                                                       writes.movedThenDeleted > 0));
            if (complete) {
                return writes;
            }
        }
        ADD_FAILURE() << "[check: setup] " << MAX_ATTEMPTS
                      << " attempts, the CREATE always ended before a write of each kind";
        return writes;
    }

    WritesDuringCreate createIndexUnderWritesOnce(int64_t numRows, bool withChanges,
        std::string& outcome) {
        WritesDuringCreate writes;
        const std::string vec = "[i % 17, i % 23, i % 29, i % 31, i % 37, i % 41, i % 43, i % 47, "
                                "i % 53, i % 59, i % 61, i % 67, i % 71, i % 73, i % 79, i / 1000.0]";
        table = "C";
        mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY, vec FLOAT[16]);");
        mustRun(stringFormat("UNWIND range(0, {}) AS i CREATE (:C {id: i, vec: {}});",
            numRows - 1, vec));
        rag3db::main::Connection creator(database.get());
        rag3db::main::Connection writer(database.get());
        std::atomic<bool> created{false};
        auto creating = std::async(std::launch::async, [&] {
            auto result =
                creator.query("CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');");
            created = true;
            return result->isSuccess() ? std::string("ok") : result->getErrorMessage();
        });
        for (int64_t j = 0; !created.load() && j < 100000; j++) {
            const auto i = 100000 + j;
            auto inserted = writer.query(
                stringFormat("CREATE (:C {id: {}, vec: {}});", i, createRowVector(i)));
            if (inserted->isSuccess() && !created.load()) {
                writes.inserted++;
            }
            if (!withChanges || 2 * j + 1 >= numRows) {
                continue;
            }
            auto deleted = writer.query(stringFormat("MATCH (n:C {id: {}}) DELETE n;", 2 * j));
            if (deleted->isSuccess() && !created.load()) {
                writes.deleted++;
            }
            const auto moved = movedRowVector(j);
            auto updated = writer.query(
                stringFormat("MATCH (n:C {id: {}}) SET n.vec = {};", 2 * j + 1, moved));
            if (!updated->isSuccess() || created.load()) {
                continue;
            }
            if (j % 5 == 4) {
                auto gone =
                    writer.query(stringFormat("MATCH (n:C {id: {}}) DELETE n;", 2 * j + 1));
                // Supprimée, la ligne n'est plus à chercher, même si sa suppression a validé
                // après la fin du CREATE : elle restait attendue à son nouveau vecteur, et la
                // recherche courte « ne la trouvait pas » (139, 149, 169, 179, toutes d'indice
                // j % 5 = 4, introuvables même à efs 2000).
                if (gone->isSuccess()) {
                    if (!created.load()) {
                        writes.movedThenDeleted++;
                    }
                    continue;
                }
            }
            writes.updated.emplace_back(2 * j + 1, moved);
        }
        outcome = creating.get();
        return writes;
    }

    // Toutes les lignes vivantes sont joignables par une recherche exhaustive, aucune ligne
    // supprimée n'est rendue, et chaque ligne déplacée pendant le bâti se retrouve première à son
    // nouveau vecteur. Rend les étiquettes en défaut.
    std::set<std::string> checkAfterCreateUnderWrites(const WritesDuringCreate& writes) {
        std::set<std::string> failed;
        std::set<int64_t> alive;
        auto scan = conn->query("MATCH (n:C) WHERE n.vec IS NOT NULL RETURN n.id;");
        while (scan->hasNext()) {
            alive.insert(scan->getNext()->getValue(0)->getValue<int64_t>());
        }
        const auto numLive = static_cast<int64_t>(alive.size());
        const std::string origin = "[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]";
        auto all = conn->query(stringFormat(
            "CALL QUERY_VECTOR_INDEX('C', 'c_index', {}, {}, efs := {}) RETURN node.id;", origin,
            numLive, numLive));
        if (!all->isSuccess()) {
            ADD_FAILURE() << "[check: query] " << all->getErrorMessage();
            failed.insert("query");
            return failed;
        }
        std::set<int64_t> reached;
        int64_t dead = 0;
        int64_t returned = 0;
        while (all->hasNext()) {
            const auto id = all->getNext()->getValue(0)->getValue<int64_t>();
            returned++;
            if (alive.contains(id)) {
                reached.insert(id);
            } else {
                dead++;
            }
        }
        const auto duplicates = returned - dead - static_cast<int64_t>(reached.size());
        // Une recherche courte : exhaustive, elle relit le vecteur courant de chaque ligne
        // atteinte et retrouve une ligne déplacée même restée à sa place d'avant dans le graphe.
        // Les lignes déplacées forment un nuage lointain et régulier : à sa vraie place, chacune
        // se trouve en 64 candidats.
        int64_t movedFound = 0;
        for (const auto& [id, moved] : writes.updated) {
            auto own = conn->query(stringFormat(
                "CALL QUERY_VECTOR_INDEX('C', 'c_index', {}, 1, efs := 64) RETURN node.id;",
                moved));
            int64_t got = -1;
            if (own->isSuccess() && own->hasNext()) {
                got = own->getNext()->getValue(0)->getValue<int64_t>();
            }
            if (got == id) {
                movedFound++;
                continue;
            }
            // Ce qu'un manque rend, et si une recherche large retrouve la ligne : un défaut de
            // graphe se distingue ainsi d'une ligne qui n'est plus là.
            auto wide = conn->query(stringFormat(
                "CALL QUERY_VECTOR_INDEX('C', 'c_index', {}, 1, efs := 2000) RETURN node.id, "
                "distance;",
                moved));
            std::cerr << "  moved row " << id << " not found first at " << moved << ": got " << got
                      << ", at efs 2000 "
                      << (wide->isSuccess() && wide->hasNext() ? wide->getNext()->toString() :
                                                                 std::string("nothing"))
                      << "\n";
        }
        std::cerr << "  create under writes: " << writes.inserted << " inserted, "
                  << writes.deleted << " deleted, " << writes.updated.size() << " moved, "
                  << writes.movedThenDeleted << " moved then deleted during the build; "
                  << reached.size() << "/" << numLive << " reached, " << dead
                  << " dead returned, " << duplicates << " returned twice, " << movedFound << "/" << writes.updated.size()
                  << " moved rows found at their new vector\n";
        if (static_cast<int64_t>(reached.size()) != numLive) {
            failed.insert("rows-committed-during-create-indexed");
        }
        if (dead != 0) {
            failed.insert("vector-search-no-dead-row");
        }
        if (duplicates != 0) {
            failed.insert("vector-search-no-duplicate-row");
        }
        if (movedFound != static_cast<int64_t>(writes.updated.size())) {
            failed.insert("rows-moved-during-create-found");
        }
        return failed;
    }

    static std::string checksOf(const std::set<std::string>& failed) {
        std::string checks;
        for (const auto& name : failed) {
            checks += "[check: " + name + "] ";
        }
        return checks;
    }

    int forcedRuns = 0;
    bool checkSecondIndex = false;
    // Le compte du dernier contrôle (check), pour le message d'un essai perdant.
    std::string lastCheckSummary;
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

// Le CALL de la recherche ne rend aucun nœud supprimé (ticket
// 2026-10-11-query-vector-index-rend-des-noeuds-supprimes-au-call) : relevé par le cœur C++ sur
// I1, cent lignes, deux supprimées, k = 100 rendait 100 lignes au CALL et 98 clés projetées. Les
// autres témoins ne le voient pas : ils projettent le nœud, et la projection écarte les lignes
// supprimées. Deux moments : après la suppression validée, et dans la transaction qui supprime.
TEST_F(VectorIndexUpdate, DeletedRowsAreNotReturnedByTheCall) {
    const auto count = [&](const std::string& query) -> int64_t {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << "[check: query] " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    };
    createDocs(0, 100);
    const auto callRows = [&]() {
        return count(stringFormat(
            "CALL QUERY_VECTOR_INDEX('{}', 'doc_index', [0, 0, 0, 0], 100, efs := 100) "
            "RETURN count(*);",
            table));
    };
    const auto projectedKeys = [&]() {
        return count(stringFormat(
            "CALL QUERY_VECTOR_INDEX('{}', 'doc_index', [0, 0, 0, 0], 100, efs := 100) "
            "RETURN count(node.id);",
            table));
    };
    mustRun("BEGIN TRANSACTION;");
    mustRun(stringFormat("MATCH (n:{}) WHERE n.id IN [10, 11] DELETE n;", table));
    const auto callInTransaction = callRows();
    const auto keysInTransaction = projectedKeys();
    mustRun("COMMIT;");
    const auto callAfterCommit = callRows();
    const auto keysAfterCommit = projectedKeys();
    EXPECT_TRUE(callInTransaction == 98 && callAfterCommit == 98)
        << "[check: vector-call-dead] the CALL returns " << callInTransaction
        << " rows in the deleting transaction (" << keysInTransaction << " keys) and "
        << callAfterCommit << " after its commit (" << keysAfterCommit
        << " keys), for 98 living rows";
}

// Une ligne créée puis mise à jour dans la même transaction (page 04, §7). NodeTable::update
// appelait index->update sans tester si la ligne est locale : OnDiskHNSWIndex::update l'insérait
// dans le graphe, et le commit l'insérait de nouveau (commitInsert). Attendu : la ligne n'entre
// qu'une fois dans l'index — aucune arête en double dans le graphe, la recherche exhaustive rend
// chaque ligne une seule fois, chaque ligne se retrouve par son vecteur final — après le COMMIT
// et après réouverture. La recherche ne voyait pas le doublon : seul le graphe le montre.
// Le paramètre : la ligne est-elle mise à jour dans la transaction (le cas), ou créée directement
// avec son vecteur final (le contrôle, qui dit ce que vaut un graphe sain sur ces données) ?
class RowsCreatedInATransaction : public VectorIndexUpdate,
                                  public ::testing::WithParamInterface<bool> {};

TEST_P(RowsCreatedInATransaction, AreIndexedOnce) {
    const auto updated = GetParam();
    std::string failures;
    const auto runs = forcedRuns > 0 ? forcedRuns : 5;
    for (auto run = 0; run < runs; run++) {
        createDocs(run, 200);
        mustRun("BEGIN TRANSACTION;");
        if (updated) {
            mustRun(stringFormat(
                "UNWIND range(1000, 1009) AS i CREATE (:{} {id: i, vec: [9000, 9000, 9000, i]});",
                table));
            mustRun(stringFormat(
                "MATCH (n:{}) WHERE n.id >= 1000 SET n.vec = [500, 500, 500, n.id];", table));
        } else {
            mustRun(stringFormat(
                "UNWIND range(1000, 1009) AS i CREATE (:{} {id: i, vec: [500, 500, 500, i]});",
                table));
        }
        mustRun("COMMIT;");
        if (HasFatalFailure()) {
            return;
        }
        for (const auto* moment : {"after the commit", "after reopening"}) {
            if (std::string(moment) == "after reopening") {
                conn.reset();
                database.reset();
                createDBAndConn();
                mustRun("CALL auto_checkpoint=false;");
                concurrency::loadVectorExtension(*conn);
            }
            // La recherche exhaustive : chaque ligne une fois, aucune absente.
            std::map<int64_t, int> seen;
            auto all = conn->query(stringFormat(
                "CALL QUERY_VECTOR_INDEX('{}', 'doc_index', [500, 500, 500, 1005], 210, efs := "
                "210) RETURN node.id;",
                table));
            ASSERT_TRUE(all->isSuccess()) << "[check: query] " << all->getErrorMessage();
            while (all->hasNext()) {
                seen[all->getNext()->getValue(0)->getValue<int64_t>()]++;
            }
            int64_t twice = 0;
            for (const auto& [id, count] : seen) {
                twice += count > 1;
            }
            int64_t newRows = 0;
            for (auto id = 1000; id < 1010; id++) {
                newRows += seen.contains(id);
            }
            // Chaque ligne mise à jour, par son vecteur final.
            int64_t ownFound = 0;
            for (auto id = 1000; id < 1010; id++) {
                auto own = conn->query(stringFormat(
                    "CALL QUERY_VECTOR_INDEX('{}', 'doc_index', [500, 500, 500, {}], 1) RETURN "
                    "node.id;",
                    table, id));
                ASSERT_TRUE(own->isSuccess()) << "[check: query] " << own->getErrorMessage();
                ownFound += own->hasNext() && own->getNext()->getValue(0)->getValue<int64_t>() == id;
            }
            // Le graphe lui-même (sa couche basse, une table de relations cachée) : une ligne
            // insérée deux fois y laisse des arêtes en double, que la recherche ne montre pas
            // (10 octobre : 741 arêtes en double pour dix lignes mises à jour, 0 au contrôle).
            auto tables = conn->query("CALL SHOW_TABLES() RETURN id, name;");
            ASSERT_TRUE(tables->isSuccess()) << "[check: query] " << tables->getErrorMessage();
            std::string lower;
            while (tables->hasNext()) {
                auto tuple = tables->getNext();
                if (tuple->getValue(1)->toString() == table) {
                    lower = stringFormat("_{}_doc_index_LOWER", tuple->getValue(0)->toString());
                }
            }
            // Les tables du graphe ne sont visibles qu'avec le catalogue interne.
            mustRun("CALL ENABLE_INTERNAL_CATALOG=true;");
            const auto graphCount = [&](const std::string& query) -> int64_t {
                auto result = conn->query(query);
                EXPECT_TRUE(result->isSuccess()) << "[check: query] " << query << "\n"
                                                 << result->getErrorMessage();
                return result->isSuccess() && result->hasNext() ?
                           result->getNext()->getValue(0)->getValue<int64_t>() :
                           -1;
            };
            const auto duplicated = graphCount(stringFormat(
                "MATCH (a:{})-[r:{}]->(b:{}) WITH a.id AS s, b.id AS d, count(*) AS c WHERE c > 1 "
                "RETURN count(*);",
                table, lower, table));
            const auto toItself = graphCount(stringFormat(
                "MATCH (a:{})-[r:{}]->(b:{}) WHERE a.id = b.id RETURN count(*);", table, lower,
                table));
            mustRun("CALL ENABLE_INTERNAL_CATALOG=false;");
            const auto summary = stringFormat(
                "run {} {} : {} rows returned by the exhaustive search, {} returned twice, {}/10 "
                "rows of the transaction reachable, {}/10 found first by their final vector ; in "
                "the graph, {} duplicated edges, {} edges to a row itself",
                run, moment, seen.size(), twice, newRows, ownFound, duplicated, toItself);
            std::cerr << "  " << summary << "\n";
            if (twice != 0 || newRows != 10 || seen.size() != 210 || ownFound != 10 ||
                duplicated != 0 || toItself != 0) {
                failures += "\n  " + summary;
            }
        }
    }
    EXPECT_TRUE(failures.empty()) << "[check: updated-local-row-indexed-once]" << failures;
}

// Le pendant : des lignes de la transaction VERSÉES dans la table par un COPY (f1d8c7190) ne
// sont plus locales — elles sont dans l'index depuis le versement. Leur mise à jour doit donc
// l'atteindre : le correctif qui saute les lignes locales ne doit pas les sauter, sans quoi
// l'index garderait leur ancien vecteur (getMinUncommittedNodeOffset rend INVALID_OFFSET après
// un versement, la table locale vide).
TEST_F(VectorIndexUpdate, RowsFlushedByACopyThenUpdatedFollowTheirVector) {
    std::string failures;
    const auto runs = forcedRuns > 0 ? forcedRuns : 5;
    for (auto run = 0; run < runs; run++) {
        createDocs(run, 200);
        const auto csv = databasePath + stringFormat(".flushed{}.csv", run);
        {
            std::ofstream out(csv);
            for (auto id = 2000; id < 2005; id++) {
                out << id << ",\"[1,2,3," << id << "]\"\n";
            }
        }
        mustRun("BEGIN TRANSACTION;");
        mustRun(stringFormat(
            "UNWIND range(1000, 1009) AS i CREATE (:{} {id: i, vec: [9000, 9000, 9000, i]});",
            table));
        mustRun(stringFormat("COPY {}(id, vec) FROM '{}' (header=false);", table, csv));
        mustRun(stringFormat(
            "MATCH (n:{}) WHERE n.id >= 1000 AND n.id < 1010 SET n.vec = [500, 500, 500, n.id];",
            table));
        mustRun("COMMIT;");
        std::filesystem::remove(csv);
        if (HasFatalFailure()) {
            return;
        }
        int64_t ownFound = 0;
        for (auto id = 1000; id < 1010; id++) {
            auto own = conn->query(stringFormat(
                "CALL QUERY_VECTOR_INDEX('{}', 'doc_index', [500, 500, 500, {}], 1) RETURN "
                "node.id;",
                table, id));
            ASSERT_TRUE(own->isSuccess()) << "[check: query] " << own->getErrorMessage();
            ownFound += own->hasNext() && own->getNext()->getValue(0)->getValue<int64_t>() == id;
        }
        const auto summary =
            stringFormat("run {} : {}/10 flushed then updated rows found by their final vector",
                run, ownFound);
        std::cerr << "  " << summary << "\n";
        if (ownFound != 10) {
            failures += "\n  " + summary;
        }
    }
    EXPECT_TRUE(failures.empty()) << "[check: flushed-row-update-reaches-the-index]" << failures;
}

INSTANTIATE_TEST_SUITE_P(Kinds, RowsCreatedInATransaction, ::testing::Bool(),
    [](const ::testing::TestParamInfo<bool>& info) {
        return std::string(info.param ? "ThenUpdated" : "WithTheirFinalVector");
    });

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
    auto* found = nodeTable.getIndex(index, nullptr).value();
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
        nodeTable.getIndex(index, nullptr).value()->getStorageInfo());
    const auto count =
        static_cast<int64_t>(storage->getTable(info.lowerRelTableID)->getNumTotalRows(transaction));
    EXPECT_TRUE(connection.query("COMMIT;")->isSuccess());
    return count;
}

// Le point d'entrée de la couche haute de l'index, lu ou, avec newEntryPoint, écrit par les
// internes.
offset_t upperEntryPointOf(rag3db::main::Connection& connection, const std::string& table,
    const std::string& index, std::optional<offset_t> newEntryPoint = std::nullopt) {
    EXPECT_TRUE(connection.query("BEGIN TRANSACTION READ ONLY;")->isSuccess());
    auto* context = connection.getClientContext();
    auto* transaction = rag3db::transaction::Transaction::Get(*context);
    const auto* entry =
        rag3db::catalog::Catalog::Get(*context)->getTableCatalogEntry(transaction, table);
    auto& nodeTable = rag3db::storage::StorageManager::Get(*context)
                          ->getTable(entry->getTableID())
                          ->cast<rag3db::storage::NodeTable>();
    auto& info = static_cast<HNSWStorageInfoMirror&>(const_cast<rag3db::storage::IndexStorageInfo&>(
        nodeTable.getIndex(index, nullptr).value()->getStorageInfo()));
    if (newEntryPoint) {
        info.upperEntryPoint = *newEntryPoint;
    }
    const auto entryPoint = info.upperEntryPoint;
    EXPECT_TRUE(connection.query("COMMIT;")->isSuccess());
    return entryPoint;
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
        std::vector<int64_t> latencies;
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
            const auto queryStart = std::chrono::steady_clock::now();
            auto result = conn->query("CALL QUERY_VECTOR_INDEX('T', 'idx', " +
                                      vectors[uuids[row]] + ", 10) RETURN node.uuid;");
            latencies.push_back(std::chrono::duration_cast<std::chrono::microseconds>(
                std::chrono::steady_clock::now() - queryStart)
                                    .count());
            int hits = 0;
            while (result->isSuccess() && result->hasNext()) {
                hits += dists[index[result->getNext()->getValue(0)->toString()]] <= tenth + 1e-6;
            }
            recallSum += std::min(hits, 10) / 10.0;
        }
        const auto numEdges = lowerGraphEdgeCount(*conn, "T", "idx");
        // Le temps d'une requête (k = 10, efs par défaut), mesuré autour de conn->query : le
        // prix d'un degré plus grand. Puis la taille du fichier après un point de reprise : à
        // données égales, l'écart entre deux règles est celui de l'index.
        std::ranges::sort(latencies);
        mustRun("CHECKPOINT;");
        std::cerr << "  " << GetParam() << ": query latency median "
                  << latencies[latencies.size() / 2] << " us, p90 "
                  << latencies[latencies.size() * 9 / 10] << " us; database file "
                  << std::filesystem::file_size(databasePath) / 1024 << " KiB after a checkpoint\n";
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


// Sous le mode multi-écrivains : un SET de vecteur qui attend l'index derrière une insertion non
// validée, puis reprend après sa validation (ticket
// 2026-10-11-index-vectoriel-sous-ecrivains-simultanes-offset-hors-de-l-ensemble-visite). L'état de
// mise à jour de l'index a été dimensionné sur les lignes d'avant l'attente (300) ; l'instantané,
// repris après l'attente, voit les arêtes vers les 50 lignes de l'autre, au-delà : « node offset N
// is beyond the visited set of 300 nodes », trois fois sur trois (sonde du 11 octobre). Un DELETE
// ou un CREATE à la même place passe. Le relevé venait de H4_IndexedRandomMix, 10 sur 10.
TEST_F(VectorIndexUpdate, AnUpdateWaitingBehindAnInsertSeesItsNewRows) {
    createDocs(0, 300);
    rag3db::main::Connection holder(database.get());
    rag3db::main::Connection waiter(database.get());
    ASSERT_TRUE(waiter.query("CALL lock_timeout=20000;")->isSuccess());
    // L'autre écrivain tient l'index : 50 lignes neuves, tout près de celles que le SET touche.
    ASSERT_TRUE(holder.query("BEGIN TRANSACTION;")->isSuccess());
    const auto insert = holder.query(stringFormat(
        "UNWIND range(1000, 1049) AS i CREATE (n:{} {id: i}) SET n.vec = [i % 17, i % 23, i % 29, "
        "300 + (i - 1000) / 10.0];",
        table));
    ASSERT_TRUE(insert->isSuccess()) << insert->getErrorMessage();
    auto waiting = std::async(std::launch::async, [&] {
        auto result = waiter.query(stringFormat(
            "MATCH (n:{}) WHERE n.id = 5 SET n.vec = [5, 5, 5, 305];", table));
        return result->isSuccess() ? std::string("ok") : result->getErrorMessage();
    });
    std::this_thread::sleep_for(std::chrono::milliseconds(500));
    ASSERT_TRUE(holder.query("COMMIT;")->isSuccess());
    const auto outcome = waiting.get();
    EXPECT_EQ(outcome, "ok") << "[check: no-visited-set-refusal] " << outcome;
}

// Une recherche qui trouve moins de lignes vivantes que k (ici 11 pour 20) ne rend que celles-là.
// OnDiskHNSWIndex::search complétait le résultat jusqu'à k par des NodeWithDistance par défaut,
// des pseudo-lignes à INVALID_OFFSET : le CALL les rendait (le compte disait 20), et son masqueur
// les inscrivait ; le balayage sous masque les déréférençait (SIGSEGV dans lookupOffsets, relevé
// par le rejeu de hotfixs et réparé côté masque par le cœur C++, 12 octobre 2026).
TEST_F(VectorIndexUpdate, ASearchForMoreRowsThanLiveOnesReturnsOnlyTheLiveOnes) {
    createDocs(0, 30);
    mustRun(stringFormat("MATCH (n:{}) WHERE n.id >= 11 DELETE n;", table));
    auto result = conn->query(stringFormat(
        "CALL QUERY_VECTOR_INDEX('{}', 'doc_index', [1, 1, 1, 1], 20) RETURN count(*);", table));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_TRUE(result->hasNext());
    EXPECT_EQ(result->getNext()->getValue(0)->getValue<int64_t>(), 11)
        << "[check: no-padded-result] ";
}

// Des suppressions groupées dans une même région du graphe (relecture du cœur C++, 12 octobre
// 2026, sur la suppression sans recherche par voisin) : là, beaucoup de voisins tombent ensemble,
// et les ancrages qui dispensent de la recherche (un nœud qui n'a rien perdu) se raréfient. Des
// vecteurs groupés autour de trente centres ; l'index est créé avant les lignes (graphe non
// checkpointé, comme dans le produit). On retire les 10 % de lignes les plus proches d'un centre
// (une région de l'espace, pas une plage d'id), en trois transactions, ou toutes sauf cinq (le
// point d'entrée part aussi) ; directement, ou par SET NULL puis DELETE. Puis :
//  - toutes les lignes vivantes sont atteintes par une recherche exhaustive (efs = leur nombre) ;
//  - les dix plus proches d'une requête au bord de la région retirée sont ceux d'un calcul brut
//    (au moins 9 sur 10 : la recherche est approchée).
struct RegionDeleteCase {
    bool setNullFirst;
    bool keepOnlyAHandful;
};

class RegionDelete : public VectorIndexUpdate, public ::testing::WithParamInterface<RegionDeleteCase> {};

TEST_P(RegionDelete, SurvivorsStayReachableAndTheNearestAreRight) {
    constexpr int NUM_ROWS = 3000;
    constexpr int DIM = 16;
    constexpr int NUM_CENTERS = 30;
    std::mt19937 rng{20261012};
    std::uniform_real_distribution<float> unit{0.0f, 1.0f};
    std::normal_distribution<float> spread{0.0f, 0.05f};
    std::vector<std::vector<float>> centers(NUM_CENTERS, std::vector<float>(DIM));
    for (auto& c : centers) {
        for (auto& x : c) {
            x = unit(rng);
        }
    }
    std::vector<std::vector<float>> vecs(NUM_ROWS, std::vector<float>(DIM));
    for (auto i = 0; i < NUM_ROWS; i++) {
        const auto& c = centers[i % NUM_CENTERS];
        for (auto d = 0; d < DIM; d++) {
            vecs[i][d] = c[d] + spread(rng);
        }
    }
    const auto literal = [](const std::vector<float>& v) {
        std::string out = "[";
        for (auto d = 0u; d < v.size(); d++) {
            out += (d ? ", " : "") + stringFormat("{}", v[d]);
        }
        return out + "]";
    };
    const auto distance = [](const std::vector<float>& a, const std::vector<float>& b) {
        double sum = 0;
        for (auto d = 0u; d < a.size(); d++) {
            sum += (a[d] - b[d]) * (a[d] - b[d]);
        }
        return sum;
    };
    table = "R";
    mustRun(stringFormat("CREATE NODE TABLE R(id INT64 PRIMARY KEY, vec FLOAT[{}]);", DIM));
    mustRun("CALL CREATE_VECTOR_INDEX('R', 'r_index', 'vec', metric := 'l2');");
    for (auto first = 0; first < NUM_ROWS; first += 100) {
        std::string items;
        for (auto i = first; i < std::min(NUM_ROWS, first + 100); i++) {
            items += (items.empty() ? "" : ", ") + stringFormat("{i: {}, v: {}}", i, literal(vecs[i]));
        }
        mustRun("UNWIND [" + items + "] AS x CREATE (:R {id: x.i, vec: x.v});");
    }
    // La région : les plus proches du centre 0, par la distance.
    std::vector<int> byDistance(NUM_ROWS);
    for (auto i = 0; i < NUM_ROWS; i++) {
        byDistance[i] = i;
    }
    std::ranges::sort(byDistance, [&](int a, int b) {
        return distance(vecs[a], centers[0]) < distance(vecs[b], centers[0]);
    });
    const auto numDeleted = GetParam().keepOnlyAHandful ? NUM_ROWS - 5 : NUM_ROWS / 10;
    const std::vector<int> deleted(byDistance.begin(), byDistance.begin() + numDeleted);
    const std::set<int> deletedSet(deleted.begin(), deleted.end());
    const auto numParts = 3;
    for (auto part = 0; part < numParts; part++) {
        std::string ids;
        for (auto j = part; j < numDeleted; j += numParts) {
            ids += (ids.empty() ? "" : ", ") + std::to_string(deleted[j]);
        }
        if (GetParam().setNullFirst) {
            mustRun("MATCH (n:R) WHERE n.id IN [" + ids + "] SET n.vec = NULL;");
        }
        mustRun("MATCH (n:R) WHERE n.id IN [" + ids + "] DETACH DELETE n;");
    }
    std::vector<int> survivors;
    for (auto i = 0; i < NUM_ROWS; i++) {
        if (!deletedSet.contains(i)) {
            survivors.push_back(i);
        }
    }
    const auto numSurvivors = static_cast<int64_t>(survivors.size());
    // Toutes les vivantes, atteintes depuis la requête d'un survivant.
    auto all = conn->query(stringFormat(
        "CALL QUERY_VECTOR_INDEX('R', 'r_index', {}, {}, efs := {}) RETURN node.id;",
        literal(vecs[survivors[0]]), numSurvivors, numSurvivors));
    ASSERT_TRUE(all->isSuccess()) << all->getErrorMessage();
    std::set<int64_t> reached;
    while (all->hasNext()) {
        reached.insert(all->getNext()->getValue(0)->getValue<int64_t>());
    }
    EXPECT_EQ(static_cast<int64_t>(reached.size()), numSurvivors)
        << "[check: survivors-reachable] " << reached.size() << " of " << numSurvivors;
    // Les dix plus proches d'une requête au bord de la région : le centre 0 lui-même, dont tous les
    // proches sont retirés.
    const auto k = std::min<int64_t>(10, numSurvivors);
    std::vector<int> exact = survivors;
    std::ranges::sort(exact, [&](int a, int b) {
        return distance(vecs[a], centers[0]) < distance(vecs[b], centers[0]);
    });
    exact.resize(k);
    auto top = conn->query(stringFormat(
        "CALL QUERY_VECTOR_INDEX('R', 'r_index', {}, {}) RETURN node.id;", literal(centers[0]), k));
    ASSERT_TRUE(top->isSuccess()) << top->getErrorMessage();
    std::set<int64_t> found;
    while (top->hasNext()) {
        found.insert(top->getNext()->getValue(0)->getValue<int64_t>());
    }
    int64_t hits = 0;
    for (const auto id : exact) {
        hits += found.contains(id) ? 1 : 0;
    }
    std::cerr << "  region delete (" << (GetParam().setNullFirst ? "set null first" : "direct")
              << ", " << (GetParam().keepOnlyAHandful ? "all but five" : "10 %")
              << "): reached " << reached.size() << "/" << numSurvivors << ", nearest " << hits << "/"
              << k << "\n";
    EXPECT_GE(hits * 10, k * 9) << "[check: nearest-right] " << hits << " of " << k;
}

INSTANTIATE_TEST_SUITE_P(Cases, RegionDelete,
    ::testing::Values(RegionDeleteCase{false, false}, RegionDeleteCase{true, false},
        RegionDeleteCase{false, true}, RegionDeleteCase{true, true}),
    [](const ::testing::TestParamInfo<RegionDeleteCase>& info) {
        return std::string(info.param.setNullFirst ? "SetNullFirst" : "Direct") +
               (info.param.keepOnlyAHandful ? "AllButFive" : "TenPercent");
    });

// Les lignes validées pendant le bâti d'un CREATE_VECTOR_INDEX manquent à l'index (ticket
// 2026-10-12-les-lignes-validees-pendant-le-bati-d-un-create-manquent-a-l-index, relevé par le cœur
// C++ en relisant la rétention). Le CREATE bâtit l'index à son instantané, sans verrou d'index ;
// une autre connexion insère et valide des lignes pendant ce bâti ; à la fin, une recherche
// exhaustive (efs = nombre de lignes) doit atteindre toutes les lignes vivantes.
TEST_F(VectorIndexUpdate, RowsCommittedDuringACreateAreIndexed) {
    constexpr int NUM_ROWS = 20000;
    const std::string vec = "[i % 17, i % 23, i % 29, i % 31, i % 37, i % 41, i % 43, i % 47, "
                            "i % 53, i % 59, i % 61, i % 67, i % 71, i % 73, i % 79, i / 1000.0]";
    table = "C";
    mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY, vec FLOAT[16]);");
    mustRun(stringFormat("UNWIND range(0, {}) AS i CREATE (:C {id: i, vec: {}});", NUM_ROWS - 1,
        vec));
    rag3db::main::Connection creator(database.get());
    rag3db::main::Connection writer(database.get());
    std::atomic<bool> created{false};
    auto creating = std::async(std::launch::async, [&] {
        auto result = creator.query("CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');");
        created = true;
        return result->isSuccess() ? std::string("ok") : result->getErrorMessage();
    });
    int64_t insertedDuring = 0;
    for (auto j = 0; !created.load() && j < 100000; j++) {
        const auto i = 100000 + j;
        auto inserted = writer.query(stringFormat("CREATE (:C {id: {}, vec: {}});", i,
            stringFormat("[{} % 17, {} % 23, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, {}]", i, i,
                i / 1000.0)));
        if (inserted->isSuccess() && !created.load()) {
            insertedDuring++;
        }
    }
    const auto outcome = creating.get();
    ASSERT_EQ(outcome, "ok") << "[check: setup] " << outcome;
    ASSERT_GT(insertedDuring, 0) << "[check: setup] no insert committed during the CREATE";
    auto count = conn->query("MATCH (n:C) RETURN count(n);");
    ASSERT_TRUE(count->isSuccess());
    const auto numLive = count->getNext()->getValue(0)->getValue<int64_t>();
    auto all = conn->query(stringFormat(
        "CALL QUERY_VECTOR_INDEX('C', 'c_index', [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], "
        "{}, efs := {}) RETURN node.id;",
        numLive, numLive));
    ASSERT_TRUE(all->isSuccess()) << all->getErrorMessage();
    std::set<int64_t> reached;
    while (all->hasNext()) {
        reached.insert(all->getNext()->getValue(0)->getValue<int64_t>());
    }
    std::cerr << "  create under writes: " << insertedDuring << " rows committed during the build, "
              << reached.size() << "/" << numLive << " reached\n";
    EXPECT_EQ(static_cast<int64_t>(reached.size()), numLive)
        << "[check: rows-committed-during-create-indexed] " << reached.size() << " of " << numLive;
}

// La même chose avec des suppressions et des SET du vecteur validés pendant le bâti, dont des
// lignes déplacées puis supprimées dans la même fenêtre. Le CREATE les rattrape à sa validation,
// une fois les autres transactions parties (forme δ du ticket) : une ligne supprimée ne reste pas
// dans le graphe, une ligne déplacée se retrouve à son nouveau vecteur.
TEST_F(VectorIndexUpdate, ChangesCommittedDuringACreateAreSeen) {
    const auto writes = createIndexUnderWrites(20000, true /*withChanges*/);
    if (HasFailure()) {
        return;
    }
    ASSERT_GT(writes.deleted, 0) << "[check: setup] no delete committed during the CREATE";
    ASSERT_FALSE(writes.updated.empty()) << "[check: setup] no SET committed during the CREATE";
    ASSERT_GT(writes.movedThenDeleted, 0)
        << "[check: setup] no row moved then deleted during the CREATE";
    const auto failed = checkAfterCreateUnderWrites(writes);
    EXPECT_TRUE(failed.empty()) << checksOf(failed);
}

// En base en mémoire, aucun point de reprise ne force le CREATE à attendre les autres : un
// écrivain commencé avant sa validation et validé après ne voit pas l'index. Le CREATE y attend
// aussi le départ des autres, pour la même garantie qu'en base sur disque.
class VectorIndexCreateInMemory : public VectorIndexUpdate {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        inMemMode = true;
        databasePath = "";
        systemConfig->bufferPoolSize = 1024 * 1024 * 1024;
        createDBAndConn();
        concurrency::loadVectorExtension(*conn);
    }
};

// Le témoin d'acceptation de (A), le CREATE concurrent (page 07, témoin 2) : quarante écrivains
// déplacent des vecteurs en boucle pendant un CREATE, sur une base à huit fils ; aucune
// instruction d'un écrivain ne doit attendre plus de BOUND_MS. Il mêle deux défauts, que (A) règle
// tous les deux :
// - la retenue du rattrapage de (δ), joué une fois les autres partis, le temps de reprendre chaque
//   ligne déplacée (ticket 2026-10-19-un-create-vector-index-sous-ecritures-retient-les-autres-le-
//   temps-de-son-rattrapage) ;
// - la famine du bâti, qui prend tous les fils de travail pendant sa phase parallèle (ticket
//   2026-10-18-un-bati-d-index-prend-tous-les-fils-et-affame-les-autres-connexions) ; c'est elle
//   qui donne la plus grande part de l'attente sur master (environ 70 % de la durée du CREATE).
// Rouge sur master a0757d5fb : 9,1 s et 11,1 s d'attente pour un CREATE de 13 et 15 s, sur
// luciepc.
class VectorIndexCreateUnderManyWriters : public VectorIndexUpdate {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        systemConfig->bufferPoolSize = 1024 * 1024 * 1024;
        systemConfig->maxNumThreads = 8;
        createDBAndConn();
        mustRun("CALL auto_checkpoint=false;");
        concurrency::loadVectorExtension(*conn);
    }
};

TEST_F(VectorIndexCreateUnderManyWriters, NoWriterWaitsLongDuringTheCreate) {
    static constexpr int64_t NUM_ROWS = 100000;
    static constexpr int NUM_WRITERS = 40;
    static constexpr double BOUND_MS = 5000;
    table = "C";
    mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY, vec FLOAT[16]);");
    mustRun(stringFormat("UNWIND range(0, {}) AS i CREATE (:C {id: i, vec: [i % 17, i % 23, "
                         "i % 29, i % 31, i % 37, i % 41, i % 43, i % 47, i % 53, i % 59, i % 61, "
                         "i % 67, i % 71, i % 73, i % 79, i / 1000.0]});",
        NUM_ROWS - 1));
    std::atomic<bool> created{false};
    std::atomic<int64_t> statements{0};
    std::mutex worstMtx;
    double worstMs = 0;
    std::vector<std::thread> writers;
    for (auto w = 0; w < NUM_WRITERS; ++w) {
        writers.emplace_back([&, w] {
            rag3db::main::Connection connection(database.get());
            for (int64_t j = 0; !created.load(); ++j) {
                const auto id = (w * (NUM_ROWS / NUM_WRITERS) + j) % NUM_ROWS;
                const auto start = std::chrono::steady_clock::now();
                connection.query(stringFormat("MATCH (n:C {id: {}}) SET n.vec = {};", id,
                    movedRowVector(j + w * 100000)));
                const auto ms = std::chrono::duration<double, std::milli>(
                    std::chrono::steady_clock::now() - start)
                                    .count();
                statements++;
                std::lock_guard lock{worstMtx};
                worstMs = std::max(worstMs, ms);
            }
        });
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(300));
    rag3db::main::Connection creator(database.get());
    const auto createStart = std::chrono::steady_clock::now();
    auto result = creator.query("CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');");
    const auto createMs = std::chrono::duration<double, std::milli>(
        std::chrono::steady_clock::now() - createStart)
                              .count();
    created = true;
    for (auto& writer : writers) {
        writer.join();
    }
    ASSERT_TRUE(result->isSuccess()) << "[check: setup] " << result->getErrorMessage();
    std::cerr << "  create under " << NUM_WRITERS << " writers: " << createMs << " ms, "
              << statements.load() << " writer statements, the longest " << worstMs << " ms\n";
    EXPECT_LT(worstMs, BOUND_MS) << "[check: no-writer-waits-long-during-create] " << worstMs
                                 << " ms (CREATE " << createMs << " ms)";
}

TEST_F(VectorIndexCreateInMemory, ChangesCommittedDuringACreateAreSeen) {
    const auto writes = createIndexUnderWrites(20000, true /*withChanges*/);
    if (HasFailure()) {
        return;
    }
    const auto failed = checkAfterCreateUnderWrites(writes);
    EXPECT_TRUE(failed.empty()) << checksOf(failed);
}

// Une transaction d'écriture ouverte avant le CREATE et validée pendant qu'il bâtit ou attend :
// elle déplace des lignes déjà là au bâti. La validation du CREATE l'attend, puis rattrape le
// déplacement. Le CREATE ne dépasse pas le délai (la transaction part bien avant), et les lignes
// se retrouvent à leur nouveau vecteur.
TEST_F(VectorIndexUpdate, AWriterOpenAcrossTheCreateIsCaughtUp) {
    table = "C";
    mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY, vec FLOAT[16]);");
    mustRun("UNWIND range(0, 1999) AS i CREATE (:C {id: i, vec: [i % 17, i % 23, 1, 2, 3, 4, 5, "
            "6, 7, 8, 9, 10, 11, 12, 13, i / 1000.0]});");
    rag3db::main::Connection writer(database.get());
    ASSERT_TRUE(writer.query("BEGIN TRANSACTION;")->isSuccess());
    WritesDuringCreate writes;
    for (int64_t j = 0; j < 20; j++) {
        const auto id = 100 * j + 7;
        auto written = writer.query(
            stringFormat("MATCH (n:C {id: {}}) SET n.vec = {};", id, movedRowVector(j)));
        ASSERT_TRUE(written->isSuccess()) << "[check: setup] " << written->getErrorMessage();
        writes.updated.emplace_back(id, movedRowVector(j));
    }
    rag3db::main::Connection creator(database.get());
    auto creating = std::async(std::launch::async, [&] {
        auto result =
            creator.query("CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');");
        return result->isSuccess() ? std::string("ok") : result->getErrorMessage();
    });
    std::this_thread::sleep_for(std::chrono::milliseconds(500));
    auto committed = writer.query("COMMIT;");
    const auto outcome = creating.get();
    ASSERT_TRUE(committed->isSuccess()) << "[check: setup] " << committed->getErrorMessage();
    ASSERT_EQ(outcome, "ok") << "[check: create-waits-for-open-writer] " << outcome;
    const auto failed = checkAfterCreateUnderWrites(writes);
    EXPECT_TRUE(failed.empty()) << checksOf(failed);
}

// Un COPY forcé (copy_journal_threshold=1) part pendant le bâti ; sa validation attend le départ
// du CREATE, qui valide alors dans la file du point de reprise (A7) avant lui. Les lignes du COPY
// sont déjà dans la table, pas encore validées : le rattrapage s'arrête à la première, et elles
// restent hors du graphe, lues par la recherche comme toute ligne d'après lui. Les compter dans le
// graphe sans les y mettre les perdait.
TEST_F(VectorIndexUpdate, AForcedCopyCommittedRightAfterACreateIsReached) {
    table = "C";
    mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY, vec FLOAT[16]);");
    mustRun("UNWIND range(0, 19999) AS i CREATE (:C {id: i, vec: [i % 17, i % 23, i % 29, i % 31, "
            "i % 37, i % 41, i % 43, i % 47, i % 53, i % 59, i % 61, i % 67, i % 71, i % 73, i % 79, "
            "i / 1000.0]});");
    rag3db::main::Connection creator(database.get());
    rag3db::main::Connection copier(database.get());
    ASSERT_TRUE(copier.query("CALL copy_journal_threshold=1;")->isSuccess());
    auto creating = std::async(std::launch::async, [&] {
        auto result =
            creator.query("CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');");
        return result->isSuccess() ? std::string("ok") : result->getErrorMessage();
    });
    std::this_thread::sleep_for(std::chrono::milliseconds(300));
    auto copied = copier.query("COPY C FROM (UNWIND range(200000, 200999) AS i RETURN i, [i % 17, "
                               "i % 23, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, i / 1000.0]);");
    // Le COPY de mille lignes finit bien avant le bâti de vingt mille : sa validation attend
    // celle du CREATE. Les retours ne le prouvent pas (le CREATE validé dans la file attend à son
    // tour le point de reprise du COPY) ; un vert ici dirait que le bâti a fini avant le COPY.
    const auto outcome = creating.get();
    ASSERT_TRUE(copied->isSuccess()) << "[check: setup] " << copied->getErrorMessage();
    ASSERT_EQ(outcome, "ok") << "[check: setup] " << outcome;
    const auto failed = checkAfterCreateUnderWrites(WritesDuringCreate{});
    EXPECT_TRUE(failed.empty()) << checksOf(failed);
}

// Des lignes déplacées ensemble pendant un CREATE, par un écrivain ouvert avant lui. Le
// rattrapage les reprenait une à une contre une table où toutes portaient déjà leur nouveau
// vecteur : une ligne réinsérée se liait à des voisines « proches » à leur nouvelle place mais
// encore à l'ancienne dans le graphe, et le point d'entrée haut, déplacé, passait de voisine en
// voisine jusqu'à une ligne déjà partie. La recherche entrait alors dans le nuage, dont la couche
// basse ne ressortait pas : 146 lignes atteintes sur 19 984 (ChangesCommittedDuringACreateAreSeen,
// trois rouges sur quarante). Le point d'entrée du bâti tombe près du début d'un groupe de nœuds
// (2 048 lignes), et les premiers nœuds de la couche haute s'y lient entre eux : l'écrivain
// déplace les 150 premières lignes de chaque groupe vers une grille serrée. Rouge sur la moitié
// des tours ; six tours, chacun sur une table neuve.
TEST_F(VectorIndexUpdate, RowsMovedTogetherDuringACreateStayReachable) {
    constexpr int64_t NUM_ROWS = 6 * 2048;
    constexpr int64_t GROUP_SIZE = 2048;
    constexpr int64_t CHANGED_PER_GROUP = 150;
    constexpr int ROUNDS = 6;
    std::string failedRounds;
    for (int round = 0; round < ROUNDS; round++) {
        if (round > 0) {
            mustRun("CALL DROP_VECTOR_INDEX('C', 'c_index');");
            mustRun("DROP TABLE C;");
        }
        table = "C";
        mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY, vec FLOAT[16]);");
        mustRun(stringFormat("UNWIND range(0, {}) AS i CREATE (:C {id: i, vec: [i % 17, i % 23, "
                             "i % 29, i % 31, i % 37, i % 41, i % 43, i % 47, i % 53, i % 59, "
                             "i % 61, i % 67, i % 71, i % 73, i % 79, i / 1000.0]});",
            NUM_ROWS - 1));
        rag3db::main::Connection writer(database.get());
        ASSERT_TRUE(writer.query("BEGIN TRANSACTION;")->isSuccess());
        auto moved = writer.query(stringFormat(
            "MATCH (n:C) WHERE n.id % {} < {} SET n.vec = [500 + n.id % {}, n.id / {}, 0, 0, 0, "
            "0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1];",
            GROUP_SIZE, CHANGED_PER_GROUP, GROUP_SIZE, GROUP_SIZE));
        ASSERT_TRUE(moved->isSuccess()) << "[check: setup] " << moved->getErrorMessage();
        WritesDuringCreate writes;
        for (int64_t id = 0; id < NUM_ROWS; id++) {
            // Une ligne déplacée sur dix est cherchée à son vecteur : la recherche exhaustive
            // juge déjà toutes les autres.
            if (id % GROUP_SIZE < CHANGED_PER_GROUP && id % 10 == 1) {
                writes.updated.emplace_back(id,
                    stringFormat("[{}, {}, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1]",
                        500 + id % GROUP_SIZE, id / GROUP_SIZE));
            }
        }
        rag3db::main::Connection creator(database.get());
        auto creating = std::async(std::launch::async, [&] {
            auto result =
                creator.query("CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');");
            return result->isSuccess() ? std::string("ok") : result->getErrorMessage();
        });
        std::this_thread::sleep_for(std::chrono::milliseconds(300));
        auto committed = writer.query("COMMIT;");
        const auto outcome = creating.get();
        ASSERT_TRUE(committed->isSuccess()) << "[check: setup] " << committed->getErrorMessage();
        ASSERT_EQ(outcome, "ok") << "[check: setup] " << outcome;
        const auto failed = checkAfterCreateUnderWrites(writes);
        if (!failed.empty()) {
            failedRounds += stringFormat(" round {}:{}", round, checksOf(failed));
        }
    }
    EXPECT_TRUE(failedRounds.empty())
        << "[check: rows-moved-together-during-create-reachable]" << failedRounds;
}

// Un point d'entrée haut invalide (une suppression dont aucun voisin haut ne survit, ou
// l'annulation de la ligne qui le portait) : la ligne suivante qui monte dans la couche haute en
// devenait le point d'entrée sans une seule arête, comme si la couche était vide, et toute
// recherche y entrait sur elle seule. Le point d'entrée invalide est rendu tel par les internes,
// puis des lignes s'insèrent jusqu'à ce que l'une monte : le point d'entrée doit être un nœud
// qui était déjà dans la couche haute, jamais la ligne neuve.
TEST_F(VectorIndexUpdate, AnInvalidUpperEntryPointIsTakenFromTheUpperLayer) {
    constexpr int64_t NUM_ROWS = 2000;
    table = "C";
    mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY, vec FLOAT[16]);");
    mustRun(stringFormat("UNWIND range(0, {}) AS i CREATE (:C {id: i, vec: [i % 17, i % 23, "
                         "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, i / 1000.0]});",
        NUM_ROWS - 1));
    mustRun("CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');");
    mustRun("CHECKPOINT;");
    ASSERT_LT(upperEntryPointOf(*conn, "C", "c_index"), static_cast<offset_t>(NUM_ROWS))
        << "[check: setup] the mirror of HNSWStorageInfo does not read the upper entry point";
    upperEntryPointOf(*conn, "C", "c_index", INVALID_OFFSET);
    offset_t entryPoint = INVALID_OFFSET;
    for (int64_t i = NUM_ROWS; i < NUM_ROWS + 1000 && entryPoint == INVALID_OFFSET; i++) {
        mustRun(stringFormat("CREATE (:C {id: {}, vec: [{} % 17, {} % 23, 1, 2, 3, 4, 5, 6, 7, 8, "
                             "9, 10, 11, 12, 13, {}]});",
            i, i, i, i / 1000.0));
        entryPoint = upperEntryPointOf(*conn, "C", "c_index");
    }
    ASSERT_NE(entryPoint, INVALID_OFFSET) << "[check: setup] no inserted row entered the upper layer";
    EXPECT_LT(entryPoint, static_cast<offset_t>(NUM_ROWS))
        << "[check: upper-entry-point-from-the-upper-layer] the entry point is the new row "
        << entryPoint;
    const auto failed = check("after the upper entry point is replaced", 10, "c_index");
    std::string checks;
    for (const auto& name : failed) {
        checks += " [check: " + name + "]";
    }
    EXPECT_TRUE(failed.empty()) << checks;
}

// Les états d'un index en construction ((A), lots 1 et 3 : page 07 de coeur-cpp-tickets). Le
// CREATE se fait en deux transactions : T_A pose l'entrée et un holder NOTÉ, puis valide ; T_B
// bâtit et passe l'index à PRÊT. Entre les deux, une recherche est refusée par le nom de l'état,
// jamais prise pour un index inexistant. Les témoins se jouent par le temps : le bâti de 100 000
// lignes dure plusieurs secondes, et la recherche part pendant lui.
class VectorIndexCreateStates : public VectorIndexUpdate {
public:
    static constexpr int64_t NUM_ROWS = 100000;
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        systemConfig->bufferPoolSize = 1024 * 1024 * 1024;
        systemConfig->maxNumThreads = 8;
        createDBAndConn();
        mustRun("CALL auto_checkpoint=false;");
        concurrency::loadVectorExtension(*conn);
        table = "C";
        mustRun("CREATE NODE TABLE C(id INT64 PRIMARY KEY, vec FLOAT[16]);");
        mustRun(stringFormat("UNWIND range(0, {}) AS i CREATE (:C {id: i, vec: [i % 17, "
                             "i % 23, i % 29, i % 31, i % 37, i % 41, i % 43, i % 47, i % 53, "
                             "i % 59, i % 61, i % 67, i % 71, i % 73, i % 79, i / 1000.0]});",
            NUM_ROWS - 1));
    }

    static std::string searchFor(const std::string& vector, int64_t k = 1) {
        return stringFormat(
            "CALL QUERY_VECTOR_INDEX('C', 'c_index', {}, {}) RETURN node.id;", vector, k);
    }

    // Lance une recherche en boucle depuis `connection` tant que `done` est faux, et range chaque
    // message d'erreur distinct vu, dans l'ordre.
    static std::vector<std::string> searchWhile(rag3db::main::Connection& connection,
        const std::atomic<bool>& done, const std::atomic<bool>* stopOn = nullptr) {
        std::vector<std::string> seen;
        while (!done.load() && (stopOn == nullptr || !stopOn->load())) {
            auto result = connection.query(searchFor(movedRowVector(0)));
            const auto message =
                result->isSuccess() ? std::string("<success>") : result->getErrorMessage();
            if (seen.empty() || seen.back() != message) {
                seen.push_back(message);
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(20));
        }
        return seen;
    }

    static bool anyContains(const std::vector<std::string>& messages, const std::string& part) {
        return std::ranges::any_of(messages,
            [&](const std::string& m) { return m.find(part) != std::string::npos; });
    }

    static std::string joined(const std::vector<std::string>& messages) {
        std::string out;
        for (const auto& m : messages) {
            out += "\n    " + m.substr(0, 200);
        }
        return out;
    }
};

// Une recherche lancée pendant le bâti est refusée par le nom de l'état (« under construction »),
// et SHOW_INDEXES dit l'état : NOTED pendant le bâti, READY après. Sur master, l'index n'existe
// pas pour les autres avant la validation du CREATE : la recherche dit « doesn't have an index ».
TEST_F(VectorIndexCreateStates, ASearchDuringTheBuildIsRefusedByItsState) {
    std::atomic<bool> created{false};
    rag3db::main::Connection searcher(database.get());
    std::vector<std::string> seen;
    std::vector<std::string> states;
    std::thread searching([&] {
        seen = searchWhile(searcher, created);
    });
    std::thread showing([&] {
        rag3db::main::Connection shower(database.get());
        while (!created.load()) {
            auto shown = shower.query("CALL SHOW_INDEXES() RETURN index_name, state;");
            if (shown->isSuccess() && shown->hasNext()) {
                const auto state = shown->getNext()->getValue(1)->toString();
                if (states.empty() || states.back() != state) {
                    states.push_back(state);
                }
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(20));
        }
    });
    rag3db::main::Connection creator(database.get());
    auto result = creator.query("CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');");
    created = true;
    searching.join();
    showing.join();
    ASSERT_TRUE(result->isSuccess()) << "[check: setup] " << result->getErrorMessage();
    std::cerr << "  messages seen during the CREATE:" << joined(seen) << "\n  states shown:"
              << joined(states) << "\n";
    EXPECT_TRUE(anyContains(seen, "is under construction"))
        << "[check: search-during-build-refused-by-state]" << joined(seen);
    EXPECT_TRUE(anyContains(states, "NOTED")) << "[check: show-indexes-says-the-state]"
                                              << joined(states);
    auto shown = conn->query("CALL SHOW_INDEXES() RETURN state;");
    ASSERT_TRUE(shown->isSuccess()) << "[check: show-indexes-says-the-state] "
                                    << shown->getErrorMessage();
    ASSERT_TRUE(shown->hasNext());
    EXPECT_EQ(shown->getNext()->getValue(0)->toString(), "READY")
        << "[check: show-indexes-says-the-state]";
    auto served = conn->query(searchFor(movedRowVector(0)));
    EXPECT_TRUE(served->isSuccess()) << "[check: search-served-when-ready] "
                                     << served->getErrorMessage();
}

// Une lecture sur un instantané d'avant le CREATE ne voit pas l'index : interrogée après la
// validation de T_A (une autre connexion le voit alors en construction), elle dit qu'il n'existe
// pas, jamais qu'il est en construction. Elle valide ensuite : la validation forcée de T_B
// attend toutes les transactions ouvertes, lecteurs compris (page 07, §3 bis.3).
TEST_F(VectorIndexCreateStates, ASnapshotBeforeTheCreateSaysTheIndexDoesNotExist) {
    rag3db::main::Connection reader(database.get());
    ASSERT_TRUE(reader.query("BEGIN TRANSACTION READ ONLY;")->isSuccess());
    ASSERT_TRUE(reader.query("MATCH (n:C) RETURN count(n);")->isSuccess());
    std::atomic<bool> created{false};
    rag3db::main::Connection creator(database.get());
    auto creating = std::async(std::launch::async, [&] {
        auto result =
            creator.query("CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');");
        created = true;
        return result->isSuccess() ? std::string("ok") : result->getErrorMessage();
    });
    rag3db::main::Connection watcher(database.get());
    bool noted = false;
    // Court : sur master, le CREATE attend ce lecteur dès sa validation, sous un délai.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(3);
    while (!noted && !created.load() && std::chrono::steady_clock::now() < deadline) {
        auto result = watcher.query(searchFor(movedRowVector(0)));
        noted = !result->isSuccess() &&
                result->getErrorMessage().find("is under construction") != std::string::npos;
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }
    auto old = reader.query(searchFor(movedRowVector(0)));
    const auto message = old->isSuccess() ? std::string("<success>") : old->getErrorMessage();
    old.reset();
    ASSERT_TRUE(reader.query("COMMIT;")->isSuccess());
    const auto outcome = creating.get();
    std::cerr << "  search on a snapshot before the CREATE (index seen NOTED by others: "
              << (noted ? "yes" : "no") << "): " << message.substr(0, 200) << "\n";
    ASSERT_EQ(outcome, "ok") << "[check: setup] " << outcome;
    EXPECT_NE(message.find("doesn't have an index"), std::string::npos)
        << "[check: snapshot-before-create-says-no-index] " << message;
    EXPECT_EQ(message.find("under construction"), std::string::npos)
        << "[check: snapshot-before-create-says-no-index] " << message;
}

// Un écrivain parti pendant NOTÉ (après la validation de T_A) insère et déplace des vecteurs, et
// valide pendant l'attente de T_B : la validation de T_B l'attend (elle finit après lui, mesuré),
// puis le rattrape. Ses vecteurs sont trouvés, et la ligne qu'il a supprimée n'est pas rendue.
TEST_F(VectorIndexCreateStates, AWriterStartedWhileNotedIsWaitedForAndFound) {
    std::atomic<bool> created{false};
    std::atomic<bool> noted{false};
    rag3db::main::Connection searcher(database.get());
    std::thread watching([&] {
        while (!created.load() && !noted.load()) {
            auto result = searcher.query(searchFor(movedRowVector(0)));
            if (!result->isSuccess() &&
                result->getErrorMessage().find("is under construction") != std::string::npos) {
                noted = true;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
    });
    rag3db::main::Connection creator(database.get());
    std::chrono::steady_clock::time_point createEnd;
    auto creating = std::async(std::launch::async, [&] {
        auto result =
            creator.query("CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');");
        createEnd = std::chrono::steady_clock::now();
        created = true;
        return result->isSuccess() ? std::string("ok") : result->getErrorMessage();
    });
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(30);
    while (!noted.load() && !created.load() && std::chrono::steady_clock::now() < deadline) {
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }
    rag3db::main::Connection writer(database.get());
    const bool sawNoted = noted.load();
    ASSERT_TRUE(writer.query("BEGIN TRANSACTION;")->isSuccess());
    const auto inserted = "[900, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1]";
    const auto moved = "[901, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1]";
    const auto gone = "[5 % 17, 5 % 23, 5 % 29, 5 % 31, 5 % 37, 5 % 41, 5 % 43, 5 % 47, 5 % 53, "
                      "5 % 59, 5 % 61, 5 % 67, 5 % 71, 5 % 73, 5 % 79, 5 / 1000.0]";
    ASSERT_TRUE(writer.query(stringFormat("CREATE (:C {id: 900000, vec: {}});", inserted))
                    ->isSuccess());
    ASSERT_TRUE(
        writer.query(stringFormat("MATCH (n:C {id: 7}) SET n.vec = {};", moved))->isSuccess());
    ASSERT_TRUE(writer.query("MATCH (n:C {id: 5}) DELETE n;")->isSuccess());
    std::this_thread::sleep_for(std::chrono::milliseconds(500));
    const auto writerCommit = std::chrono::steady_clock::now();
    auto committed = writer.query("COMMIT;");
    const auto outcome = creating.get();
    created = true;
    watching.join();
    ASSERT_TRUE(sawNoted) << "[check: setup] the writer did not start while the index was NOTED";
    ASSERT_TRUE(committed->isSuccess()) << "[check: setup] " << committed->getErrorMessage();
    ASSERT_EQ(outcome, "ok") << "[check: setup] " << outcome;
    const auto waitedMs =
        std::chrono::duration<double, std::milli>(createEnd - writerCommit).count();
    std::cerr << "  the CREATE ended " << waitedMs << " ms after the writer committed\n";
    EXPECT_GE(waitedMs, 0.0) << "[check: tb-waited-for-the-writer] the CREATE ended "
                             << -waitedMs << " ms before the writer committed";
    const auto firstOf = [&](const std::string& vector) {
        auto result = conn->query(searchFor(vector));
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   int64_t{-1};
    };
    EXPECT_EQ(firstOf(inserted), 900000) << "[check: writer-insert-found]";
    EXPECT_EQ(firstOf(moved), 7) << "[check: writer-move-found]";
    EXPECT_NE(firstOf(gone), 5) << "[check: writer-delete-not-returned]";
}

// Un CREATE interrompu pendant son bâti laisse un index qui le dit : « incomplete, rebuild it ».
// Le DROP est permis, puis un CREATE neuf sert les recherches. Sur master, tout le CREATE est
// annulé et la recherche dit « doesn't have an index ».
TEST_F(VectorIndexCreateStates, AnInterruptedCreateLeavesAnIndexToRebuild) {
    std::atomic<bool> created{false};
    std::atomic<bool> noted{false};
    rag3db::main::Connection creator(database.get());
    auto creating = std::async(std::launch::async, [&] {
        auto result =
            creator.query("CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');");
        created = true;
        return result->isSuccess() ? std::string("ok") : result->getErrorMessage();
    });
    rag3db::main::Connection searcher(database.get());
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(30);
    while (!created.load() && std::chrono::steady_clock::now() < deadline) {
        auto result = searcher.query(searchFor(movedRowVector(0)));
        if (!result->isSuccess() &&
            result->getErrorMessage().find("is under construction") != std::string::npos) {
            noted = true;
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }
    creator.interrupt();
    const auto outcome = creating.get();
    ASSERT_TRUE(noted.load()) << "[check: setup] the index was never seen NOTED";
    ASSERT_NE(outcome, "ok") << "[check: setup] the CREATE finished before the interruption";
    auto refused = conn->query(searchFor(movedRowVector(0)));
    const auto message =
        refused->isSuccess() ? std::string("<success>") : refused->getErrorMessage();
    std::cerr << "  after the interrupted CREATE: " << message.substr(0, 240) << "\n";
    EXPECT_NE(message.find("is incomplete"), std::string::npos)
        << "[check: interrupted-create-leaves-incomplete] " << message;
    auto shown = conn->query("CALL SHOW_INDEXES() RETURN state;");
    ASSERT_TRUE(shown->isSuccess()) << shown->getErrorMessage();
    ASSERT_TRUE(shown->hasNext()) << "[check: interrupted-create-leaves-incomplete]";
    EXPECT_EQ(shown->getNext()->getValue(0)->toString(), "INCOMPLETE")
        << "[check: interrupted-create-leaves-incomplete]";
    shown.reset();
    auto dropped = conn->query("CALL DROP_VECTOR_INDEX('C', 'c_index');");
    EXPECT_TRUE(dropped->isSuccess()) << "[check: incomplete-index-dropped] "
                                      << dropped->getErrorMessage();
    auto again = conn->query("CALL CREATE_VECTOR_INDEX('C', 'c_index', 'vec', metric := 'l2');");
    ASSERT_TRUE(again->isSuccess()) << "[check: incomplete-index-rebuilt] "
                                    << again->getErrorMessage();
    auto served = conn->query(searchFor(movedRowVector(0)));
    EXPECT_TRUE(served->isSuccess()) << "[check: incomplete-index-rebuilt] "
                                     << served->getErrorMessage();
}

} // namespace

#endif
