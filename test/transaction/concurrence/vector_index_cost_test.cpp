// Le coût d'une suppression et d'une insertion sous index vectoriel (ticket vector de hotfixs, 12
// octobre 2026). Mesuré par le proto sur 15 631 vecteurs de 768 : le DETACH DELETE d'un lot de 45
// morceaux portant un vecteur indexé prend 112 s (259 ms sans vecteur), soit environ 2,5 s par
// ligne ; l'insertion prend 20 ms par vecteur (309 s pour 15 631).
//
// Les écritures du produit, reprises ici : l'index est posé par
// CREATE_VECTOR_INDEX(..., metric := 'cosine') avec les paramètres par défaut ; les vecteurs se
// posent par SET depuis NULL, un UNWIND d'une liste de maps passée en paramètre ; un lot de
// morceaux se supprime en une requête, UNWIND puis MATCH puis DETACH DELETE. Les vecteurs sont
// groupés autour de 64 centres, comme des plongements de code, et non tirés uniformément.
//
// - Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize : le témoin. Deux tables indexées, de
//   1 500 et de 15 000 lignes ; trois lots de 10 lignes supprimés chacun en une requête, avant puis
//   après un point de reprise ; le temps médian par ligne de la grande table doit rester sous deux
//   fois celui de la petite (plus 2 ms de marge contre le bruit). Pour un facteur 10 sur la
//   taille, un coût en log passe et un coût linéaire rougit. Trois façons de faire l'index : par
//   CREATE sur la table pleine, rempli par SET depuis NULL en lots de 64 (le produit), et le même
//   avec une relation entrante par ligne.
// - VectorIndexCost.ProductShapeDeleteCost : la forme du produit au plus près (index créé sur la
//   table vide, COPY sans vecteur, relation sortante, SET en lots de 64, aucun point de reprise),
//   cinq DETACH DELETE successifs filtrés sur une colonne non clé, puis cinq précédés d'un SET
//   NULL : le DELETE direct ne doit pas coûter plus de cinq fois SET NULL puis DELETE, et son coût
//   ne doit pas croître d'un essai à l'autre (le cinquième sous 1,5 fois le premier).
// - VectorIndexCost.MeasureDelete : le temps par ligne pour des lots de 1, 10 et 45 lignes en une
//   requête, et pour 10 lignes une requête par ligne, à 1 500 et 15 000 lignes, index non
//   checkpointé puis checkpointé, après SET du vecteur à NULL, et sur la même table sans index.
// - VectorIndexCost.MeasureInsert : à 1, 8 et 32 fils, la création de l'index sur 15 000 vecteurs
//   posés, puis des lots de 500 SET depuis NULL au début du remplissage (index vide) et à la fin
//   (13 000 vecteurs déjà indexés), chaque fois avec la même table sans index pour référence.
// Les deux mesures sont sautées sans CONCURRENCE_MEASURE=1 ; à jouer sous `poste mesure`.

#include <algorithm>
#include <chrono>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <functional>
#include <iostream>
#include <map>
#include <numeric>
#include <random>
#include <string>
#include <vector>

#include "bench_harness.h"
#include "common/build_id.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr int64_t SMALL = 1500;
constexpr int64_t LARGE = 15000;
constexpr int DIM = 768;
constexpr int NUM_CENTERS = 64;
constexpr double SIZE_FACTOR = 2;
constexpr double NOISE_MS = 2;

std::vector<float> vectorOf(int64_t id) {
    static const auto centers = [] {
        std::mt19937 engine(42);
        std::normal_distribution<float> normal;
        std::vector<std::vector<float>> out(NUM_CENTERS, std::vector<float>(DIM));
        for (auto& center : out) {
            for (auto& value : center) {
                value = normal(engine);
            }
        }
        return out;
    }();
    std::mt19937 engine(static_cast<uint32_t>(id + 1));
    std::normal_distribution<float> normal(0, 0.5);
    auto out = centers[id % NUM_CENTERS];
    for (auto& value : out) {
        value += normal(engine);
    }
    return out;
}

bool measuring() {
    return std::getenv("CONCURRENCE_MEASURE") != nullptr;
}

