// Le balayage filtré d'une colonne de chaînes ou de listes, à cheval sur deux segments d'un groupe
// (la cause du texte perdu du produit, trouvée par le cœur C++ dans StringColumn::scanFiltered,
// string_column.cpp:240, et la même dans ListColumn::scanFiltered). Dans le second segment d'une
// fenêtre de 2 048, une position n'était écrite que si elle tombait sous le nombre de valeurs de ce
// segment : le reste de la fenêtre restait vide, ou gardait les chaînes de la fenêtre d'avant.
//
// Ce qui fait les segments : leur TAILLE, pas le nombre de COPY. Au point de reprise,
// ColumnChunkData::split découpe chaque colonne en segments d'au plus min(taille/2, 256 Kio). Il faut
// donc assez d'octets : 6 000 lignes de chaînes d'environ 200 octets donnent 5 segments (recette du
// cœur C++, prouvée rouge sans son correctif et verte avec). Le témoin vérifie d'abord cette forme
// par storage_info (au moins deux segments pour `s`) : sans elle, il ne prouverait rien.
//
// Ce qui fait la sélection filtrée : les versions du groupe dans la FENÊTRE lue, pas un WHERE. Une
// ligne supprimée (id 10 pour la fenêtre 0, id 2 100 pour la fenêtre 1). Puis le balayage, comparé
// à l'attendu et à la relecture par clé. Une variante meurt (SIGKILL) après le COPY, avant tout point
// de reprise, et relit après le rejeu.
//
// Prouvé le 12 octobre 2026 sur le poste principal : sans le correctif (master 79431d9c4),
// DeletedRowInWindow0 rend 801 lignes fausses, DeletedRowInWindow1 886, AfterADeath 800 (la chaîne
// d'une autre ligne, 204 octets ; la relecture par clé juste) ; avec le correctif (f9b3d53e0), 0
// partout. OwnWriteInWindow0, une écriture dans la transaction qui lit, ne rougit pas sans le
// correctif : ce chemin ne passe pas par la borne fausse ; il reste comme garde.

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <fstream>
#include <set>
#include <sstream>
#include <string>

#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr int64_t NUM_ROWS = 6000;

enum class Kind { DELETED_ROW_IN_WINDOW_0, DELETED_ROW_IN_WINDOW_1, OWN_WRITE_IN_WINDOW_0, AFTER_A_DEATH };

std::string textOf(int64_t id) {
    return std::string(200, 'x') + std::to_string(id);
}

std::string listOf(int64_t id) {
    std::string out = "[";
    for (int64_t i = 0; i < id % 4; ++i) {
        out += (i == 0 ? "" : ",") + std::to_string(id + i);
    }
    return out + "]";
}

class StringScanFiltered : public EmptyDBTest, public ::testing::WithParamInterface<Kind> {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed (the segments come from a checkpoint)";
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

    std::string csv() const {
        const auto path = databasePath + ".rows.csv";
        std::ofstream out(path);
        for (int64_t id = 0; id < NUM_ROWS; ++id) {
            out << id << "," << textOf(id) << ",\"" << listOf(id) << "\",\n";
        }
        return path;
    }

