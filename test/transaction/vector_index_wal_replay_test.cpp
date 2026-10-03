// Un index vectoriel dont la création ou le retrait ne vit que dans le journal. rag3weaver
// retire ses index avant un chargement en masse et les recrée après ; un arrêt brutal
// entre les deux laisse un DROP_VECTOR_INDEX que seul le rejeu du journal connaît. Le rejeu
// retirait l'index du catalogue et le laissait dans la table, jamais chargé : la base
// rouverte ne savait plus le recréer (« Index … is not loaded yet »). Cas minimal de la
// session de l'arbre principal, 3 octobre 2026. Les tests se sautent si l'extension vector
// n'est pas bâtie.

#include <filesystem>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "storage/storage_utils.h"
#include "test_helper/test_helper.h"

using namespace rag3db::testing;

namespace {

class VectorIndexWalReplayTest : public EmptyDBTest {
protected:
    std::string extension;

    void SetUp() override {
        EmptyDBTest::SetUp();
        extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        if (!std::filesystem::exists(extension)) {
            GTEST_SKIP() << "the vector extension is not built (-DBUILD_EXTENSIONS=vector)";
        }
        if (inMemMode) {
            GTEST_SKIP() << "the journal is replayed only for an on-disk database";
        }
        createDBAndConn();
        ok("LOAD EXTENSION '" + extension + "';");
        ok("CREATE NODE TABLE T(id INT64 PRIMARY KEY, v FLOAT[8]);");
        ok("UNWIND range(0, 299) AS i CREATE (:T {id: i, v: [CAST(i AS FLOAT), 1.0, 2.0, 3.0, "
           "4.0, 5.0, 6.0, CAST(i % 7 AS FLOAT)]});");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    int64_t count(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        return result->isSuccess() ? static_cast<int64_t>(result->getNumTuples()) : -1;
    }

    void createIndex() {
        ok("CALL CREATE_VECTOR_INDEX('T', 'T_vec', 'v', metric := 'cosine', skip_if_exists := "
           "true);");
    }

    // Ce qui suit ne vit que dans le journal : ni point de reprise automatique, ni point de
    // reprise à la fermeture.
    void onlyInTheJournalFromHere() {
        ok("CALL auto_checkpoint=false;");
        ok("CALL force_checkpoint_on_close=false;");
    }

    // Ferme la base puis la rouvre. Avec journalExpected, exige qu'un journal non vide
    // attende la réouverture : sans lui le test ne rejouerait rien et serait vert pour rien.
    bool journalWasThere = false;
    void reopen(bool journalExpected) {
        conn.reset();
        database.reset();
        const auto walPath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
        journalWasThere =
            std::filesystem::exists(walPath) && std::filesystem::file_size(walPath) > 0;
        if (journalExpected) {
            ASSERT_TRUE(journalWasThere) << walPath;
        }
        createDBAndConn();
        ok("LOAD EXTENSION '" + extension + "';");
    }

    int64_t search() {
        return count("CALL QUERY_VECTOR_INDEX('T', 'T_vec', [1.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, "
                     "1.0], 5) RETURN node.id;");
    }
};

TEST_F(VectorIndexWalReplayTest, ADropReplayedFromTheJournalLeavesNothingInTheTable) {
    createIndex();
    ok("CHECKPOINT;");
    onlyInTheJournalFromHere();
    ok("CALL DROP_VECTOR_INDEX('T', 'T_vec');");
    ASSERT_NO_FATAL_FAILURE(reopen(true));
    EXPECT_EQ(count("CALL SHOW_INDEXES() RETURN *;"), 0);
    // L'index n'existe plus : on doit pouvoir le recréer, et s'en servir.
    createIndex();
    EXPECT_EQ(count("CALL SHOW_INDEXES() RETURN *;"), 1);
    EXPECT_EQ(search(), 5);
    // Et l'état recréé tient à une réouverture de plus.
    reopen(false);
    EXPECT_EQ(search(), 5);
}

// Le retrait rejoué doit aussi survivre au point de reprise qui suit : après lui le
// journal est vide, et seule la table dit encore quels index elle porte.
TEST_F(VectorIndexWalReplayTest, AReplayedDropSurvivesTheNextCheckpoint) {
    createIndex();
    ok("CHECKPOINT;");
    onlyInTheJournalFromHere();
    ok("CALL DROP_VECTOR_INDEX('T', 'T_vec');");
    ASSERT_NO_FATAL_FAILURE(reopen(true)); // rejoue le retrait ; la fermeture qui suit écrit un point de reprise
    ASSERT_NO_FATAL_FAILURE(reopen(false));
    EXPECT_EQ(count("CALL SHOW_INDEXES() RETURN *;"), 0);
    createIndex();
    EXPECT_EQ(search(), 5);
}

// Le symétrique n'existe pas : la création d'un index écrit elle-même un point de reprise,
// même quand le point de reprise automatique est coupé. Il ne reste donc jamais de journal
// qui porterait une création seule — et un retrait suivi d'une création n'en laisse pas non
// plus. Les deux cas le disent, pour que le jour où cela change, le rejeu de la création
// reçoive son test.
TEST_F(VectorIndexWalReplayTest, ACreateLeavesNoJournalBehind) {
    ok("CHECKPOINT;");
    onlyInTheJournalFromHere();
    createIndex();
    ASSERT_NO_FATAL_FAILURE(reopen(false));
    EXPECT_FALSE(journalWasThere);
    EXPECT_EQ(count("CALL SHOW_INDEXES() RETURN *;"), 1);
    EXPECT_EQ(search(), 5);
}

TEST_F(VectorIndexWalReplayTest, ADropThenCreateLeavesNoJournalBehind) {
    createIndex();
    ok("CHECKPOINT;");
    onlyInTheJournalFromHere();
    ok("CALL DROP_VECTOR_INDEX('T', 'T_vec');");
    createIndex();
    ASSERT_NO_FATAL_FAILURE(reopen(false));
    EXPECT_FALSE(journalWasThere);
    EXPECT_EQ(count("CALL SHOW_INDEXES() RETURN *;"), 1);
    EXPECT_EQ(search(), 5);
}

} // namespace