class VectorIndexCost : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed (checkpointed index)";
        }
        systemConfig->bufferPoolSize = 2ull * 1024 * 1024 * 1024;
        createDBAndConn();
        ok("CALL auto_checkpoint=false;");
        concurrency::loadVectorExtension(*conn);
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess())
            << "[check: query] " << query.substr(0, 160) << " : " << result->getErrorMessage();
    }

    static double timed(const std::function<void()>& work) {
        const auto start = std::chrono::steady_clock::now();
        work();
        return std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start)
            .count();
    }

    static void line(const std::string& text) {
        std::cerr << "  [coût vectoriel] " << text << " bati=" << RAG3DB_BUILD_ID << "\n";
    }

    // Une table de `rows` lignes, dont les `withVector` premières portent un vecteur et les
    // autres NULL, chargée par COPY ; l'ordre des lignes à supprimer est tiré une fois pour elle.
    void createTable(const std::string& name, int64_t rows, int64_t withVector) {
        ok(stringFormat("CREATE NODE TABLE {}(id INT64 PRIMARY KEY, vec FLOAT[{}]);", name, DIM));
        const auto path = databasePath + "." + name + ".csv";
        {
            std::ofstream out(path);
            for (int64_t id = 0; id < rows; ++id) {
                out << id << ",";
                if (id < withVector) {
                    out << "\"[";
                    const auto vector = vectorOf(id);
                    for (auto d = 0; d < DIM; ++d) {
                        out << (d ? "," : "") << vector[d];
                    }
                    out << "]\"";
                }
                out << "\n";
            }
        }
        ok("COPY " + name + " FROM '" + path + "' (header=false);");
        std::filesystem::remove(path);
        auto& order = deletionOrder[name];
        for (int64_t id = 0; id < withVector; ++id) {
            order.push_back(id);
        }
        std::shuffle(order.begin(), order.end(), std::mt19937(7));
    }

    double createIndex(const std::string& name) {
        return timed([&] {
            ok(stringFormat(
                "CALL CREATE_VECTOR_INDEX('{}', '{}_index', 'vec', metric := 'cosine');",
                name, name));
        });
    }

    // Des lignes encore présentes, éparpillées dans la table.
    std::vector<int64_t> takeIds(const std::string& name, int64_t count) {
        auto& order = deletionOrder[name];
        EXPECT_GE(static_cast<int64_t>(order.size()), count) << "[check: enough-rows] " << name;
        const auto taken = std::min<int64_t>(count, order.size());
        std::vector<int64_t> out(order.end() - taken, order.end());
        order.resize(order.size() - taken);
        return out;
    }

    static std::string listOf(const std::vector<int64_t>& ids) {
        std::string out = "[";
        for (auto i = 0u; i < ids.size(); ++i) {
            out += (i ? "," : "") + std::to_string(ids[i]);
        }
        return out + "]";
    }

    // La forme du produit : une requête pour tout le lot.
    double deleteInOneQuery(const std::string& name, const std::vector<int64_t>& ids) {
        return timed([&] {
            ok(stringFormat("UNWIND {} AS i MATCH (n:{} {id: i}) DETACH DELETE n;", listOf(ids),
                name));
        });
    }

    double deleteLineByLine(const std::string& name, const std::vector<int64_t>& ids) {
        return timed([&] {
            for (const auto id : ids) {
                ok(stringFormat("MATCH (n:{} {id: {}}) DETACH DELETE n;", name, id));
            }
        });
    }

    // La forme du produit : UNWIND d'une liste de maps {id, emb} passée en paramètre, SET depuis
    // NULL. Les vecteurs sont des listes de DOUBLE, comme les Float(f64) de rag3weaver.
    double setFromNull(const std::string& name, const std::vector<int64_t>& ids) {
        auto prepared = conn->prepare(stringFormat(
            "UNWIND $items AS item MATCH (n:{} {id: item.id}) SET n.vec = item.emb;", name));
        EXPECT_TRUE(prepared->isSuccess()) << "[check: query] " << prepared->getErrorMessage();
        std::vector<StructField> fields;
        fields.emplace_back("id", LogicalType::INT64());
        fields.emplace_back("emb", LogicalType::LIST(LogicalType::DOUBLE()));
        const auto itemType = LogicalType::STRUCT(std::move(fields));
        std::vector<std::unique_ptr<Value>> items;
        for (const auto id : ids) {
            std::vector<std::unique_ptr<Value>> emb;
            for (const auto value : vectorOf(id)) {
                emb.push_back(std::make_unique<Value>(static_cast<double>(value)));
            }
            std::vector<std::unique_ptr<Value>> item;
            item.push_back(std::make_unique<Value>(id));
            item.push_back(
                std::make_unique<Value>(LogicalType::LIST(LogicalType::DOUBLE()), std::move(emb)));
            items.push_back(std::make_unique<Value>(itemType.copy(), std::move(item)));
        }
        std::unordered_map<std::string, std::unique_ptr<Value>> params;
        params["items"] =
            std::make_unique<Value>(LogicalType::LIST(itemType.copy()), std::move(items));
        return timed([&] {
            auto result = conn->executeWithParams(prepared.get(), std::move(params));
            EXPECT_TRUE(result->isSuccess()) << "[check: query] " << result->getErrorMessage();
        });
    }

    std::map<std::string, std::vector<int64_t>> deletionOrder;
};

