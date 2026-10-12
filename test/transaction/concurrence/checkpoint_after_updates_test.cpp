// Le point de reprise après de nombreux SET d'une colonne (ticket du 12 octobre 2026, tiré du
// témoin du coût vectoriel : le CHECKPOINT qui suit 15 000 SET de vecteurs depuis NULL prend 19 à
// 25 s, ColumnChunkData::resize y fait 80 % du temps propre).
//
// La lecture du code : au point de reprise, LazySegmentScanner::applyCommittedUpdates applique les
// mises à jour validées ligne à ligne (segmentData->write(..., 1)) ; pour une colonne de listes ou
// de tableaux, ListChunkData::write agrandit alors la donnée au plus juste
// (dataColumnChunk->resize(numValues + taille)), et ColumnChunkData::resize réalloue et recopie tout
// le tampon. Le coût serait quadratique en nombre de lignes mises à jour.
//
// - Columns/CheckpointAfterUpdates.ScalesWithTheUpdatedRows : le témoin, une colonne par cas
//   (FLOAT[768], STRING, INT64). Deux tables de 1 500 et 15 000 lignes chargées par COPY, la
//   colonne NULL, puis SET de chaque ligne par lots de 64, sans point de reprise ; puis un
//   CHECKPOINT chronométré par table. Pour dix fois plus de lignes, un coût proportionnel au
//   volume écrit fait environ dix fois plus, un coût quadratique cent fois : la grande table doit
//   rester sous vingt fois la petite (plus 50 ms de marge contre le bruit). La référence, imprimée :
//   le même volume écrit par COPY puis CHECKPOINT.

#include <algorithm>
#include <chrono>
#include <filesystem>
#include <fstream>
#include <functional>
#include <iostream>
#include <string>
#include <vector>

#include "common/build_id.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr int64_t SMALL = 1500;
constexpr int64_t LARGE = 15000;
constexpr int64_t BATCH = 64;
constexpr int DIM = 768;
constexpr double SCALE_FACTOR = 20;
constexpr double NOISE_MS = 50;

enum class Kind { VECTOR, STRING, INT64 };

class CheckpointAfterUpdates : public EmptyDBTest, public ::testing::WithParamInterface<Kind> {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed (checkpoint)";
        }
        systemConfig->bufferPoolSize = 2ull * 1024 * 1024 * 1024;
        createDBAndConn();
        ok("CALL auto_checkpoint=false;");
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
        std::cerr << "  [point de reprise] " << text << " bati=" << RAG3DB_BUILD_ID << "\n";
    }

    static std::string typeName() {
        switch (GetParam()) {
        case Kind::VECTOR:
            return stringFormat("FLOAT[{}]", DIM);
        case Kind::STRING:
            return "STRING";
        case Kind::INT64:
            return "INT64";
        }
        return "?";
    }

    // La valeur de la ligne `id`, telle qu'un CSV l'écrit.
    static std::string csvValueOf(int64_t id) {
        switch (GetParam()) {
        case Kind::VECTOR: {
            std::string out = "\"[";
            for (auto d = 0; d < DIM; ++d) {
                out += (d ? "," : "") + std::to_string((id + d) % 97 / 97.0);
            }
            return out + "]\"";
        }
        case Kind::STRING:
            return std::string(200, 'x') + std::to_string(id);
        case Kind::INT64:
            return std::to_string(id * 7);
        }
        return "";
    }

    // La même valeur, en expression Cypher sur la ligne `i`.
    static std::string cypherValue() {
        switch (GetParam()) {
        case Kind::VECTOR:
            return stringFormat("CAST(list_transform(range(0, {}), d -> CAST((i + d) % 97 / 97.0 "
                                "AS FLOAT)) AS FLOAT[{}])",
                DIM - 1, DIM);
        case Kind::STRING:
            return stringFormat("'{}' + CAST(i AS STRING)", std::string(200, 'x'));
        case Kind::INT64:
            return "i * 7";
        }
        return "";
    }

    void load(const std::string& name, int64_t rows, bool withValues) {
        ok(stringFormat("CREATE NODE TABLE {}(id INT64 PRIMARY KEY, v {});", name, typeName()));
        const auto path = databasePath + "." + name + ".csv";
        {
            std::ofstream out(path);
            for (int64_t id = 0; id < rows; ++id) {
                out << id << "," << (withValues ? csvValueOf(id) : "") << "\n";
            }
        }
        ok("COPY " + name + " FROM '" + path + "' (header=false);");
        std::filesystem::remove(path);
    }

    double setInBatches(const std::string& name, int64_t rows) {
        return timed([&] {
            for (int64_t first = 0; first < rows; first += BATCH) {
                ok(stringFormat("UNWIND range({}, {}) AS i MATCH (n:{} {id: i}) SET n.v = {};",
                    first, std::min(rows, first + BATCH) - 1, name, cypherValue()));
            }
        });
    }

    double checkpoint() {
        return timed([&] { ok("CHECKPOINT;"); });
    }
};

TEST_P(CheckpointAfterUpdates, ScalesWithTheUpdatedRows) {
    std::vector<double> afterSet;
    for (const auto rows : {SMALL, LARGE}) {
        const auto name = stringFormat("Set{}", rows);
        load(name, rows, false);
        ok("CHECKPOINT;");
        const auto setMs = setInBatches(name, rows);
        const auto checkpointMs = checkpoint();
        afterSet.push_back(checkpointMs);
        line(stringFormat("{} lignes={} SET par lots de {} : {} ms, puis CHECKPOINT {} ms",
            typeName(), rows, BATCH, setMs, checkpointMs));
        const auto copied = stringFormat("Copy{}", rows);
        const auto copyMs = timed([&] { load(copied, rows, true); });
        line(stringFormat("{} lignes={} référence COPY : {} ms, puis CHECKPOINT {} ms", typeName(),
            rows, copyMs, checkpoint()));
    }
    EXPECT_LE(afterSet[1], SCALE_FACTOR * afterSet[0] + NOISE_MS)
        << "[check: checkpoint-linear-in-updated-rows] " << typeName() << " : " << afterSet[1]
        << " ms après " << LARGE << " SET, " << afterSet[0] << " ms après " << SMALL;
}

INSTANTIATE_TEST_SUITE_P(Columns, CheckpointAfterUpdates,
    ::testing::Values(Kind::VECTOR, Kind::STRING, Kind::INT64),
    [](const ::testing::TestParamInfo<Kind>& info) {
        switch (info.param) {
        case Kind::VECTOR:
            return std::string("Vector768");
        case Kind::STRING:
            return std::string("String");
        case Kind::INT64:
            return std::string("Int64");
        }
        return std::string("?");
    });

} // namespace
