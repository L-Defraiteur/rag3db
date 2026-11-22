#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

// Sous IGNORE_ERRORS, entre deux lignes d'un COPY qui portent la même clé, la première du fichier
// gagne (ticket docs/tickets/2026-10-10-copy-garde-un-doublon-selon-les-fils.md). Le COPY à
// plusieurs fils gardait celle qui arrivait la première dans l'index de clé primaire : l'ordre de fin
// des fils de lecture, pas celui du fichier (3 passes sur 30 gardaient la seconde, sonde du
// 10 octobre). Le témoin : 20 000 lignes (une trentaine de blocs de lecture de 8 Kio), puis trois clés
// reprises en fin de fichier ; 30 COPY à 8 fils, chacun dans sa table ; chaque clé reprise doit garder
// le nom de sa première ligne, et la table compter 20 000 lignes.

namespace rag3db {
namespace testing {

class CopyFirstLineWinsTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        systemConfig->maxNumThreads = 8;
        createDBAndConn();
        ok("CALL threads=8;");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
};

TEST_F(CopyFirstLineWinsTest, TheFirstLineOfTheFileKeepsItsKey) {
    constexpr int64_t ROWS = 20000;
    constexpr int RUNS = 30;
    const std::vector<int64_t> repeated = {17, 9000, 15000};
    const auto csv = databasePath + ".first_line_wins.csv";
    {
        std::ofstream out(csv);
        for (int64_t id = 0; id < ROWS; ++id) {
            out << id << ",first " << id << "\n";
        }
        for (const auto id : repeated) {
            out << id << ",again " << id << "\n";
        }
    }
    int wrongRuns = 0;
    std::string details;
    for (auto run = 0; run < RUNS; ++run) {
        const auto table = common::stringFormat("Doc{}", run);
        ok(common::stringFormat("CREATE NODE TABLE {}(id INT64 PRIMARY KEY, name STRING);", table));
        ok(common::stringFormat("COPY {} FROM '{}' (header=false, IGNORE_ERRORS=true);", table, csv));
        if (HasFatalFailure()) {
            return;
        }
        std::string kept;
        bool wrong = false;
        for (const auto id : repeated) {
            auto result = conn->query(
                common::stringFormat("MATCH (d:{}) WHERE d.id = {} RETURN d.name;", table, id));
            ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
            const auto name =
                result->hasNext() ? result->getNext()->getValue(0)->toString() : "<absent>";
            kept += " " + name;
            wrong |= name != common::stringFormat("first {}", id);
        }
        auto count = conn->query(common::stringFormat("MATCH (d:{}) RETURN count(*);", table));
        ASSERT_TRUE(count->isSuccess()) << count->getErrorMessage();
        const auto rows = count->getNext()->getValue(0)->getValue<int64_t>();
        wrong |= rows != ROWS;
        if (wrong) {
            wrongRuns++;
            details += common::stringFormat("\n  run {} : {} rows, kept{}", run, rows, kept);
        }
        ok(common::stringFormat("DROP TABLE {};", table));
    }
    std::filesystem::remove(csv);
    EXPECT_EQ(wrongRuns, 0) << "[check: copy-first-line-of-the-file-wins] " << wrongRuns << " of "
                            << RUNS << " parallel COPY kept a later line of a repeated key:"
                            << details;
}

} // namespace testing
} // namespace rag3db