// Comment l'index s'est fait : par CREATE sur une table pleine, ou rempli au fil des SET depuis
// NULL par lots de 64 (le chemin du produit : des morceaux insérés sans vecteur, puis embarqués),
// et, pour la dernière, une relation entrante par ligne, que DETACH DELETE retire aussi.
enum class Build { CREATE_ON_FULL_TABLE, SET_IN_BATCHES_OF_64, SET_IN_BATCHES_OF_64_WITH_RELATION };

class VectorIndexDeleteCost : public VectorIndexCost, public ::testing::WithParamInterface<Build> {
protected:
    void prepare(const std::string& name, int64_t rows) {
        if (GetParam() == Build::CREATE_ON_FULL_TABLE) {
            createTable(name, rows, rows);
            createIndex(name);
            return;
        }
        createTable(name, rows, 0);
        auto& order = deletionOrder[name];
        for (int64_t id = 0; id < rows; ++id) {
            order.push_back(id);
        }
        std::shuffle(order.begin(), order.end(), std::mt19937(7));
        createIndex(name);
        for (int64_t first = 0; first < rows; first += 64) {
            std::vector<int64_t> ids;
            for (auto id = first; id < std::min(rows, first + 64); ++id) {
                ids.push_back(id);
            }
            setFromNull(name, ids);
        }
        if (GetParam() == Build::SET_IN_BATCHES_OF_64_WITH_RELATION) {
            ok(stringFormat("CREATE NODE TABLE {}Parent(id INT64 PRIMARY KEY);", name));
            ok(stringFormat("CREATE REL TABLE {}Has(FROM {}Parent TO {});", name, name, name));
            ok(stringFormat("UNWIND range(0, {}) AS i CREATE (:{}Parent {id: i});", rows - 1,
                name));
            const auto path = databasePath + "." + name + ".has.csv";
            {
                std::ofstream out(path);
                for (int64_t id = 0; id < rows; ++id) {
                    out << id << "," << id << "\n";
                }
            }
            ok(stringFormat("COPY {}Has FROM '{}' (header=false);", name, path));
            std::filesystem::remove(path);
        }
    }

    double medianPerRow(const std::string& name) {
        std::vector<double> perRow;
        for (auto batch = 0; batch < 3; ++batch) {
            perRow.push_back(deleteInOneQuery(name, takeIds(name, 10)) / 10);
        }
        std::sort(perRow.begin(), perRow.end());
        return perRow[1];
    }
};

TEST_P(VectorIndexDeleteCost, DoesNotDependOnIndexSize) {
    const auto start = std::chrono::steady_clock::now();
    prepare("Small", SMALL);
    prepare("Large", LARGE);
    line(stringFormat("préparation {} s",
        std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count()));
    for (const auto checkpointed : {false, true}) {
        if (checkpointed) {
            ok("CHECKPOINT;");
        }
        const auto state = checkpointed ? "checkpointé" : "non checkpointé";
        const auto small = medianPerRow("Small");
        const auto large = medianPerRow("Large");
        line(stringFormat("témoin, index {} : suppression médiane par ligne {} ms à {} lignes, {} "
                          "ms à {} lignes",
            state, small, SMALL, large, LARGE));
        EXPECT_LE(large, SIZE_FACTOR * small + NOISE_MS)
            << (checkpointed ? "[check: delete-independent-of-index-size-checkpointed] " :
                               "[check: delete-independent-of-index-size] ")
            << large << " ms par ligne à " << LARGE << " lignes, " << small << " ms à " << SMALL;
    }
}

