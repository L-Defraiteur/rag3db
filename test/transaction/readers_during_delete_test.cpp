// Un seul écrivain, des lecteurs dans d'autres fils du même processus (marche A5). C'est
// le régime du démon : il sert des lectures pendant qu'une ingestion supprime et réécrit.
// Aucun mode multi-écrivains ici. Les informations de version d'un bloc de lignes naissent
// à la première suppression ; avant A5, un lecteur les consultait sans aucune garde
// pendant que l'écrivain les créait et les remplissait. Le test vérifie ce qu'un lecteur
// a le droit de voir — jamais plus de lignes qu'avant, jamais moins que ce qui restera —
// et c'est sous ThreadSanitizer qu'il prouve la course : voir le commit de la marche.

#include <atomic>
#include <string>
#include <thread>
#include <vector>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::testing;

namespace {

class ReadersDuringDeleteTest : public EmptyDBTest {
protected:
    static constexpr int64_t NUM_ROWS = 20000;
    static constexpr int64_t NUM_DELETES = 2000;
    static constexpr int64_t DELETE_STRIDE = NUM_ROWS / NUM_DELETES;

    void SetUp() override {
        EmptyDBTest::SetUp();
        // Sous ThreadSanitizer, l'espace d'adressage ne suffit pas aux 8 To par défaut.
        systemConfig->maxDBSize = 1024ull * 1024 * 1024 * 1024;
        createDBAndConn();
        ok(*conn, "CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);");
        ok(*conn, "UNWIND range(1, " + std::to_string(NUM_ROWS) +
                      ") AS k CREATE (:Item {id: k, v: 1});");
        // Les lignes viennent du disque : leurs blocs n'ont pas encore d'informations de
        // version, la première suppression de chaque bloc les crée sous les lecteurs.
        ok(*conn, "CHECKPOINT;");
    }

    static void ok(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
};

TEST_F(ReadersDuringDeleteTest, ScansAndLookupsWhileOneWriterDeletes) {
    std::atomic<bool> writerDone{false};
    std::atomic<int64_t> numFailures{0};
    std::atomic<int64_t> numReads{0};

    // Un lecteur qui balaie : la somme de v est le nombre de lignes qu'il voit.
    auto scanReader = [&]() {
        rag3db::main::Connection reader(database.get());
        int64_t previous = NUM_ROWS;
        while (!writerDone.load()) {
            auto result = reader.query("MATCH (i:Item) RETURN sum(i.v);");
            if (!result->isSuccess() || !result->hasNext()) {
                numFailures++;
                continue;
            }
            auto seen = result->getNext()->getValue(0)->getValue<int64_t>();
            if (seen > previous || seen < NUM_ROWS - NUM_DELETES) {
                numFailures++;
            }
            previous = seen;
            numReads++;
        }
    };
    // Un lecteur qui cherche par clé : une ligne supprimée ne revient pas.
    auto lookupReader = [&]() {
        rag3db::main::Connection reader(database.get());
        std::vector<bool> gone(NUM_DELETES + 1, false);
        int64_t turn = 0;
        while (!writerDone.load()) {
            auto slot = 1 + (turn++ % NUM_DELETES);
            auto result = reader.query("MATCH (i:Item {id: " +
                                       std::to_string(slot * DELETE_STRIDE) + "}) RETURN i.v;");
            if (!result->isSuccess()) {
                numFailures++;
                continue;
            }
            auto present = result->hasNext();
            if (present && gone[slot]) {
                numFailures++;
            }
            gone[slot] = !present;
            numReads++;
        }
    };

    std::vector<std::thread> readers;
    readers.emplace_back(scanReader);
    readers.emplace_back(scanReader);
    readers.emplace_back(lookupReader);

    for (int64_t slot = 1; slot <= NUM_DELETES; slot++) {
        ok(*conn, "MATCH (i:Item {id: " + std::to_string(slot * DELETE_STRIDE) + "}) DELETE i;");
    }
    writerDone.store(true);
    for (auto& reader : readers) {
        reader.join();
    }

    EXPECT_EQ(numFailures.load(), 0);
    EXPECT_GT(numReads.load(), 0);
    auto result = conn->query("MATCH (i:Item) RETURN sum(i.v);");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    EXPECT_EQ(result->getNext()->getValue(0)->getValue<int64_t>(), NUM_ROWS - NUM_DELETES);
}

} // namespace
