// Une chaîne refusée par le dictionnaire ne laisse pas de ligne comptée sans indice (ticket
// docs/coeur-cpp/tickets/2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-
// colonne-de-chaines.md). StringChunkData comptait la ligne (updateNumValues) avant d'ajouter sa
// chaîne au dictionnaire (DictionaryChunk::appendString). Sous un pool plein, l'ajout lève sur son
// agrandissement : la ligne restait comptée, non nulle, avec un indice jamais écrit. Le point de
// reprise suivant la relisait dans StringChunkData::finalize par cet indice, et mourait dans le
// hachage d'une chaîne sauvage. Le point de faute string-dictionary-append refuse l'ajout comme le
// pool plein.
#include <string>

#include "common/test_hooks.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::main;

namespace rag3db {
namespace testing {

class DictionaryAppendRefused : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        run("CALL auto_checkpoint=false;");
        run("CREATE NODE TABLE T(id INT64, s STRING, PRIMARY KEY(id));");
        run("COPY T FROM (UNWIND range(0, 999) AS i RETURN i, 's' + CAST(i AS STRING));");
        run("CHECKPOINT;");
        // Des lignes en mémoire, dans le groupe où le COMMIT versera les siennes.
        run("UNWIND range(1000, 1049) AS i CREATE (:T {id: i, s: 'm' + CAST(i AS STRING)});");
    }
    void TearDown() override {
        TestHooks::armFailPoint(TestFailPoint::NONE);
        EmptyDBTest::TearDown();
    }
    void run(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    std::string one(const std::string& query) {
        auto result = conn->query(query);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        return result->hasNext() ? result->getNext()->toString() : "AUCUNE";
    }
    // Les lignes d'avant le refus se relisent, avant et après un point de reprise, puis une
    // réouverture.
    void expectTheRowsReadBack(const std::string& moment) {
        EXPECT_EQ(one("MATCH (t:T) WHERE t.s = 's' + CAST(t.id AS STRING) OR t.s = 'm' + "
                      "CAST(t.id AS STRING) RETURN count(*);"),
            "1050\n")
            << "[check: a-refused-string-leaves-no-row-without-an-index] " << moment;
        run("CHECKPOINT;");
        EXPECT_EQ(one("MATCH (t:T) WHERE t.s = 's' + CAST(t.id AS STRING) OR t.s = 'm' + "
                      "CAST(t.id AS STRING) RETURN count(*);"),
            "1050\n")
            << "[check: a-checkpoint-after-a-refused-string-keeps-the-rows] " << moment;
        createDBAndConn();
        EXPECT_EQ(one("MATCH (t:T) RETURN count(*);"), "1050\n")
            << "[check: a-checkpoint-after-a-refused-string-keeps-the-rows] " << moment
            << ", reopened";
    }
};

TEST_F(DictionaryAppendRefused, ACommitRefusedInItsStringsLeavesTheGroupReadable) {
    run("BEGIN TRANSACTION;");
    run("UNWIND range(2000, 2099) AS i CREATE (:T {id: i, s: 'neuve ' + CAST(i AS STRING)});");
    TestHooks::armFailPoint(TestFailPoint::STRING_DICTIONARY_APPEND);
    auto commit = conn->query("COMMIT;");
    TestHooks::armFailPoint(TestFailPoint::NONE);
    if (!commit->isSuccess()) {
        conn->query("ROLLBACK;");
    }
    EXPECT_EQ(one("MATCH (t:T) WHERE t.id >= 2000 RETURN count(*);"),
        commit->isSuccess() ? "100\n" : "0\n")
        << commit->getErrorMessage();
    expectTheRowsReadBack("after a refused commit");
}

TEST_F(DictionaryAppendRefused, ACopyRefusedInItsStringsLeavesTheTableReadable) {
    TestHooks::armFailPoint(TestFailPoint::STRING_DICTIONARY_APPEND);
    auto copy = conn->query("COPY T FROM (UNWIND range(3000, 3999) AS i RETURN i, 'copie ' + "
                            "CAST(i AS STRING));");
    TestHooks::armFailPoint(TestFailPoint::NONE);
    EXPECT_FALSE(copy->isSuccess()) << "[check: setup] the COPY was not refused";
    EXPECT_EQ(one("MATCH (t:T) WHERE t.id >= 3000 RETURN count(*);"), "0\n");
    expectTheRowsReadBack("after a refused copy");
}

} // namespace testing
} // namespace rag3db