INSTANTIATE_TEST_SUITE_P(Builds, VectorIndexDeleteCost,
    ::testing::Values(Build::CREATE_ON_FULL_TABLE, Build::SET_IN_BATCHES_OF_64,
        Build::SET_IN_BATCHES_OF_64_WITH_RELATION),
    [](const ::testing::TestParamInfo<Build>& info) {
        switch (info.param) {
        case Build::CREATE_ON_FULL_TABLE:
            return std::string("CreateOnFullTable");
        case Build::SET_IN_BATCHES_OF_64:
            return std::string("SetInBatchesOf64");
        case Build::SET_IN_BATCHES_OF_64_WITH_RELATION:
            return std::string("SetInBatchesOf64WithRelation");
        }
        return std::string("?");
    });

// La forme du produit, au plus près (faits du proto, 12 octobre) : l'index créé sur la table vide,
// les morceaux chargés par COPY sans vecteur avec une relation sortante chacun vers leur Scope, les
// vecteurs posés par SET depuis NULL en lots de 64, aucun point de reprise. Puis cinq DETACH DELETE
// successifs d'un morceau, filtrés sur _parent_uuid (pas la clé), chacun chronométré ; puis cinq
// autres précédés d'un SET du vecteur à NULL. Le proto mesurait 2,1 à 2,9 s, croissant d'un essai à
// l'autre, pour le DELETE direct, et 7 ms + 4 à 8 ms pour SET NULL puis DELETE.
TEST_F(VectorIndexCost, ProductShapeDeleteCost) {
    constexpr auto TRIALS = 5;
    constexpr double SET_NULL_FACTOR = 5;
    constexpr double GROWTH_FACTOR = 1.5;
    ok("CREATE NODE TABLE Scope(_uuid STRING PRIMARY KEY);");
    ok(stringFormat("CREATE NODE TABLE Chunk(_uuid STRING PRIMARY KEY, _parent_uuid STRING, vec "
                    "FLOAT[{}]);",
        DIM));
    ok("CREATE REL TABLE CHUNKED_FROM(FROM Chunk TO Scope);");
    createIndex("Chunk");
    const auto start = std::chrono::steady_clock::now();
    const auto scopes = databasePath + ".scopes.csv";
    const auto chunks = databasePath + ".chunks.csv";
    const auto edges = databasePath + ".edges.csv";
    {
        std::ofstream scopeOut(scopes);
        std::ofstream chunkOut(chunks);
        std::ofstream edgeOut(edges);
        for (int64_t id = 0; id < LARGE; ++id) {
            scopeOut << "s-" << id << "\n";
            chunkOut << "c-" << id << ",s-" << id << ",\n";
            edgeOut << "c-" << id << ",s-" << id << "\n";
        }
    }
    ok("COPY Scope FROM '" + scopes + "' (header=false);");
    ok("COPY Chunk FROM '" + chunks + "' (header=false);");
    ok("COPY CHUNKED_FROM FROM '" + edges + "' (header=false);");
    for (const auto& path : {scopes, chunks, edges}) {
        std::filesystem::remove(path);
    }
    auto prepared = conn->prepare(
        "UNWIND $items AS item MATCH (n:Chunk {_uuid: item.uuid}) SET n.vec = item.emb;");
    ASSERT_TRUE(prepared->isSuccess()) << "[check: query] " << prepared->getErrorMessage();
    for (int64_t first = 0; first < LARGE; first += 64) {
        std::vector<StructField> fields;
        fields.emplace_back("uuid", LogicalType::STRING());
        fields.emplace_back("emb", LogicalType::LIST(LogicalType::DOUBLE()));
        const auto itemType = LogicalType::STRUCT(std::move(fields));
        std::vector<std::unique_ptr<Value>> items;
        for (auto id = first; id < std::min(LARGE, first + 64); ++id) {
            std::vector<std::unique_ptr<Value>> emb;
            for (const auto value : vectorOf(id)) {
                emb.push_back(std::make_unique<Value>(static_cast<double>(value)));
            }
            std::vector<std::unique_ptr<Value>> item;
            item.push_back(std::make_unique<Value>("c-" + std::to_string(id)));
            item.push_back(
                std::make_unique<Value>(LogicalType::LIST(LogicalType::DOUBLE()), std::move(emb)));
            items.push_back(std::make_unique<Value>(itemType.copy(), std::move(item)));
        }
        std::unordered_map<std::string, std::unique_ptr<Value>> params;
        params["items"] =
            std::make_unique<Value>(LogicalType::LIST(itemType.copy()), std::move(items));
        auto result = conn->executeWithParams(prepared.get(), std::move(params));
        ASSERT_TRUE(result->isSuccess()) << "[check: query] " << result->getErrorMessage();
    }
    line(stringFormat("forme du produit : préparation {} s (dont {} SET de 64)",
        std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count(),
        (LARGE + 63) / 64));
    // Des morceaux éparpillés, chacun seul de son Scope.
    std::vector<int64_t> order(LARGE);
    std::iota(order.begin(), order.end(), 0);
    std::shuffle(order.begin(), order.end(), std::mt19937(7));
    const auto deleteOne = [&](int64_t id) {
        return timed([&] {
            auto result = conn->query(stringFormat("UNWIND ['s-{}'] AS uuid MATCH (c:Chunk "
                                                   "{_parent_uuid: uuid}) DETACH DELETE c RETURN "
                                                   "uuid, count(c);",
                id));
            ASSERT_TRUE(result->isSuccess()) << "[check: query] " << result->getErrorMessage();
            ASSERT_TRUE(result->hasNext());
            EXPECT_EQ(result->getNext()->getValue(1)->getValue<int64_t>(), 1)
                << "[check: one-chunk-deleted]";
        });
    };
    std::vector<double> direct;
    std::vector<double> throughNull;
    for (auto trial = 0; trial < TRIALS; ++trial) {
        direct.push_back(deleteOne(order[trial]));
    }
    for (auto trial = 0; trial < TRIALS; ++trial) {
        const auto id = order[TRIALS + trial];
        const auto nullMs = timed([&] {
            ok(stringFormat("MATCH (c:Chunk {_parent_uuid: 's-{}'}) SET c.vec = NULL;", id));
        });
        throughNull.push_back(nullMs + deleteOne(id));
    }
    std::string directText;
    std::string nullText;
    for (auto trial = 0; trial < TRIALS; ++trial) {
        directText += stringFormat(" {}", direct[trial]);
        nullText += stringFormat(" {}", throughNull[trial]);
    }
    line("forme du produit : DETACH DELETE direct, ms par essai :" + directText);
    line("forme du produit : SET NULL puis DETACH DELETE, ms par essai :" + nullText);
    auto sortedDirect = direct;
    auto sortedNull = throughNull;
    std::sort(sortedDirect.begin(), sortedDirect.end());
    std::sort(sortedNull.begin(), sortedNull.end());
    EXPECT_LE(sortedDirect[TRIALS / 2], SET_NULL_FACTOR * sortedNull[TRIALS / 2] + NOISE_MS)
        << "[check: delete-close-to-set-null-then-delete] médianes " << sortedDirect[TRIALS / 2]
        << " ms et " << sortedNull[TRIALS / 2] << " ms";
    EXPECT_LE(direct[TRIALS - 1], GROWTH_FACTOR * direct[0] + NOISE_MS)
        << "[check: delete-cost-does-not-grow] premier " << direct[0] << " ms, dernier "
        << direct[TRIALS - 1] << " ms";
    // Les mêmes suppressions après un point de reprise (hypothèse de hotfixs : c'est la lecture
    // des arêtes écrites depuis le dernier point de reprise qui coûte, pas l'algorithme).
    const auto checkpointMs = timed([&] { ok("CHECKPOINT;"); });
    std::string afterText;
    for (auto trial = 0; trial < TRIALS; ++trial) {
        afterText += stringFormat(" {}", deleteOne(order[2 * TRIALS + trial]));
    }
    line(stringFormat("forme du produit : DETACH DELETE direct après un point de reprise ({} ms), "
                      "ms par essai :{}",
        checkpointMs, afterText));
}

