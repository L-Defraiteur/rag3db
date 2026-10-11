// A9 (condition 3, page 11) : le coût du mode multi-écrivains, mesuré avant de l'allumer par
// défaut. Le mode ajoute, par ligne écrite, une clé imprimée et une entrée dans la table des
// verrous sous un mutex, sans attente quand l'écrivain est seul ; un COPY ne prend qu'un verrou
// d'index. Trois mesures, chacune jouée mode éteint (un seul écrivain, le défaut d'aujourd'hui)
// puis mode allumé, sur une base neuve à chaque fois :
//
//  1. mille SET d'un vecteur vers un autre, ligne à ligne, sous un index HNSW (le témoin
//     SetToAnotherVectorLineByLine de vector_index_update_test.cpp, en une seule passe) ;
//  2. un COPY de 200 000 lignes depuis un fichier CSV ;
//  3. 100 000 insertions ligne à ligne, en auto-commit, par une instruction préparée.
//
// C'est une mesure, pas un témoin : elle ne tourne qu'avec CONCURRENCE_MEASURE=1 dans
// l'environnement (sinon sautée), sous la tenue exclusive du poste, et se répète par
// --gtest_repeat. Chaque chiffre sort sur une ligne « mesure: <cas> mode=<éteint|allumé>
// ms=<durée> bati=<identifiant> » : l'identifiant de bâti dit quel moteur a été mesuré, et
// l'extension vector chargée porte le même (LOAD EXTENSION la refuse sinon). Le protocole et
// les chiffres lus sont dans la page 11 du cœur C++.

#ifndef __SINGLE_THREADED__

#include <chrono>
#include <cstdlib>
#include <fstream>
#include <functional>
#include <iostream>

#include "bench_harness.h"
#include "common/build_id.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/client_context.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr auto ORIGIN_VECTOR = "[n.id % 17, n.id % 23, n.id % 29, n.id]";
constexpr auto OTHER_VECTOR = "[n.id % 13 + 0.5, n.id % 19, n.id % 31, n.id + 0.25]";

// Le paramètre : le mode multi-écrivains est-il allumé ?
class LockCost : public EmptyDBTest, public ::testing::WithParamInterface<bool> {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (std::getenv("CONCURRENCE_MEASURE") == nullptr) {
            GTEST_SKIP() << "a measure, not a witness: CONCURRENCE_MEASURE=1 to run it";
        }
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        systemConfig->bufferPoolSize = 1024 * 1024 * 1024;
        createDBAndConn();
        mustRun("CALL auto_checkpoint=false;");
        mustRun(GetParam() ? "CALL enable_multi_writes=true;" :
                             "CALL enable_multi_writes=false;");
    }

    void mustRun(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << "[check: query] " << query.substr(0, 200) << "\n"
                                         << result->getErrorMessage();
    }

    int64_t queryInt(const std::string& query) {
        auto result = conn->query(query);
        if (!result->isSuccess() || !result->hasNext()) {
            return -1;
        }
        return result->getNext()->getValue(0)->getValue<int64_t>();
    }

    // Joue `work` et imprime sa durée sur la ligne de mesure.
    void measure(const std::string& name, const std::function<void()>& work) {
        const auto start = std::chrono::steady_clock::now();
        work();
        const auto ms = std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::steady_clock::now() - start)
                            .count();
        std::cout << "mesure: " << name << " mode=" << (GetParam() ? "allumé" : "éteint")
                  << " ms=" << ms << " bati=" << RAG3DB_BUILD_ID << "\n";
    }
};

} // namespace

// 1. Mille SET d'un vecteur vers un autre, ligne à ligne, sous index : le chemin de l'index
// vectoriel juste en service (une ligne verrouillée par instruction, plus l'index de la table).
TEST_P(LockCost, SetToAnotherVectorLineByLine) {
    concurrency::loadVectorExtension(*conn);
    mustRun("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
    mustRun(stringFormat("UNWIND range(0, 999) AS i CREATE (n:Doc {id: i}) SET n.vec = {};",
        ORIGIN_VECTOR));
    mustRun("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    measure("set-vecteur-1000-ligne-a-ligne", [&] {
        for (auto id = 0; id < 1000; ++id) {
            mustRun(stringFormat("MATCH (n:Doc) WHERE n.id = {} SET n.vec = {};", id,
                OTHER_VECTOR));
        }
    });
    EXPECT_EQ(queryInt("MATCH (n:Doc) WHERE n.vec[1] = CAST(n.id % 13 + 0.5, 'FLOAT') "
                       "RETURN count(n);"),
        1000)
        << "[check: rows-set] ";
}

// 2. Un COPY de 200 000 lignes : le mode n'y prend qu'un verrou d'index, le coût attendu est nul.
TEST_P(LockCost, CopyOf200000Rows) {
    constexpr int64_t NUM_ROWS = 200'000;
    const auto path = TestHelper::getTempDir("lock_cost") + "/items.csv";
    {
        std::ofstream csv(path);
        for (int64_t id = 0; id < NUM_ROWS; ++id) {
            csv << id << ",item" << id << "," << (id % 97) * 0.25 << "\n";
        }
    }
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, name STRING, score DOUBLE);");
    measure("copy-200000", [&] { mustRun("COPY Item FROM '" + path + "' (header=false);"); });
    EXPECT_EQ(queryInt("MATCH (n:Item) RETURN count(n);"), NUM_ROWS) << "[check: rows-copied] ";
}

// 3. 100 000 insertions ligne à ligne en auto-commit, par une instruction préparée (l'analyse de
// la requête ne se paie qu'une fois : ce qui reste est l'écriture, et le verrou).
TEST_P(LockCost, Insert100000RowsOneByOne) {
    constexpr int64_t NUM_ROWS = 100'000;
    mustRun("CREATE NODE TABLE Item(id INT64 PRIMARY KEY, name STRING);");
    auto prepared = conn->prepare("CREATE (:Item {id: $id, name: $name});");
    ASSERT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
    measure("insert-100000-ligne-a-ligne", [&] {
        for (int64_t id = 0; id < NUM_ROWS; ++id) {
            auto result = conn->execute(prepared.get(), std::make_pair(std::string("id"), id),
                std::make_pair(std::string("name"), std::string("item") + std::to_string(id)));
            ASSERT_TRUE(result->isSuccess()) << "[check: query] row " << id << "\n"
                                             << result->getErrorMessage();
        }
    });
    EXPECT_EQ(queryInt("MATCH (n:Item) RETURN count(n);"), NUM_ROWS)
        << "[check: rows-inserted] ";
}

INSTANTIATE_TEST_SUITE_P(Mode, LockCost, ::testing::Bool(),
    [](const ::testing::TestParamInfo<bool>& info) {
        return info.param ? "MultiWrites" : "SingleWriter";
    });

#endif
