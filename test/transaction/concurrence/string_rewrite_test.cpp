// La réécriture d'une colonne de chaînes sur des lignes existantes (le P1 du produit, 12 octobre
// 2026 : le texte des morceaux de code semblait perdu après une réingestion). La version courte du
// banc, quelques secondes : les variantes longues (trente combinaisons, vertes sur master de la
// nuit du 12 octobre, lues sans filtre) sont dans les notes du banc. Le texte perdu venait en fait
// de la lecture filtrée (StringColumn::scanFiltered, son témoin à part) ; ce témoin garde
// l'écriture.
//
// Deux groupes et plus de lignes par COPY, des chaînes courtes, longues et répétées ; MERGE … SET
// de la chaîne et d'une autre colonne sur chaque ligne, par lots ; un point de reprise ; une
// réouverture ; la relecture par clé et le balayage non filtré, identiques à l'attendu.

#include <algorithm>
#include <atomic>
#include <fstream>
#include <iostream>
#include <thread>
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
            auto row =
                conn->query(stringFormat("MATCH (n:Chunk {_uuid: 'c-{}'}) RETURN n._text;", id));
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
        ok("UNWIND [" + rows +
            "] AS r MERGE (n:Chunk {_uuid: r.u}) SET n._text = r.t, n.marker = r.m;");
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

// Le point de reprise d'une colonne de chaînes après un ajout de colonne (ticket
// docs/coeur-cpp/tickets/2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-
// colonne-de-chaines.md, core 3980144) : sous TwentyWriters.DdlEveryMinute, le point de reprise
// automatique d'un COMMIT a lu, dans la colonne en mémoire des écritures non reprises
// (Column::checkpointSegment → StringChunkData::write), une chaîne au-delà de son tampon. Les cas
// suivants mettent un ADD COLUMN entre les écritures et le point de reprise.
class StringRewriteAcrossAddColumn : public StringRewrite {
protected:
    void load() {
        const auto csv = databasePath + ".chunks.csv";
        {
            std::ofstream out(csv);
            for (int64_t id = 0; id < NUM_ROWS; ++id) {
                out << "c-" << id << "," << textOf(id, 0) << ",\n";
            }
        }
        ok("COPY Chunk FROM '" + csv + "' (header=false);");
    }

    // Les lignes [first, last) réécrites au tour 1, par lots de 64, sur la connexion donnée.
    static void rewrite(rag3db::main::Connection& connection, int64_t first, int64_t last) {
        for (auto start = first; start < last; start += 64) {
            std::string rows;
            for (auto id = start; id < std::min(last, start + 64); ++id) {
                rows += (rows.empty() ? "" : ", ") +
                        stringFormat("{u: 'c-{}', t: '{}', m: 'm-{}'}", id, textOf(id, 1), id);
            }
            auto result = connection.query("UNWIND [" + rows +
                                           "] AS r MERGE (n:Chunk {_uuid: r.u}) SET n._text = r.t, "
                                           "n.marker = r.m;");
            ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        }
    }

    void expectAllRight(const std::string& when) {
        std::string summary;
        EXPECT_EQ(wrongRows(summary), 0) << "[check: " << when << "] " << summary;
    }

    void expectDefault(const std::string& column, const std::string& value) {
        auto result = conn->query("MATCH (n:Chunk) WHERE n." + column + " <> '" + value +
                                  "' OR n." + column + " IS NULL RETURN count(n);");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        EXPECT_EQ(result->getNext()->getValue(0)->getValue<int64_t>(), 0)
            << "des lignes ne lisent pas le défaut de " << column;
    }
};

// Une réécriture, un ADD avec défaut, une seconde réécriture, puis le point de reprise.
TEST_F(StringRewriteAcrossAddColumn, RewriteThenAddThenCheckpoint) {
    load();
    rewrite(*conn, 0, NUM_ROWS / 2);
    ok("ALTER TABLE Chunk ADD note STRING DEFAULT 'n-defaut';");
    rewrite(*conn, NUM_ROWS / 2, NUM_ROWS);
    expectAllRight("before-checkpoint");
    ok("CHECKPOINT;");
    expectAllRight("after-checkpoint");
    expectDefault("note", "n-defaut");
    conn.reset();
    database.reset();
    open();
    expectAllRight("after-reopen");
    expectDefault("note", "n-defaut");
}

// Une transaction ouverte réécrit toutes les lignes ; une autre connexion ajoute une colonne
// pendant ce temps ; la transaction valide ; puis le point de reprise.
TEST_F(StringRewriteAcrossAddColumn, AnOpenWriterCommitsAfterAnAdd) {
    load();
    rag3db::main::Connection writer(database.get());
    ASSERT_TRUE(writer.query("BEGIN TRANSACTION;")->isSuccess());
    rewrite(writer, 0, NUM_ROWS);
    std::string alterError;
    std::thread alter([&] {
        rag3db::main::Connection other(database.get());
        auto result = other.query("ALTER TABLE Chunk ADD note STRING DEFAULT 'n-defaut';");
        if (!result->isSuccess()) {
            alterError = result->getErrorMessage();
        }
    });
    std::this_thread::sleep_for(std::chrono::milliseconds(200));
    auto commit = writer.query("COMMIT;");
    alter.join();
    ASSERT_TRUE(commit->isSuccess()) << commit->getErrorMessage();
    std::cout << "[témoin] ADD pendant la transaction : "
              << (alterError.empty() ? "accepté" : "refusé : " + alterError) << std::endl;
    expectAllRight("before-checkpoint");
    ok("CHECKPOINT;");
    expectAllRight("after-checkpoint");
    conn.reset();
    database.reset();
    open();
    expectAllRight("after-reopen");
}

// Le point de reprise automatique demandé par les COMMIT eux-mêmes, des ADD et des DROP entre les
// lots, et un lecteur qui balaie la table pendant ce temps.
TEST_F(StringRewriteAcrossAddColumn, AutomaticCheckpointsBetweenAddsAndDrops) {
    load();
    ok("CALL auto_checkpoint=true;");
    ok("CALL checkpoint_threshold=4096;");
    std::atomic<bool> done{false};
    std::thread reader([&] {
        rag3db::main::Connection scan(database.get());
        while (!done.load()) {
            auto result = scan.query("MATCH (n:Chunk) RETURN sum(size(n._text));");
            (void)result;
        }
    });
    const auto step = NUM_ROWS / 5;
    for (int64_t round = 0; round < 5; ++round) {
        rewrite(*conn, round * step, (round + 1) * step);
        const auto column = "c" + std::to_string(round);
        ok("ALTER TABLE Chunk ADD " + column + " STRING DEFAULT 'd-" + column + "';");
        if (round % 2 == 1) {
            ok("ALTER TABLE Chunk DROP c" + std::to_string(round - 1) + ";");
        }
    }
    done = true;
    reader.join();
    expectAllRight("after-automatic-checkpoints");
    ok("CHECKPOINT;");
    expectAllRight("after-checkpoint");
    expectDefault("c4", "d-c4");
    conn.reset();
    database.reset();
    open();
    expectAllRight("after-reopen");
    expectDefault("c4", "d-c4");
}

} // namespace
