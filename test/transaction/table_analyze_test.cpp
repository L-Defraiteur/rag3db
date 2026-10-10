#include <unistd.h>

#include <cmath>
#include <filesystem>
#include <fstream>
#include <functional>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

namespace rag3db {
namespace testing {

// Les statistiques d'une table (STATS_INFO : cardinalité, comptes de valeurs distinctes) sont
// des estimations tenues au fil des écritures ; trois écritures les laissent fausses, et
// CALL analyze les rebâtit depuis les lignes vivantes (mesuré avant lui, le 10 octobre 2026 :
// cardinalité 2 000 pour 1 000 lignes après IGNORE_ERRORS et après DELETE, 2 051 et 1 931
// noms distincts pour 1 000, 10 pour 1 000 après SET, tout cela encore après réouverture ; page
// extension/rag3weaver/docs/10-octobre-2026-coeur-cpp-tickets/03-le-recalcul-des-statistiques.md).
class TableAnalyzeTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "needs an on-disk database: the statistics are reopened";
        }
        csvPath = (std::filesystem::temp_directory_path() /
                   ("table_analyze_" + std::to_string(::getpid()) + ".csv"))
                      .string();
        createDBAndConn();
        ok("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
    }

    void TearDown() override {
        std::filesystem::remove(csvPath);
        EmptyDBTest::TearDown();
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    int64_t single(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        if (!result->isSuccess() || !result->hasNext()) {
            return -1;
        }
        return result->getNext()->getValue(0)->getValue<int64_t>();
    }

    // Les lignes id, name pour id dans [first, first + count), name = namer(id).
    void copyRows(int64_t first, int64_t count, const std::function<std::string(int64_t)>& namer,
        const std::string& options = "") {
        {
            std::ofstream csv(csvPath);
            for (auto id = first; id < first + count; id++) {
                csv << id << "," << namer(id) << "\n";
            }
        }
        ok("COPY Doc FROM '" + csvPath + "'" + options + ";");
    }

    // La cardinalité doit être le nombre de lignes, et le compte de noms distincts doit tomber
    // dans la marge de l'HyperLogLog (64 registres, erreur type ≈ 13 % ; 40 % ≈ trois erreurs
    // types) autour du vrai compte.
    void expectStatisticsTrue(const std::string& when) {
        SCOPED_TRACE(when);
        const auto rows = single("MATCH (d:Doc) RETURN count(*);");
        const auto names = single("MATCH (d:Doc) RETURN count(DISTINCT d.name);");
        const auto cardinality = single("CALL STATS_INFO('Doc') RETURN cardinality;");
        const auto estimatedNames = single("CALL STATS_INFO('Doc') RETURN name_distinct_count;");
        EXPECT_EQ(cardinality, rows) << "cardinalité " << cardinality << " pour " << rows
                                     << " lignes";
        EXPECT_LT(std::abs(static_cast<double>(estimatedNames - names)), 0.4 * names)
            << "noms distincts estimés " << estimatedNames << " pour " << names;
    }

    // Le recalcul, puis la même vérité après une réouverture.
    void analyzeAndReopen() {
        ok("CALL analyze('Doc');");
        expectStatisticsTrue("après CALL analyze");
        conn.reset();
        database.reset();
        createDBAndConn();
        expectStatisticsTrue("après la réouverture");
    }

    std::string csvPath;
};

// Mille clés en double portant chacune un nom neuf, écartées par IGNORE_ERRORS : elles ont été
// comptées avant le contrôle de la clé.
TEST_F(TableAnalyzeTest, AfterACopyThatIgnoresDuplicateKeys) {
    copyRows(0, 1000, [](int64_t id) { return "name " + std::to_string(id); });
    copyRows(0, 1000, [](int64_t id) { return "other " + std::to_string(id); },
        " (IGNORE_ERRORS=true)");
    analyzeAndReopen();
}

// La moitié des lignes supprimée : ni la cardinalité ni les distincts ne reculent.
TEST_F(TableAnalyzeTest, AfterDeletingHalfTheRows) {
    copyRows(0, 2000, [](int64_t id) { return "name " + std::to_string(id); });
    ok("MATCH (d:Doc) WHERE d.id >= 1000 DELETE d;");
    analyzeAndReopen();
}

// Dix noms deviennent mille : les valeurs posées par SET ne sont pas comptées.
TEST_F(TableAnalyzeTest, AfterSettingNewValues) {
    copyRows(0, 1000, [](int64_t id) { return "name " + std::to_string(id % 10); });
    ok("MATCH (d:Doc) SET d.name = 'renamed ' + CAST(d.id AS STRING);");
    analyzeAndReopen();
}

// Le cas limite, dit plutôt que caché : analyze remplace les statistiques à l'exécution, pas à la
// validation (comme reltuples dans PostgreSQL, qui survit au ROLLBACK de son ANALYZE). Dans une
// transaction annulée, il a vu les lignes validées moins celles que la transaction supprimait, et
// pas celles qu'elle insérait (locales) : une estimation périmée, pas une corruption — aucun
// compte faux au-delà de l'estimation, et un analyze de plus la remet d'équerre.
// Ticket 2026-10-10-analyze-non-transactionnel.
TEST_F(TableAnalyzeTest, AnAnalyzeInARolledBackTransactionLeavesAStaleEstimate) {
    copyRows(0, 2000, [](int64_t id) { return "name " + std::to_string(id); });
    ok("BEGIN TRANSACTION;");
    ok("MATCH (d:Doc) WHERE d.id >= 1000 DELETE d;");
    ok("UNWIND range(5000, 5499) AS i CREATE (:Doc {id: i, name: 'new ' + CAST(i AS STRING)});");
    ok("CALL analyze('Doc');");
    ok("ROLLBACK;");

    EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 2000);
    const auto cardinality = single("CALL STATS_INFO('Doc') RETURN cardinality;");
    const auto estimatedNames = single("CALL STATS_INFO('Doc') RETURN name_distinct_count;");
    EXPECT_EQ(cardinality, 1000) << "ce que l'analyze annulé a vu";
    EXPECT_GT(estimatedNames, 0);
    EXPECT_LT(estimatedNames, 1.4 * cardinality) << "des distincts sans plus de lignes";

    ok("CALL analyze('Doc');");
    expectStatisticsTrue("après un analyze hors transaction");
}

// Sans analyze, le point de reprise recale la cardinalité sur les lignes vivantes, d'après les
// informations de version des groupes ; les distincts, eux, attendent analyze.
TEST_F(TableAnalyzeTest, TheCheckpointRecalibratesTheCardinalityOnLiveRows) {
    copyRows(0, 2000, [](int64_t id) { return "name " + std::to_string(id); });
    ok("MATCH (d:Doc) WHERE d.id >= 1000 DELETE d;");
    ok("CHECKPOINT;");
    EXPECT_EQ(single("CALL STATS_INFO('Doc') RETURN cardinality;"), 1000);
    conn.reset();
    database.reset();
    createDBAndConn();
    EXPECT_EQ(single("CALL STATS_INFO('Doc') RETURN cardinality;"), 1000);
}

} // namespace testing
} // namespace rag3db
