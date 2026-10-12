// La réécriture d'une colonne de chaînes sur des lignes existantes (le P1 du produit, 12 octobre
// 2026 : le texte des morceaux de code semblait perdu après une réingestion). La version courte du
// banc, quelques secondes : les variantes longues (trente combinaisons, vertes sur master de la nuit
// du 12 octobre, lues sans filtre) sont dans les notes du banc. Le texte perdu venait en fait de la
// lecture filtrée (StringColumn::scanFiltered, son témoin à part) ; ce témoin garde l'écriture.
//
// Deux groupes et plus de lignes par COPY, des chaînes courtes, longues et répétées ; MERGE … SET de
// la chaîne et d'une autre colonne sur chaque ligne, par lots ; un point de reprise ; une
// réouverture ; la relecture par clé et le balayage non filtré, identiques à l'attendu.

#include <algorithm>
#include <fstream>
#include <sstream>
#include <string>

#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr int64_t NUM_ROWS = 5000;

std::string textOf(int64_t id, int64_t round) {
    if (id % 5 == 0) {
        return stringFormat("repeated {} {}", id % 3, round);
    }
    std::string out;
    const auto length = id % 3 == 0 ? 400 + id % 3000 : 1 + id % 40;
    for (int64_t i = 0; i < length; ++i) {
        out += static_cast<char>('a' + (id * 31 + i * 7 + round) % 26);
    }
    return out;
}

class StringRewrite : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed (checkpoint and reopen)";
        }
        open();
        ok("CREATE NODE TABLE Chunk(_uuid STRING PRIMARY KEY, _text STRING, marker STRING);");
    }

    void open() {
        createDBAndConn();
        ok("CALL auto_checkpoint=false;");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess())
            << "[check: query] " << query.substr(0, 160) << " : " << result->getErrorMessage();
    }

    // Les lignes fausses, par le balayage non filtré puis par la clé.
    int64_t wrongRows(std::string& summary) {
        int64_t wrong = 0;
        std::ostringstream out;
        auto scan = conn->query("MATCH (n:Chunk) RETURN n._uuid, n._text, n.marker;");
        int64_t seen = 0;
        while (scan->isSuccess() && scan->hasNext()) {
            auto tuple = scan->getNext();
            const auto id = std::stoll(tuple->getValue(0)->getValue<std::string>().substr(2));
            seen++;
            if (tuple->getValue(1)->isNull() ||
                tuple->getValue(1)->getValue<std::string>() != textOf(id, 1) ||
                tuple->getValue(2)->getValue<std::string>() != stringFormat("m-{}", id)) {
                if (wrong++ < 5) {
                    out << " [balayage, c-" << id << "]";
                }
            }
        }
        if (seen != NUM_ROWS) {
            out << " (" << seen << " lignes balayées)";
            wrong++;
        }
        for (int64_t id = 0; id < NUM_ROWS; id += 7) {
            auto row = conn->query(stringFormat("MATCH (n:Chunk {_uuid: 'c-{}'}) RETURN n._text;", id));
            if (!row->isSuccess() || !row->hasNext() ||
                row->getNext()->getValue(0)->getValue<std::string>() != textOf(id, 1)) {
                if (wrong++ < 10) {
                    out << " [clé, c-" << id << "]";
                }
            }
        }
        summary = out.str();
        return wrong;
    }
};

TEST_F(StringRewrite, MergeSetSurvivesCheckpointAndReopen) {
    const auto csv = databasePath + ".chunks.csv";
    {
        std::ofstream out(csv);
        for (int64_t id = 0; id < NUM_ROWS; ++id) {
            out << "c-" << id << "," << textOf(id, 0) << ",\n";
        }
    }
    ok("COPY Chunk FROM '" + csv + "' (header=false);");
    for (int64_t first = 0; first < NUM_ROWS; first += 64) {
        std::string rows;
        for (auto id = first; id < std::min(NUM_ROWS, first + 64); ++id) {
            rows += (rows.empty() ? "" : ", ") +
                    stringFormat("{u: 'c-{}', t: '{}', m: 'm-{}'}", id, textOf(id, 1), id);
        }
        ok("UNWIND [" + rows + "] AS r MERGE (n:Chunk {_uuid: r.u}) SET n._text = r.t, n.marker = r.m;");
    }
    std::string summary;
    EXPECT_EQ(wrongRows(summary), 0) << "[check: rewritten-strings] " << summary;
    ok("CHECKPOINT;");
    EXPECT_EQ(wrongRows(summary), 0) << "[check: rewritten-strings-after-checkpoint] " << summary;
    conn.reset();
    database.reset();
    open();
    EXPECT_EQ(wrongRows(summary), 0) << "[check: rewritten-strings-after-reopen] " << summary;
}

} // namespace
