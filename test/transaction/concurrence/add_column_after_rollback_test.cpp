// Une colonne ajoutée sous écrivains, juste après un ADD annulé, se relit sans mourir (ticket
// docs/coeur-cpp/tickets/2026-11-15-ddl-every-minute-meurt-d-un-sigsegv-pile-perdue.md). Le scénario
// des vingt écrivains meurt d'un SIGSEGV dans DefaultColumnReadWriter::readCompressedValuesToVector,
// sous le count qui relit la colonne b (minute 6 : BEGIN ; ADD a ; ADD a refusé ; ROLLBACK ; puis
// ADD b DEFAULT), sur master comme avec le verrou de schéma. Ce témoin rejoue cette minute seule,
// tour après tour, sous des écrivains qui fusionnent des lignes neuves et anciennes.
#ifndef __SINGLE_THREADED__

#include <atomic>
#include <random>
#include <string>
#include <thread>
#include <vector>

#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::main;

namespace rag3db {
namespace testing {

namespace {

constexpr int64_t NUM_ROWS = 20'000;
constexpr int64_t KEY_RANGE = NUM_ROWS * 6 / 5;
constexpr int NUM_WRITERS = 8;
constexpr int NUM_ROUNDS = 6;

int64_t countOf(Connection& connection, const std::string& query) {
    auto result = connection.query(query);
    if (!result->isSuccess() || !result->hasNext()) {
        ADD_FAILURE() << query << " : " << result->getErrorMessage();
        return -1;
    }
    return result->getNext()->getValue(0)->getValue<int64_t>();
}

} // namespace

class AddColumnAfterRollback : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
    }
    void mustRun(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
};

TEST_F(AddColumnAfterRollback, AColumnAddedUnderWritersAfterARolledBackAddReadsBack) {
    mustRun(*conn, "CALL auto_checkpoint=false;");
    for (auto round = 0; round < NUM_ROUNDS; ++round) {
        const auto table = stringFormat("D{}", round);
        mustRun(*conn, stringFormat("CREATE NODE TABLE {}(id INT64 PRIMARY KEY, titre STRING);",
                           table));
        mustRun(*conn, stringFormat("COPY {} FROM (UNWIND range(0, {}) AS i RETURN i, 'titre ' + "
                                    "CAST(i AS STRING));",
                           table, NUM_ROWS - 1));
        mustRun(*conn, "CHECKPOINT;");
        std::atomic<bool> stop{false};
        std::vector<std::thread> writers;
        for (auto w = 0; w < NUM_WRITERS; ++w) {
            writers.emplace_back([&, w] {
                Connection connection(database.get());
                std::mt19937 engine(static_cast<uint32_t>(1000 * round + w));
                std::uniform_int_distribution<int64_t> key(0, KEY_RANGE - 1);
                for (int64_t i = 0; !stop.load(); ++i) {
                    const auto query = stringFormat(
                        "MERGE (d:{} {id: {}}) SET d.titre = 'titre {} {}';", table, key(engine),
                        w, i);
                    // Un refus de schéma changé se recommence, comme le fera rag3weaver.
                    for (auto tries = 0; tries < 100; ++tries) {
                        auto result = connection.query(query);
                        if (result->isSuccess() ||
                            result->getErrorMessage().find("Schema changed, retry:") ==
                                std::string::npos) {
                            break;
                        }
                    }
                }
            });
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(400));
        {
            Connection migration(database.get());
            mustRun(migration, "BEGIN TRANSACTION;");
            mustRun(migration, stringFormat("ALTER TABLE {} ADD a STRING;", table));
            auto second = migration.query(stringFormat("ALTER TABLE {} ADD a STRING;", table));
            EXPECT_FALSE(second->isSuccess()) << "[check: setup] the second ADD a passed";
            migration.query("ROLLBACK;");
            mustRun(migration, stringFormat("ALTER TABLE {} ADD b STRING DEFAULT 'b-defaut';",
                                   table));
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(400));
        stop = true;
        for (auto& writer : writers) {
            writer.join();
        }
        mustRun(*conn, stringFormat("MATCH (d:{}) WHERE d.id < 10 SET d.b = concat('b-ecrit-', "
                                    "CAST(d.id AS STRING));",
                           table));
        // Le count du scénario, celui sous lequel le processus meurt.
        EXPECT_EQ(countOf(*conn, stringFormat("MATCH (d:{}) WHERE d.id < 10 AND d.b = "
                                              "concat('b-ecrit-', CAST(d.id AS STRING)) RETURN "
                                              "count(d);",
                                     table)),
            10)
            << "[check: added-column-reads-what-was-set] round " << round;
        EXPECT_EQ(countOf(*conn, stringFormat("MATCH (d:{}) WHERE d.id < {} AND d.id >= 10 AND "
                                              "(d.b IS NULL OR d.b <> 'b-defaut') RETURN count(d);",
                                     table, NUM_ROWS)),
            0)
            << "[check: added-column-reads-its-default] round " << round;
    }
}

} // namespace testing
} // namespace rag3db

#endif