// La pente de l'insertion à taille d'index presque constante (12 octobre 2026, après le lot de
// l'insertion du cœur C++, 3cca0ae1e). Depuis un index vide, l'insertion coûte naturellement plus à
// mesure que l'index grandit (la recherche s'allonge en log N) ; ici l'index est fait par CREATE
// sur 11 000 vecteurs, et 4 096 autres lignes reçoivent leur vecteur par SET depuis NULL en 64 lots
// de 64 : l'index grandit de 37 %, un coût en log N n'en bouge que de quelques pour cent.
// Lu par le cœur C++ : après un CREATE, les degrés sont compacts, et les premières insertions
// ajoutent des arêtes sans rien rétrécir ; vers 1 300 SET, les nœuds pivots atteignent leur seuil
// et chaque insertion déclenche ses rétrécissements, un régime permanent. Trois lectures :
// - VectorIndexInsertSlope.CostPerBatchSettles, contrôle
//   insert-cost-without-checkpoint-stays-close : les mêmes SET avec et sans un CHECKPOINT tous
//   les 8 lots ; à la fin, sans point de reprise, les 5 derniers lots au plus 1,2 fois ceux de la
//   courbe avec, plus 2 ms. Rouge le 12 octobre (1,5 à 1,7 fois sur quatre passes) : sans point
//   de reprise, après la montée des rétrécissements, le coût croît d'environ 1,1 ms par lot ;
//   ce qui s'accumule vit en mémoire jusqu'au point de reprise (lignes mortes du CSR validé que
//   chaque balayage de voisins relit, chaîne des versions). Une première borne, les 5 derniers
//   lots contre les lots 25 à 30 de la même courbe, rougissait trois passes sur quatre : trop
//   près de la croissance sous une charge qui varie.
// - le même cas, contrôle checkpoint-keeps-insert-flat (la garde) : les mêmes SET avec un
//   CHECKPOINT tous les 8 lots, la même borne ; la courbe y reste plate.
// - le plateau avec points de reprise contre les premiers lots, imprimé sans contrôle : le coût des
//   rétrécissements, côté extension.
// Les lignes « premiers lots », « lots du plateau » (au quart) et « derniers lots » marquent les
// phases pour le profil.
class VectorIndexInsertSlope : public VectorIndexCost {
protected:
    static constexpr int64_t INDEXED = 11000;
    // Un multiple du lot : un dernier lot partiel fausserait la moyenne des derniers.
    static constexpr int64_t ADDED = 4096;
    static constexpr int64_t BATCH = 64;
    static constexpr int64_t EDGE_BATCHES = 5;
    static constexpr int64_t PLATEAU_FROM = 25;
    static constexpr int64_t PLATEAU_BATCHES = 6;
    static constexpr int64_t CHECKPOINT_EVERY = 8;
    static constexpr double SETTLE_FACTOR = 1.2;

