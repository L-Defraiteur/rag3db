// Le balayage filtré d'une colonne de chaînes ou de listes, à cheval sur deux segments d'un groupe
// (la cause du texte perdu du produit, trouvée par le cœur C++ dans StringColumn::scanFiltered,
// string_column.cpp:240, et la même dans ListColumn::scanFiltered). Dans le second segment d'une
// fenêtre de 2 048, la position `pos` n'était écrite que si `pos < numValues(second segment)` : avec
// un second segment COURT, ses lignes tombent au-delà et ne sont pas écrites. On relit alors les
// chaînes de la fenêtre d'avant, ou des vides. Le chemin non filtré est juste ; il faut une
// sélection filtrée (une ligne supprimée, ou une écriture dans la transaction qui lit).
//
// Un groupe écrit en deux segments : un COPY de 3 000 lignes, CHECKPOINT, puis un COPY de 500 lignes
// (le second segment court, rouge attendu) ou de 3 000 (long, vert des deux côtés), CHECKPOINT. Puis
// le balayage, avec et sans WHERE (comme le produit, `_embed_claim IS NULL`), comparé à l'attendu et
// à la relecture par clé ligne à ligne. Une variante meurt (SIGKILL) après le second COPY, avant tout
// point de reprise, et relit après le rejeu.

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <fstream>
#include <map>
#include <set>
#include <sstream>
#include <string>
#include <vector>

#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr int64_t FIRST_SEGMENT = 3000;

enum class Filter { DELETED_ROW, OWN_WRITE };

struct Case {
    int64_t secondSegment; // 500 : court ; 3 000 : long
    Filter filter;
    bool death;
};

std::string textOf(int64_t id) {
    if (id % 5 == 0) {
        return "repeated " + std::to_string(id % 3);
    }
    std::string out;
    const auto length = id % 3 == 0 ? 300 + id % 200 : 5 + id % 30;
    for (int64_t i = 0; i < length; ++i) {
        out += static_cast<char>('a' + (id * 31 + i * 7) % 26);
    }
    return out;
}

std::string listOf(int64_t id) {
    std::string out = "[";
    for (int64_t i = 0; i < id % 4; ++i) {
        out += (i == 0 ? "" : ",") + std::to_string(id + i);
    }
    return out + "]";
}

class StringScanFiltered : public EmptyDBTest, public ::testing::WithParamInterface<Case> {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed (two segments need a checkpoint)";
        }
        open();
        ok("CREATE NODE TABLE T(id INT64 PRIMARY KEY, s STRING, l INT64[], flag STRING);");
    }

    void open() {
        createDBAndConn();
        ok("CALL auto_checkpoint=false;");
        ok("CALL force_checkpoint_on_close=false;");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess())
            << "[check: query] " << query.substr(0, 160) << " : " << result->getErrorMessage();
    }

    std::string csvOf(int64_t first, int64_t count) const {
        const auto path = databasePath + stringFormat(".rows-{}.csv", first);
        std::ofstream out(path);
        for (auto id = first; id < first + count; ++id) {
            out << id << "," << textOf(id) << ",\"" << listOf(id) << "\",\n";
        }
        return path;
    }

    void writeTwoSegments() {
        ok("COPY T FROM '" + csvOf(0, FIRST_SEGMENT) + "' (header=false);");
        ok("CHECKPOINT;");
        ok("COPY T FROM '" + csvOf(FIRST_SEGMENT, GetParam().secondSegment) + "' (header=false);");
        ok("CHECKPOINT;");
    }

    // La variante qui meurt : le second COPY écrit, la mort avant tout point de reprise, puis le
    // rejeu et un point de reprise (deux segments sur disque).
    void writeTwoSegmentsWithADeath() {
        ok("COPY T FROM '" + csvOf(0, FIRST_SEGMENT) + "' (header=false);");
        ok("CHECKPOINT;");
        const auto second = csvOf(FIRST_SEGMENT, GetParam().secondSegment);
        conn.reset();
        database.reset();
        const auto pid = fork();
        if (pid == 0) {
            try {
                rag3db::main::Database child(databasePath, *systemConfig);
                rag3db::main::Connection connection(&child);
                connection.query("CALL auto_checkpoint=false;");
                connection.query("CALL force_checkpoint_on_close=false;");
                if (!connection.query("COPY T FROM '" + second + "' (header=false);")->isSuccess()) {
                    _exit(3);
                }
                kill(getpid(), SIGKILL);
            } catch (...) {
                _exit(4);
            }
            _exit(5);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
            << "[check: child-died] status " << status;
        open();
        ok("CHECKPOINT;");
    }

    // Compare un balayage à l'attendu ; rend le nombre de lignes fausses et un résumé borné.
    int64_t compareScan(const std::string& query, const std::set<int64_t>& absent,
        std::string& summary) {
        auto result = conn->query(query);
        if (!result->isSuccess()) {
            summary = result->getErrorMessage();
            return -1;
        }
        int64_t wrong = 0;
        int64_t seen = 0;
        std::ostringstream out;
        while (result->hasNext()) {
            auto tuple = result->getNext();
            const auto id = tuple->getValue(0)->getValue<int64_t>();
            seen++;
            const auto* s = tuple->getValue(1);
            const auto sText = s->isNull() ? std::string("<null>") : s->getValue<std::string>();
            const auto lText = tuple->getValue(2)->toString();
            if (sText != textOf(id) || lText != listOf(id)) {
                if (wrong++ < 5) {
                    out << " [id " << id << ", position " << id % 2048 << " : s lue de " << sText.size()
                        << " octets" << (sText == textOf(id) ? " (juste)" : "") << ", l " << lText
                        << (lText == listOf(id) ? " (juste)" : "") << "]";
                }
            }
        }
        const auto expected = FIRST_SEGMENT + GetParam().secondSegment - static_cast<int64_t>(absent.size());
        if (seen != expected) {
            out << " (" << seen << " lignes lues pour " << expected << ")";
            wrong += std::abs(expected - seen);
        }
        summary = out.str();
        return wrong;
    }

    // La relecture par clé, ligne à ligne : la référence que le balayage doit égaler.
    int64_t compareByKey(const std::set<int64_t>& absent) {
        int64_t wrong = 0;
        for (int64_t id = 0; id < FIRST_SEGMENT + GetParam().secondSegment; ++id) {
            if (absent.contains(id)) {
                continue;
            }
            auto result = conn->query(stringFormat("MATCH (n:T {id: {}}) RETURN n.s, n.l;", id));
            if (!result->isSuccess() || !result->hasNext()) {
                wrong++;
                continue;
            }
            auto tuple = result->getNext();
            if (tuple->getValue(0)->getValue<std::string>() != textOf(id) ||
                tuple->getValue(1)->toString() != listOf(id)) {
                wrong++;
            }
        }
        return wrong;
    }
};

