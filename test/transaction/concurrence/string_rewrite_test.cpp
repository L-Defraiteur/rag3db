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
#include <cstdlib>
#include <fstream>
#include <functional>
#include <iomanip>
#include <iostream>
#include <memory>
#include <mutex>
#include <sstream>
#include <string>
#include <thread>

#include "../flaky_checkpointer.h"
#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "storage/checkpointer.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr int64_t NUM_ROWS = 5000;
constexpr int64_t NEW_ROWS = 500;

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

// Sonde privée activée explicitement ; aucun accès SQL réentrant dans le checkpoint.
struct StringDiagnosticState {
    std::mutex mutex;
    std::string step = "setup";
    std::atomic<uint64_t> checkpoints{0};
    void setStep(std::string value) {
        const std::lock_guard lock(mutex);
        step = std::move(value);
    }
    std::string getStep() {
        const std::lock_guard lock(mutex);
        return step;
    }
};

class StringDiagnosticCheckpointer final : public rag3db::storage::Checkpointer {
public:
    StringDiagnosticCheckpointer(rag3db::main::ClientContext& context,
        std::shared_ptr<StringDiagnosticState> state)
        : Checkpointer(context), state(std::move(state)) {}

protected:
    bool checkpointStorage() override {
        std::cerr << "[string-diagnostic] checkpoint-before-storage sequence="
                  << ++state->checkpoints << " statement=" << state->getStep() << std::endl;
        const auto changed = Checkpointer::checkpointStorage();
        std::cerr << "[string-diagnostic] checkpoint-after-storage sequence=" << state->checkpoints
                  << " statement=" << state->getStep() << std::endl;
        return changed;
    }

private:
    std::shared_ptr<StringDiagnosticState> state;
};