    // Les temps de chaque lot de SET depuis NULL, un CHECKPOINT tous les `checkpointEvery` lots
    // (0 : jamais).
    std::vector<double> fill(const std::string& name, int64_t checkpointEvery) {
        createTable(name, INDEXED + ADDED, INDEXED);
        const auto createMs = createIndex(name);
        line(stringFormat("pente {} : CREATE de l'index sur {} vecteurs {} ms ({} ms par vecteur)",
            name, INDEXED, createMs, createMs / INDEXED));
        std::vector<double> perBatch;
        const auto numBatches = (ADDED + BATCH - 1) / BATCH;
        for (int64_t batch = 0; batch < numBatches; ++batch) {
            if (batch == 0) {
                line(stringFormat("pente {} : premiers lots", name));
            }
            if (batch == numBatches / 4) {
                line(stringFormat("pente {} : lots du plateau", name));
            }
            if (batch == numBatches - EDGE_BATCHES) {
                line(stringFormat("pente {} : derniers lots", name));
            }
            std::vector<int64_t> ids;
            const auto first = INDEXED + batch * BATCH;
            for (auto id = first; id < std::min(INDEXED + ADDED, first + BATCH); ++id) {
                ids.push_back(id);
            }
            perBatch.push_back(setFromNull(name, ids));
            if (checkpointEvery > 0 && (batch + 1) % checkpointEvery == 0 &&
                batch + 1 < numBatches) {
                line(stringFormat("pente {} : CHECKPOINT après le lot {} : {} ms", name, batch,
                    timed([&] { ok("CHECKPOINT;"); })));
            }
        }
        std::string text;
        for (const auto ms : perBatch) {
            text += stringFormat(" {}", static_cast<int64_t>(ms));
        }
        line(stringFormat("pente {} : ms par lot de {} :{}", name, BATCH, text));
        return perBatch;
    }

    static double mean(const std::vector<double>& values, size_t from, size_t count) {
        double sum = 0;
        for (auto i = from; i < from + count; ++i) {
            sum += values[i];
        }
        return sum / static_cast<double>(count);
    }
};

