// Des lecteurs balaient une table pendant qu'on lui ajoute des colonnes (ticket
// docs/coeur-cpp/tickets/2026-11-18-un-add-column-realloue-les-colonnes-d-une-table-sous-un-lecteur.md).
// NodeTable::addColumn agrandit le vecteur de ses colonnes ; un lecteur qui prépare son balayage le
// lit au même moment (NodeTableScanState::setToTable → NodeTable::getColumn). Les lecteurs ne
// prennent pas le verrou de schéma de la table. TSan l'a vu le 16 novembre sous DdlEveryMinute. Ce
// cas en est le témoin réduit : sous TSan, aucune course sur le vecteur des colonnes ; en Release,
// aucune lecture refusée et toutes les colonnes ajoutées présentes.
#ifndef __SINGLE_THREADED__

#include <atomic>
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

class AddColumnUnderReaders : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        run("CREATE NODE TABLE T(id INT64 PRIMARY KEY, s STRING);");
        run("COPY T FROM (UNWIND range(0, 1999) AS i RETURN i, 's' + CAST(i AS STRING));");
        run("CHECKPOINT;");
    }
    void run(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
};

TEST_F(AddColumnUnderReaders, ReadersScanWhileColumnsAreAdded) {
    constexpr int NUM_READERS = 4;
    constexpr int NUM_COLUMNS = 60;
    std::atomic<bool> done{false};
    std::atomic<int64_t> scans{0};
    std::atomic<int64_t> refused{0};
    std::vector<std::thread> readers;
    for (auto r = 0; r < NUM_READERS; ++r) {
        readers.emplace_back([&] {
            Connection reader(database.get());
            while (!done.load()) {
                auto result = reader.query("MATCH (n:T) RETURN count(n.s);");
                if (!result->isSuccess()) {
                    refused++;
                }
                scans++;
            }
        });
    }
    for (auto c = 0; c < NUM_COLUMNS; ++c) {
        run(stringFormat("ALTER TABLE T ADD c{} INT64 DEFAULT {};", c, c));
    }
    done = true;
    for (auto& reader : readers) {
        reader.join();
    }
    EXPECT_GT(scans.load(), 0) << "[check: setup] no reader ran";
    EXPECT_EQ(refused.load(), 0) << "[check: readers-are-not-refused-by-an-add-column]";
    auto result = conn->query(stringFormat("MATCH (n:T) WHERE n.c{} = {} RETURN count(*);",
        NUM_COLUMNS - 1, NUM_COLUMNS - 1));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    EXPECT_EQ(result->getNext()->getValue(0)->getValue<int64_t>(), 2000)
        << "[check: every-added-column-reads-its-default]";
}

} // namespace testing
} // namespace rag3db

#endif