std::string diagnosticValue(const Value& value) {
    if (value.isNull()) {
        return "NULL";
    }
    const auto text = value.getValue<std::string>();
    uint64_t hash = 14695981039346656037ull;
    std::ostringstream hex;
    hex << std::hex << std::setfill('0');
    for (const unsigned char byte : text) {
        hash = (hash ^ byte) * 1099511628211ull;
        hex << std::setw(2) << static_cast<unsigned>(byte);
    }
    return stringFormat("length={} fnv1a64={} hex={}", text.size(), hash, hex.str());
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
        auto scan = conn->query(
            "MATCH (n:Chunk) WHERE n._uuid STARTS WITH 'c-' RETURN n._uuid, n._text, n.marker;");
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
// (Column::checkpointSegment → StringChunkData::write → DictionaryChunk::appendString), une chaîne
// au-delà de son tampon. Les cas suivants mettent un ADD COLUMN entre les écritures et le point de
// reprise.
//
// **Ce sont des non-régressions, pas le témoin de ce plantage** : ils chassaient le core 3980144 et
// ne l'ont pas reproduit (verts sous ASAN le 14 novembre 2026, 4 sur 4). Il leur manque, par
// rapport au scénario, les vingt écrivains simultanés, les MERGE avec relations et vecteur, et
// CREATE_VECTOR_INDEX. AnOpenWriterCommitsAfterAnAdd montre en passant qu'un ADD COLUMN aboutit
// sous une transaction d'écriture ouverte (ticket
// docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-14-un-add-column-passe-sous-une-transaction-d-ecriture-ouverte.md).
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
    static void rewrite(rag3db::main::Connection& connection, int64_t first, int64_t last,
        const std::function<void(int64_t, int64_t, bool)>& observe = {}) {
        for (auto start = first; start < last; start += 64) {
            std::string rows;
            for (auto id = start; id < std::min(last, start + 64); ++id) {
                rows += (rows.empty() ? "" : ", ") +
                        stringFormat("{u: 'c-{}', t: '{}', m: 'm-{}'}", id, textOf(id, 1), id);
            }
            if (observe) {
                observe(start, std::min(last, start + 64), false);
            }
            auto result = connection.query("UNWIND [" + rows +
                                           "] AS r MERGE (n:Chunk {_uuid: r.u}) SET n._text = r.t, "
                                           "n.marker = r.m;");
            ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
            if (observe) {
                observe(start, std::min(last, start + 64), true);
            }
        }
    }

    // Trois chemins SQL distincts, mêmes valeurs attendues selon le préfixe déjà réécrit.
    void diagnose(const std::string& phase, int64_t rewrittenEnd) {
        for (const auto& mode : {std::string("scan"), std::string("prefix")}) {
            auto rows =
                conn->query("MATCH (n:Chunk) " +
                            std::string(mode == "prefix" ? "WHERE n._uuid STARTS WITH 'c-' " : "") +
                            "RETURN n._uuid, n._text, n.marker;");
            ASSERT_TRUE(rows->isSuccess())
                << "[check: diagnostic-query] " << rows->getErrorMessage();
            int64_t seen = 0;
            int64_t wrong = 0;
            while (rows->hasNext()) {
                const auto row = rows->getNext();
                const auto key = row->getValue(0)->getValue<std::string>();
                const auto id = std::stoll(key.substr(2));
                const auto expected = textOf(id, id < rewrittenEnd ? 1 : 0);
                const auto* text = row->getValue(1);
                const auto* marker = row->getValue(2);
                const bool badText = text->isNull() || text->getValue<std::string>() != expected;
                const bool badMarker = id < rewrittenEnd ?
                                           marker->isNull() || marker->getValue<std::string>() !=
                                                                   stringFormat("m-{}", id) :
                                           !marker->isNull();
                ++seen;
                if (badText || badMarker) {
                    if (wrong++ < 4) {
                        std::cerr << "[string-diagnostic] phase=" << phase << " mode=" << mode
                                  << " key=" << key
                                  << " expected-text=" << diagnosticValue(Value(expected))
                                  << " actual-text=" << diagnosticValue(*text)
                                  << " actual-marker=" << diagnosticValue(*marker)
                                  << " expected-marker="
                                  << (id < rewrittenEnd ? stringFormat("m-{}", id) : "NULL")
                                  << std::endl;
                        auto point = conn->query(
                            "MATCH (n:Chunk {_uuid: '" + key + "'}) RETURN n._text, n.marker;");
                        ASSERT_TRUE(point->isSuccess())
                            << "[check: diagnostic-point] " << point->getErrorMessage();
                        if (point->hasNext()) {
                            auto value = point->getNext();
                            std::cerr << "[string-diagnostic] phase=" << phase
                                      << " mode=point key=" << key
                                      << " actual-text=" << diagnosticValue(*value->getValue(0))
                                      << " actual-marker=" << diagnosticValue(*value->getValue(1))
                                      << std::endl;
                        } else {
                            ADD_FAILURE() << "[check: diagnostic-point-missing] " << key;
                        }
                    }
                }
            }
            std::cerr << "[string-diagnostic] phase=" << phase << " mode=" << mode
                      << " seen=" << seen << " wrong=" << wrong << std::endl;
            EXPECT_EQ(seen, NUM_ROWS) << "[check: diagnostic-cardinality] " << phase << " " << mode;
            EXPECT_EQ(wrong, 0) << "[check: diagnostic-values] " << phase << " " << mode;
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
    std::string alterError;
    {
        // La connexion de l'écrivain et ses résultats meurent avant la réouverture de la base.
        rag3db::main::Connection writer(database.get());
        ASSERT_TRUE(writer.query("BEGIN TRANSACTION;")->isSuccess());
        rewrite(writer, 0, NUM_ROWS);
        // Des lignes neuves aussi, nées sous l'ancien schéma et validées après l'ADD.
        auto created = writer.query(stringFormat(
            "UNWIND range({}, {}) AS i CREATE (:Chunk {_uuid: 'n-' + CAST(i AS STRING), "
            "_text: 'neuve-' + CAST(i AS STRING), marker: 'neuf'});",
            0, NEW_ROWS - 1));
        ASSERT_TRUE(created->isSuccess()) << created->getErrorMessage();
        std::atomic<bool> alterDone{false};
        std::thread alter([&] {
            rag3db::main::Connection other(database.get());
            auto result = other.query("ALTER TABLE Chunk ADD note STRING DEFAULT 'n-defaut';");
            if (!result->isSuccess()) {
                alterError = result->getErrorMessage();
            }
            alterDone = true;
        });
        std::this_thread::sleep_for(std::chrono::milliseconds(200));
        // Ce que voit l'écrivain : l'ADD a-t-il abouti avant son COMMIT, et la colonne lui est-elle
        // visible dans sa transaction ? Imprimé pour le ticket de l'isolement du catalogue.
        const bool doneBeforeCommit = alterDone.load();
        // table_info ne peut pas échouer : une erreur dans la transaction la ferait annuler.
        auto seen = writer.query("CALL table_info('Chunk') RETURN count(*);");
        std::cout << "[témoin] ADD terminé avant le COMMIT de l'écrivain : "
                  << (doneBeforeCommit ? "oui" : "non") << " ; colonnes vues dans sa transaction : "
                  << (seen->isSuccess() ? seen->getNext()->getValue(0)->toString() :
                                          seen->getErrorMessage().substr(0, 120))
                  << " (3 avant l'ADD, 4 après)" << std::endl;
        auto commit = writer.query("COMMIT;");
        alter.join();
        ASSERT_TRUE(commit->isSuccess()) << commit->getErrorMessage();
    }
    std::cout << "[témoin] ADD pendant la transaction : "
              << (alterError.empty() ? "accepté" : "refusé : " + alterError) << std::endl;
    expectAllRight("before-checkpoint");
    {
        for (const auto* which : {"n.marker <> 'neuf'", "n.marker = 'neuf'"}) {
            auto result = conn->query(std::string("MATCH (n:Chunk) WHERE ") + which +
                                      " RETURN count(n), count(n.note), "
                                      "sum(CASE WHEN n.note = 'n-defaut' THEN 1 ELSE 0 END);");
            if (result->isSuccess()) {
                std::cout << "[témoin] après le COMMIT, " << which
                          << " : lignes, note non nulles, note = défaut : "
                          << result->getNext()->toString() << std::endl;
            }
        }
    }
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
    const bool diagnostic = std::getenv("RAG3DB_STRING_DIAGNOSTIC") != nullptr;
    auto state = std::make_shared<StringDiagnosticState>();
    if (diagnostic) {
        const FlakyCheckpointer probe([state](rag3db::main::ClientContext& context) {
            return std::make_unique<StringDiagnosticCheckpointer>(context, state);
        });
        probe.setCheckpointer(*conn->getClientContext());
        diagnose("after-load", 0);
    }
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
        rewrite(*conn, round * step, (round + 1) * step,
            diagnostic ? std::function<void(int64_t, int64_t, bool)>{[&](int64_t first,
                                                                         int64_t last, bool after) {
                state->setStep(stringFormat("rewrite-{}-{}", first, last));
                diagnose(state->getStep() + (after ? "-after" : "-before"), after ? last : first);
            }} :
                         std::function<void(int64_t, int64_t, bool)>{});
        const auto column = "c" + std::to_string(round);
        if (diagnostic) {
            state->setStep("add-" + column);
            diagnose(state->getStep() + "-before", (round + 1) * step);
        }
        ok("ALTER TABLE Chunk ADD " + column + " STRING DEFAULT 'd-" + column + "';");
        if (diagnostic) {
            diagnose(state->getStep() + "-after", (round + 1) * step);
        }
        if (round % 2 == 1) {
            if (diagnostic) {
                state->setStep("drop-c" + std::to_string(round - 1));
                diagnose(state->getStep() + "-before", (round + 1) * step);
            }
            ok("ALTER TABLE Chunk DROP c" + std::to_string(round - 1) + ";");
            if (diagnostic) {
                diagnose(state->getStep() + "-after", (round + 1) * step);
            }
        }
    }
    done = true;
    reader.join();
    if (diagnostic) {
        diagnose("after-automatic-checkpoints", NUM_ROWS);
        state->setStep("explicit-checkpoint");
    }
    expectAllRight("after-automatic-checkpoints");
    ok("CHECKPOINT;");
    expectAllRight("after-checkpoint");
    expectDefault("c4", "d-c4");
    if (diagnostic) {
        diagnose("after-explicit-checkpoint", NUM_ROWS);
    }
    conn.reset();
    database.reset();
    open();
    expectAllRight("after-reopen");
    expectDefault("c4", "d-c4");
    if (diagnostic) {
        diagnose("after-reopen", NUM_ROWS);
    }
}

} // namespace