TEST_F(VectorIndexInsertSlope, CostPerBatchSettles) {
    const auto plain = fill("Slope", 0);
    const auto first = mean(plain, 0, EDGE_BATCHES);
    const auto plateau = mean(plain, PLATEAU_FROM, PLATEAU_BATCHES);
    const auto last = mean(plain, plain.size() - EDGE_BATCHES, EDGE_BATCHES);
    // Sans point de reprise, imprimé.
    line(stringFormat("pente : {} premiers lots {} ms, lots {} à {} {} ms, {} derniers {} ms ; "
                      "derniers / premiers ×{}",
        EDGE_BATCHES, first, PLATEAU_FROM, PLATEAU_FROM + PLATEAU_BATCHES - 1, plateau,
        EDGE_BATCHES, last, last / first));
    const auto withCheckpoints = fill("SlopeCheckpointed", CHECKPOINT_EVERY);
    // Le lot juste avant et juste après chaque point de reprise, imprimés.
    for (auto batch = static_cast<size_t>(CHECKPOINT_EVERY); batch < withCheckpoints.size();
        batch += CHECKPOINT_EVERY) {
        line(stringFormat("pente avec CHECKPOINT tous les {} lots : lot {} avant {} ms, lot {} "
                          "après {} ms",
            CHECKPOINT_EVERY, batch - 1, withCheckpoints[batch - 1], batch,
            withCheckpoints[batch]));
    }
    const auto checkpointedFirst = mean(withCheckpoints, 0, EDGE_BATCHES);
    const auto checkpointedPlateau = mean(withCheckpoints, PLATEAU_FROM, PLATEAU_BATCHES);
    const auto checkpointedLast =
        mean(withCheckpoints, withCheckpoints.size() - EDGE_BATCHES, EDGE_BATCHES);
    // (3) Le coût des rétrécissements, côté extension : le plateau avec points de reprise contre
    // les premiers lots, imprimé sans contrôle.
    line(stringFormat("pente avec CHECKPOINT : {} premiers lots {} ms, lots {} à {} {} ms, {} "
                      "derniers {} ms ; plateau / premiers ×{}",
        EDGE_BATCHES, checkpointedFirst, PLATEAU_FROM, PLATEAU_FROM + PLATEAU_BATCHES - 1,
        checkpointedPlateau, EDGE_BATCHES, checkpointedLast,
        checkpointedPlateau / checkpointedFirst));
    // (2) La garde : avec un point de reprise tous les 8 lots, rien ne croît.
    EXPECT_LE(checkpointedLast, SETTLE_FACTOR * checkpointedPlateau + NOISE_MS)
        << "[check: checkpoint-keeps-insert-flat] " << checkpointedLast
        << " ms par lot à la fin, " << checkpointedPlateau << " ms sur le plateau";
    // (1) Ce qui s'accumule en mémoire jusqu'au point de reprise : à la fin, sans point de reprise,
    // le coût par lot au plus 1,2 fois celui de la courbe avec points de reprise (les deux
    // courbes de la même passe, sous la même charge du poste).
    EXPECT_LE(last, SETTLE_FACTOR * checkpointedLast + NOISE_MS)
        << "[check: insert-cost-without-checkpoint-stays-close] " << last
        << " ms par lot à la fin sans point de reprise, " << checkpointedLast << " ms avec";
}

TEST_F(VectorIndexCost, MeasureDelete) {
    if (!measuring()) {
        GTEST_SKIP() << "a measure: CONCURRENCE_MEASURE=1, under `poste mesure`";
    }
    const std::vector<std::pair<std::string, int64_t>> indexed{{"Small", SMALL}, {"Large", LARGE}};
    for (const auto& [name, rows] : indexed) {
        createTable(name, rows, rows);
        line(stringFormat("création de l'index sur {} vecteurs : {} ms", rows, createIndex(name)));
    }
    createTable("Plain", LARGE, LARGE);
    const auto round = [&](const std::string& name, int64_t rows, const std::string& state) {
        for (const auto count : {1, 10, 45}) {
            const auto ms = deleteInOneQuery(name, takeIds(name, count));
            line(stringFormat("suppression taille={} état={} lot={} forme=une-requête "
                              "ms_par_ligne={} ms_total={}",
                rows, state, count, ms / count, ms));
        }
        const auto ms = deleteLineByLine(name, takeIds(name, 10));
        line(stringFormat("suppression taille={} état={} lot=10 forme=une-requête-par-ligne "
                          "ms_par_ligne={} ms_total={}",
            rows, state, ms / 10, ms));
    };
    for (const auto& [name, rows] : indexed) {
        round(name, rows, "index-non-checkpointé");
    }
    round("Plain", LARGE, "sans-index-non-checkpointé");
    ok("CHECKPOINT;");
    for (const auto& [name, rows] : indexed) {
        round(name, rows, "index-checkpointé");
    }
    round("Plain", LARGE, "sans-index-checkpointé");
    for (const auto& [name, rows] : indexed) {
        const auto ids = takeIds(name, 45);
        const auto nullMs = timed([&] {
            ok(stringFormat("UNWIND {} AS i MATCH (n:{} {id: i}) SET n.vec = NULL;", listOf(ids),
                name));
        });
        const auto ms = deleteInOneQuery(name, ids);
        line(stringFormat("suppression taille={} état=vecteur-NULL lot=45 forme=une-requête "
                          "ms_par_ligne={} ms_total={} (SET NULL avant : {} ms)",
            rows, ms / 45, ms, nullMs));
    }
}