TEST_P(StringScanFiltered, ScanEqualsTheKeyLookups) {
    if (GetParam().death) {
        writeTwoSegmentsWithADeath();
    } else {
        writeTwoSegments();
    }
    std::set<int64_t> absent;
    if (GetParam().filter == Filter::DELETED_ROW) {
        ok("MATCH (n:T {id: 10}) DELETE n;");
        absent.insert(10);
    } else {
        ok("BEGIN TRANSACTION;");
        ok("MATCH (n:T {id: 10}) SET n.flag = 'written';");
    }
    std::string summary;
    const auto unfiltered = compareScan("MATCH (n:T) RETURN n.id, n.s, n.l;", absent, summary);
    std::cerr << "  [balayage] sans WHERE : " << unfiltered << " lignes fausses" << summary << "\n";
    EXPECT_EQ(unfiltered, 0) << "[check: scan-equals-expected] " << summary;
    auto whereAbsent = absent;
    if (GetParam().filter == Filter::OWN_WRITE) {
        whereAbsent.insert(10); // sa marque n'est plus nulle
    }
    const auto filtered =
        compareScan("MATCH (n:T) WHERE n.flag IS NULL RETURN n.id, n.s, n.l;", whereAbsent, summary);
    std::cerr << "  [balayage] avec WHERE flag IS NULL : " << filtered << " lignes fausses" << summary
              << "\n";
    EXPECT_EQ(filtered, 0) << "[check: filtered-scan-equals-expected] " << summary;
    const auto byKey = compareByKey(absent);
    std::cerr << "  [balayage] relecture par clé : " << byKey << " lignes fausses\n";
    EXPECT_EQ(byKey, 0) << "[check: key-lookup-equals-expected] ";
    if (GetParam().filter == Filter::OWN_WRITE) {
        ok("COMMIT;");
    }
}

INSTANTIATE_TEST_SUITE_P(Segments, StringScanFiltered,
    ::testing::Values(Case{500, Filter::DELETED_ROW, false}, Case{500, Filter::OWN_WRITE, false},
        Case{3000, Filter::DELETED_ROW, false}, Case{3000, Filter::OWN_WRITE, false},
        Case{500, Filter::DELETED_ROW, true}),
    [](const ::testing::TestParamInfo<Case>& info) {
        return std::string(info.param.secondSegment < 2048 ? "ShortSecondSegment" : "LongSecondSegment") +
               (info.param.filter == Filter::DELETED_ROW ? "DeletedRow" : "OwnWrite") +
               (info.param.death ? "AfterADeath" : "");
    });

} // namespace