    // Le COPY et un point de reprise ; ou, pour la variante qui meurt, le COPY dans un processus
    // fils tué avant tout point de reprise, puis le rejeu et le point de reprise ici.
    void load() {
        const auto path = csv();
        if (GetParam() != Kind::AFTER_A_DEATH) {
            ok("COPY T FROM '" + path + "' (header=false);");
            ok("CHECKPOINT;");
            return;
        }
        conn.reset();
        database.reset();
        const auto pid = fork();
        if (pid == 0) {
            try {
                rag3db::main::Database child(databasePath, *systemConfig);
                rag3db::main::Connection connection(&child);
                connection.query("CALL auto_checkpoint=false;");
                connection.query("CALL force_checkpoint_on_close=false;");
                if (!connection.query("COPY T FROM '" + path + "' (header=false);")->isSuccess()) {
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

    int64_t segmentsOfS() {
        auto result = conn->query("CALL storage_info('T') WHERE column_name = 's' AND "
                                  "node_group_id = 0 RETURN count(*);");
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    // Le balayage contre l'attendu : rend le nombre de lignes fausses et un résumé borné.
    int64_t compareScan(const std::set<int64_t>& absent, std::string& summary) {
        auto result = conn->query("MATCH (n:T) RETURN n.id, n.s, n.l;");
        if (!result->isSuccess()) {
            summary = result->getErrorMessage();
            return -1;
        }
        int64_t wrong = 0;
        int64_t seen = 0;
        int64_t firstWrong = -1;
        std::ostringstream out;
        while (result->hasNext()) {
            auto tuple = result->getNext();
            const auto id = tuple->getValue(0)->getValue<int64_t>();
            seen++;
            const auto* s = tuple->getValue(1);
            const auto sText = s->isNull() ? std::string("<null>") : s->getValue<std::string>();
            const auto lText = tuple->getValue(2)->toString();
            if (sText != textOf(id) || lText != listOf(id)) {
                if (firstWrong < 0) {
                    firstWrong = id;
                }
                if (wrong++ < 5) {
                    out << " [id " << id << " : s de " << sText.size() << " octets"
                        << (sText == textOf(id) ? " (juste)" : "") << ", l " << lText
                        << (lText == listOf(id) ? " (juste)" : "") << "]";
                }
            }
        }
        if (firstWrong >= 0) {
            out << " ; première fausse : id " << firstWrong;
        }
        const auto expected = NUM_ROWS - static_cast<int64_t>(absent.size());
        if (seen != expected) {
            out << " (" << seen << " lignes lues pour " << expected << ")";
            wrong += std::abs(expected - seen);
        }
        summary = out.str();
        return wrong;
    }

    // La relecture par clé, sur toute la table : l'autre chemin, la référence.
    int64_t compareByKey(const std::set<int64_t>& absent) {
        int64_t wrong = 0;
        for (int64_t id = 0; id < NUM_ROWS; ++id) {
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
    load();
    const auto segments = segmentsOfS();
    std::cerr << "  [balayage] segments de s dans le groupe 0 : " << segments << "\n";
    ASSERT_GE(segments, 2) << "[check: plan] the column s is not split into segments: the witness "
                              "would prove nothing";
    std::set<int64_t> absent;
    switch (GetParam()) {
    case Kind::DELETED_ROW_IN_WINDOW_0:
    case Kind::AFTER_A_DEATH:
        ok("MATCH (n:T {id: 10}) DELETE n;");
        absent.insert(10);
        break;
    case Kind::DELETED_ROW_IN_WINDOW_1:
        ok("MATCH (n:T {id: 2100}) DELETE n;");
        absent.insert(2100);
        break;
    case Kind::OWN_WRITE_IN_WINDOW_0:
        ok("BEGIN TRANSACTION;");
        ok("MATCH (n:T {id: 10}) SET n.flag = 'written';");
        break;
    }
    std::string summary;
    const auto scanned = compareScan(absent, summary);
    std::cerr << "  [balayage] " << scanned << " lignes fausses" << summary << "\n";
    EXPECT_EQ(scanned, 0) << "[check: scan-equals-expected] " << summary;
    const auto byKey = compareByKey(absent);
    std::cerr << "  [balayage] relecture par clé : " << byKey << " lignes fausses\n";
    EXPECT_EQ(byKey, 0) << "[check: key-lookup-equals-expected] ";
    if (GetParam() == Kind::OWN_WRITE_IN_WINDOW_0) {
        ok("COMMIT;");
    }
}

INSTANTIATE_TEST_SUITE_P(Windows, StringScanFiltered,
    ::testing::Values(Kind::DELETED_ROW_IN_WINDOW_0, Kind::DELETED_ROW_IN_WINDOW_1,
        Kind::OWN_WRITE_IN_WINDOW_0, Kind::AFTER_A_DEATH),
    [](const ::testing::TestParamInfo<Kind>& info) {
        switch (info.param) {
        case Kind::DELETED_ROW_IN_WINDOW_0:
            return std::string("DeletedRowInWindow0");
        case Kind::DELETED_ROW_IN_WINDOW_1:
            return std::string("DeletedRowInWindow1");
        case Kind::OWN_WRITE_IN_WINDOW_0:
            return std::string("OwnWriteInWindow0");
        case Kind::AFTER_A_DEATH:
            return std::string("AfterADeath");
        }
        return std::string("?");
    });


// Une garde pour ListColumn::scanFiltered, pas un témoin de défaut. Sa borne
// (`startOffsetInSegment + pos - offsetInResult < numValues`) était déjà juste pour un segment ; le
// cœur C++ l'a bornée aussi par numValuesToScan, par forme, sans cas atteignable (12 octobre 2026).
// Ce cas est vert sans ce changement comme avec : des listes assez grosses pour que leur colonne se
// découpe (64 éléments sur 6 000 lignes, 4 segments), une ligne supprimée dans la fenêtre lue, et le
// balayage des listes contre l'attendu. Le contrôle `plan` exige au moins deux segments pour `l`.
std::string bigListOf(int64_t id) {
    std::string out = "[";
    for (int64_t i = 0; i < 64; ++i) {
        out += (i == 0 ? "" : ",") + std::to_string(id * 64 + i);
    }
    return out + "]";
}

class ListScanFiltered : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed (the segments come from a checkpoint)";
        }
        createDBAndConn();
        ok("CALL auto_checkpoint=false;");
        ok("CREATE NODE TABLE L(id INT64 PRIMARY KEY, l INT64[]);");
        const auto path = databasePath + ".lists.csv";
        {
            std::ofstream out(path);
            for (int64_t id = 0; id < NUM_ROWS; ++id) {
                out << id << ",\"" << bigListOf(id) << "\"\n";
            }
        }
        ok("COPY L FROM '" + path + "' (header=false);");
        ok("CHECKPOINT;");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess())
            << "[check: query] " << query.substr(0, 160) << " : " << result->getErrorMessage();
    }
};

TEST_F(ListScanFiltered, ScanOfBigListsEqualsExpected) {
    auto segments = conn->query("CALL storage_info('L') WHERE column_name = 'l' AND node_group_id "
                                "= 0 RETURN count(*);");
    const auto numSegments = segments->isSuccess() && segments->hasNext() ?
                                 segments->getNext()->getValue(0)->getValue<int64_t>() :
                                 -1;
    std::cerr << "  [listes] segments de l dans le groupe 0 : " << numSegments << "\n";
    ASSERT_GE(numSegments, 2) << "[check: plan] the column l is not split into segments: the "
                                 "witness would prove nothing";
    ok("MATCH (n:L {id: 10}) DELETE n;");
    auto result = conn->query("MATCH (n:L) RETURN n.id, n.l;");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    int64_t wrong = 0;
    int64_t seen = 0;
    int64_t firstWrong = -1;
    while (result->hasNext()) {
        auto tuple = result->getNext();
        const auto id = tuple->getValue(0)->getValue<int64_t>();
        seen++;
        if (tuple->getValue(1)->toString() != bigListOf(id)) {
            if (firstWrong < 0) {
                firstWrong = id;
            }
            wrong++;
        }
    }
    std::cerr << "  [listes] " << wrong << " listes fausses sur " << seen << " (première : id "
              << firstWrong << ")\n";
    EXPECT_EQ(seen, NUM_ROWS - 1) << "[check: list-scan-equals-expected] rows";
    EXPECT_EQ(wrong, 0) << "[check: list-scan-equals-expected] first wrong id " << firstWrong;
}

} // namespace