TEST_F(VectorIndexCost, MeasureInsert) {
    if (!measuring()) {
        GTEST_SKIP() << "a measure: CONCURRENCE_MEASURE=1, under `poste mesure`";
    }
    constexpr int64_t BATCH = 500;
    constexpr int64_t TAIL = 2000;
    for (const auto threads : {1, 8, 32}) {
        ok(stringFormat("CALL threads={};", threads));
        const auto full = stringFormat("Full{}", threads);
        createTable(full, LARGE, LARGE);
        line(stringFormat("fils={} création de l'index sur {} vecteurs : {} ms", threads, LARGE,
            createIndex(full)));
        // Le début du remplissage (index vide) et la fin (13 000 vecteurs déjà indexés), chaque
        // fois avec et sans index.
        for (const auto& [phase, rows] :
            std::vector<std::pair<std::string, int64_t>>{{"début", TAIL}, {"fin", LARGE}}) {
            for (const auto withIndex : {true, false}) {
                const auto name = stringFormat("{}{}{}", phase == "début" ? "Start" : "End",
                    withIndex ? "Indexed" : "Plain", threads);
                createTable(name, rows, rows - TAIL);
                if (withIndex) {
                    createIndex(name);
                }
                for (auto batch = 0; batch < TAIL / BATCH; ++batch) {
                    std::vector<int64_t> ids;
                    const auto first = rows - TAIL + batch * BATCH;
                    for (auto id = first; id < first + BATCH; ++id) {
                        ids.push_back(id);
                    }
                    const auto ms = setFromNull(name, ids);
                    line(stringFormat("insertion fils={} phase={} index={} lot={} vecteurs-déjà={} "
                                      "ms_par_vecteur={} ms_total={}",
                        threads, phase, withIndex ? "oui" : "non", batch, first, ms / BATCH, ms));
                }
            }
        }
    }
}

// L'insertion de MeasureInsert, avec un point de reprise entre chaque lot de 500 (hypothèse de
// hotfixs : si le coût par vecteur reste à celui du premier lot, c'est la lecture des arêtes
// écrites depuis le dernier point de reprise qui le fait monter).
TEST_F(VectorIndexCost, MeasureInsertWithCheckpoints) {
    if (!measuring()) {
        GTEST_SKIP() << "a measure: CONCURRENCE_MEASURE=1, under `poste mesure`";
    }
    constexpr int64_t BATCH = 500;
    constexpr int64_t TAIL = 2000;
    for (const auto& [phase, rows] :
        std::vector<std::pair<std::string, int64_t>>{{"début", TAIL}, {"fin", LARGE}}) {
        // « End » est un mot réservé du parser.
        const auto name = phase == "début" ? std::string("Head") : std::string("Tail");
        createTable(name, rows, rows - TAIL);
        createIndex(name);
        line(stringFormat("insertion phase={} point de reprise initial {} ms", phase,
            timed([&] { ok("CHECKPOINT;"); })));
        for (auto batch = 0; batch < TAIL / BATCH; ++batch) {
            std::vector<int64_t> ids;
            const auto first = rows - TAIL + batch * BATCH;
            for (auto id = first; id < first + BATCH; ++id) {
                ids.push_back(id);
            }
            const auto ms = setFromNull(name, ids);
            const auto checkpointMs = timed([&] { ok("CHECKPOINT;"); });
            line(stringFormat("insertion avec-point-de-reprise phase={} lot={} vecteurs-déjà={} "
                              "ms_par_vecteur={} ms_total={} point_de_reprise_ms={}",
                phase, batch, first, ms / BATCH, ms, checkpointMs));
        }
    }
}

} // namespace
