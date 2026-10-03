// La mort d'un processus qui écrivait dans une table indexée par HNSW, puis l'ouverture de
// la base par un processus neuf — un service qui redémarre. Un seul écrivain, aucun mode
// multi-écrivains. Trouvé par le banc de concurrence le 3 octobre 2026 : la base ne
// s'ouvrait plus (SIGSEGV dans NodeTable::initInsertState, au rejeu du journal).
//
// Deux précautions, sans lesquelles ces tests seraient verts pour rien :
//   - le processus meurt base ouverte (SIGKILL), sans destructeur ni point de reprise ;
//   - la base se rouvre dans un processus neuf (exec de ce même binaire), parce qu'un
//     processus qui a déjà chargé l'extension ne voit pas le défaut.
// Ils se sautent si l'extension vector n'est pas bâtie.

#include <csignal>
#include <cstdlib>
#include <filesystem>
#include <string>
#include <vector>

#include <sys/wait.h>
#include <unistd.h>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/storage_utils.h"
#include "test_helper/test_helper.h"

using namespace rag3db::testing;

namespace {

constexpr const char* PROBE_PATH = "RAG3DB_CRASH_REOPEN_PROBE_PATH";
constexpr const char* PROBE_EXTENSION = "RAG3DB_CRASH_REOPEN_PROBE_EXTENSION";

std::string vectorExtensionPath() {
    return TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
}

constexpr const char* PROBE_ROWS = "RAG3DB_CRASH_REOPEN_PROBE_ROWS";
constexpr const char* PROBE_VECTOR = "RAG3DB_CRASH_REOPEN_PROBE_VECTOR";
constexpr const char* PROBE_NEAREST = "RAG3DB_CRASH_REOPEN_PROBE_NEAREST";
constexpr const char* PROBE_OPEN_ONLY = "RAG3DB_CRASH_REOPEN_PROBE_OPEN_ONLY";
constexpr const char* PROBE_WRITE_WITHOUT_EXTENSION = "RAG3DB_CRASH_REOPEN_PROBE_WRITE";

// Les codes de sortie du processus neuf.
constexpr int INDEX_INTACT = 0;   // l'index répond, et juste
constexpr int INDEX_REBUILT = 20; // l'index était « en retard » ; retiré, rebâti, il répond juste
constexpr int OPENED = 30;        // mode « ouvrir seulement »
constexpr int WRITE_REFUSED = 40; // mode « écrire sans l'extension » : refus nommé, rien d'écrit

// Le corps du processus neuf : ouvrir la base, charger l'extension, compter, chercher. Si
// l'index se dit en retard sur sa table, vérifier qu'il refuse aussi d'être recréé par-dessus,
// le retirer, le rebâtir, et chercher à nouveau. Sort par un code qui dit où cela s'arrête.
int probe(const std::string& path, const std::string& extension) {
    const auto expectedRows = std::atoll(std::getenv(PROBE_ROWS));
    const std::string vector = std::getenv(PROBE_VECTOR);
    const auto expectedNearest = std::atoll(std::getenv(PROBE_NEAREST));
    rag3db::main::SystemConfig config;
    config.maxDBSize = 1024ull * 1024 * 1024 * 1024;
    std::unique_ptr<rag3db::main::Database> database;
    try {
        database = std::make_unique<rag3db::main::Database>(path, config);
    } catch (const std::exception& e) {
        fprintf(stderr, "[probe] open: %s\n", e.what());
        return 10;
    }
    rag3db::main::Connection connection(database.get());
    auto rows = connection.query("MATCH (d:Doc) RETURN count(*);");
    if (!rows->isSuccess()) {
        fprintf(stderr, "[probe] count: %s\n", rows->getErrorMessage().c_str());
        return 12;
    }
    const auto numRows = rows->getNext()->getValue(0)->getValue<int64_t>();
    if (numRows != expectedRows) {
        fprintf(stderr, "[probe] rows=%ld, expected %ld\n", static_cast<long>(numRows),
            static_cast<long>(expectedRows));
        return 14;
    }
    if (std::getenv(PROBE_WRITE_WITHOUT_EXTENSION) != nullptr) {
        // Hors rejeu, une écriture dans une table dont un index n'est pas chargé est refusée
        // en le disant, pour l'insertion, la suppression et la mise à jour de la colonne.
        // C'est le lieur qui refuse (« … but its extension is not loaded ») ; la table a son
        // propre refus derrière lui (NodeTable::INDEX_NOT_LOADED_FOR_WRITE), pour un chemin
        // qui ne passerait pas par le lieur.
        const std::string refusal = "loaded"; // « not loaded », « unloaded »
        for (const std::string statement :
            {"CREATE (:Doc {id: 700, vec: [700.0, 2.0, 3.0, 4.0]});",
                "MATCH (d:Doc {id: 7}) DELETE d;",
                "MATCH (d:Doc {id: 7}) SET d.vec = [250.5, 2.0, 3.0, 4.0];"}) {
            auto result = connection.query(statement);
            if (result->isSuccess() ||
                result->getErrorMessage().find(refusal) == std::string::npos) {
                fprintf(stderr, "[probe] %s : %s\n", statement.c_str(),
                    result->isSuccess() ? "accepted" : result->getErrorMessage().c_str());
                return 41;
            }
        }
        auto after = connection.query("MATCH (d:Doc) RETURN count(*);");
        if (!after->isSuccess() ||
            after->getNext()->getValue(0)->getValue<int64_t>() != expectedRows) {
            return 42;
        }
        return WRITE_REFUSED;
    }
    if (std::getenv(PROBE_OPEN_ONLY) != nullptr) {
        return OPENED; // sans fermer : le journal reste, la prochaine ouverture le rejoue
    }
    auto loaded = connection.query("LOAD EXTENSION '" + extension + "';");
    if (!loaded->isSuccess()) {
        fprintf(stderr, "[probe] load: %s\n", loaded->getErrorMessage().c_str());
        return 11;
    }
    const auto search = "CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', " + vector +
                        ", 1) RETURN node.id;";
    const std::string create =
        "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2', skip_if_exists := "
        "true);";
    // Un expectedNearest négatif veut dire « tout sauf cette ligne » : elle a été supprimée.
    auto nearestIsRight = [&](rag3db::main::QueryResult& result) {
        const auto nearest =
            result.hasNext() ? result.getNext()->getValue(0)->getValue<int64_t>() : -1;
        fprintf(stderr, "[probe] rows=%ld nearest=%ld\n", static_cast<long>(numRows),
            static_cast<long>(nearest));
        return expectedNearest >= 0 ? nearest == expectedNearest : nearest != -expectedNearest;
    };
    auto found = connection.query(search);
    if (found->isSuccess()) {
        return nearestIsRight(*found) ? INDEX_INTACT : 15;
    }
    const std::string behind = "is behind its table";
    if (found->getErrorMessage().find(behind) == std::string::npos) {
        fprintf(stderr, "[probe] search: %s\n", found->getErrorMessage().c_str());
        return 13;
    }
    fprintf(stderr, "[probe] %s\n", found->getErrorMessage().c_str());
    auto recreated = connection.query(create);
    if (recreated->isSuccess() || recreated->getErrorMessage().find(behind) == std::string::npos) {
        fprintf(stderr, "[probe] create over a late index: %s\n",
            recreated->isSuccess() ? "accepted" : recreated->getErrorMessage().c_str());
        return 16;
    }
    auto dropped = connection.query("CALL DROP_VECTOR_INDEX('Doc', 'doc_index');");
    if (!dropped->isSuccess()) {
        fprintf(stderr, "[probe] drop: %s\n", dropped->getErrorMessage().c_str());
        return 17;
    }
    auto rebuilt = connection.query(create);
    if (!rebuilt->isSuccess()) {
        fprintf(stderr, "[probe] rebuild: %s\n", rebuilt->getErrorMessage().c_str());
        return 18;
    }
    auto foundAgain = connection.query(search);
    if (!foundAgain->isSuccess() || !nearestIsRight(*foundAgain)) {
        fprintf(stderr, "[probe] search after rebuild: %s\n",
            foundAgain->isSuccess() ? "wrong row" : foundAgain->getErrorMessage().c_str());
        return 19;
    }
    return INDEX_REBUILT;
}

// N'est un test que de nom : c'est le point d'entrée du processus neuf.
TEST(VectorIndexCrashReopenProbe, OpenInAFreshProcess) {
    const auto path = std::getenv(PROBE_PATH);
    const auto extension = std::getenv(PROBE_EXTENSION);
    if (path == nullptr || extension == nullptr) {
        GTEST_SKIP() << "entry point of the fresh process, not a test";
    }
    _exit(probe(path, extension));
}

class VectorIndexCrashReopenTest : public EmptyDBTest {
protected:
    std::string extension;

    void SetUp() override {
        EmptyDBTest::SetUp();
        extension = vectorExtensionPath();
        if (!std::filesystem::exists(extension)) {
            GTEST_SKIP() << "the vector extension is not built (-DBUILD_EXTENSIONS=vector)";
        }
        if (inMemMode) {
            GTEST_SKIP() << "needs an on-disk database";
        }
        systemConfig->maxDBSize = 1024ull * 1024 * 1024 * 1024;
    }

    static void must(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        if (!result->isSuccess()) {
            fprintf(stderr, "[session] %s : %s\n", query.c_str(),
                result->getErrorMessage().c_str());
            _exit(3);
        }
    }

    // Une session dans un processus fils : elle charge l'extension, coupe le point de
    // reprise automatique, joue ses instructions, puis se tue base ouverte (SIGKILL) ou se
    // ferme proprement.
    void session(const std::vector<std::string>& statements, bool dies) {
        const auto child = fork();
        ASSERT_GE(child, 0);
        if (child == 0) {
            {
                auto database =
                    std::make_unique<rag3db::main::Database>(databasePath, *systemConfig);
                rag3db::main::Connection connection(database.get());
                must(connection, "LOAD EXTENSION '" + extension + "';");
                must(connection, "CALL auto_checkpoint=false;");
                for (const auto& statement : statements) {
                    must(connection, statement);
                }
                if (dies) {
                    kill(getpid(), SIGKILL);
                }
            }
            _exit(0);
        }
        int status = 0;
        ASSERT_EQ(waitpid(child, &status, 0), child);
        if (dies) {
            ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
                << "the session did not die by SIGKILL (status " << status << ")";
        } else {
            ASSERT_TRUE(WIFEXITED(status) && WEXITSTATUS(status) == 0) << "status " << status;
        }
    }

    static std::string vectorOf(int64_t id) {
        return "[" + std::to_string(id) + ".0, 2.0, 3.0, 4.0]";
    }
    // Le vecteur que reçoit une ligne mise à jour : entre deux lignes, à distance nulle de
    // personne d'autre. Pas un vecteur très loin des autres : une telle ligne est injoignable
    // dans un index bâti d'un coup (défaut de construction connu, journal des chantiers §6),
    // et le cas rougirait pour une raison qui n'est pas la sienne.
    const std::string updatedVector = "[250.5, 2.0, 3.0, 4.0]";
    static std::string insertDoc(int64_t id) {
        return "CREATE (:Doc {id: " + std::to_string(id) + ", vec: " + vectorOf(id) + "});";
    }
    const std::string createTable = "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);";
    const std::string fill = "UNWIND range(0, 499) AS i CREATE (:Doc {id: i, vec: [CAST(i AS "
                             "FLOAT), 2.0, 3.0, 4.0]});";
    const std::string createIndex =
        "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');";

    // Une base dont l'index a été créé par une session fermée proprement.
    void indexedBaseFromAnEarlierSession() {
        ASSERT_NO_FATAL_FAILURE(session({createTable, fill, createIndex}, false));
    }

    bool journalIsThere() const {
        const auto walPath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
        return std::filesystem::exists(walPath) && std::filesystem::file_size(walPath) > 0;
    }

    // Ouvre la base dans un processus neuf. Rend son code de sortie, ou -signal.
    int openInAFreshProcess(int64_t expectedRows, const std::string& vector,
        int64_t expectedNearest, bool openOnly = false, bool writeWithoutExtension = false) const {
        const auto child = fork();
        if (child == 0) {
            setenv(PROBE_PATH, databasePath.c_str(), 1);
            setenv(PROBE_EXTENSION, extension.c_str(), 1);
            setenv(PROBE_ROWS, std::to_string(expectedRows).c_str(), 1);
            setenv(PROBE_VECTOR, vector.c_str(), 1);
            setenv(PROBE_NEAREST, std::to_string(expectedNearest).c_str(), 1);
            if (openOnly) {
                setenv(PROBE_OPEN_ONLY, "1", 1);
            }
            if (writeWithoutExtension) {
                setenv(PROBE_WRITE_WITHOUT_EXTENSION, "1", 1);
            }
            execl("/proc/self/exe", "transaction_test",
                "--gtest_filter=VectorIndexCrashReopenProbe.OpenInAFreshProcess",
                static_cast<char*>(nullptr));
            _exit(127);
        }
        int status = 0;
        waitpid(child, &status, 0);
        return WIFSIGNALED(status) ? -WTERMSIG(status) : WEXITSTATUS(status);
    }
};

// L'index est créé par la session qui meurt : c'est la première indexation d'un dépôt. La
// création de l'index écrit un point de reprise ; le journal ne porte donc plus le chargement
// de l'extension, seulement l'insertion. La base s'ouvre, la ligne est là, et l'index se dit
// en retard jusqu'à ce qu'on le rebâtisse.
TEST_F(VectorIndexCrashReopenTest, IndexCreatedByTheSessionThatDies) {
    session({createTable, fill, createIndex, insertDoc(600)}, true);
    ASSERT_TRUE(journalIsThere());
    EXPECT_EQ(openInAFreshProcess(501, vectorOf(600), 600), INDEX_REBUILT);
}

// L'index vient d'une session d'avant ; la session qui meurt n'a fait que charger l'extension
// et insérer. Son journal porte encore le chargement de l'extension : le rejeu la charge,
// tient l'index à jour, et rien n'est à rebâtir.
TEST_F(VectorIndexCrashReopenTest, TheJournalStillLoadsTheExtension) {
    indexedBaseFromAnEarlierSession();
    session({insertDoc(600)}, true);
    ASSERT_TRUE(journalIsThere());
    EXPECT_EQ(openInAFreshProcess(501, vectorOf(600), 600), INDEX_INTACT);
}

// La même, mais un point de reprise a eu lieu dans la session qui meurt, avant l'écriture :
// le journal ne porte plus le chargement de l'extension. C'est le cas qui dit la condition.
TEST_F(VectorIndexCrashReopenTest, AnInsertAfterACheckpointInTheDyingSession) {
    indexedBaseFromAnEarlierSession();
    session({"CHECKPOINT;", insertDoc(600)}, true);
    ASSERT_TRUE(journalIsThere());
    EXPECT_EQ(openInAFreshProcess(501, vectorOf(600), 600), INDEX_REBUILT);
}

TEST_F(VectorIndexCrashReopenTest, ADeleteAfterACheckpointInTheDyingSession) {
    indexedBaseFromAnEarlierSession();
    session({"CHECKPOINT;", "MATCH (d:Doc {id: 7}) DELETE d;"}, true);
    ASSERT_TRUE(journalIsThere());
    // La ligne 7 n'est plus là : chercher son vecteur rend une autre ligne.
    EXPECT_EQ(openInAFreshProcess(499, vectorOf(7), -7), INDEX_REBUILT);
}

TEST_F(VectorIndexCrashReopenTest, AVectorUpdateAfterACheckpointInTheDyingSession) {
    indexedBaseFromAnEarlierSession();
    session({"CHECKPOINT;", "MATCH (d:Doc {id: 7}) SET d.vec = " + updatedVector + ";"}, true);
    ASSERT_TRUE(journalIsThere());
    EXPECT_EQ(openInAFreshProcess(500, updatedVector, 7), INDEX_REBUILT);
}

// Deux ouvertures par des processus qui meurent sans fermer, puis une seconde session qui
// écrit encore et meurt à son tour. La reprise écrit elle-même un point de reprise à la fin
// du rejeu : dès la première ouverture c'est la base, et non plus le journal, qui dit que
// l'index est à rebâtir. L'index rebâti porte les deux écritures.
TEST_F(VectorIndexCrashReopenTest, TheLateStateSurvivesReopeningsAndASecondDeath) {
    indexedBaseFromAnEarlierSession();
    session({"CHECKPOINT;", insertDoc(600)}, true);
    ASSERT_TRUE(journalIsThere());
    EXPECT_EQ(openInAFreshProcess(501, vectorOf(600), 600, true), OPENED);
    EXPECT_EQ(openInAFreshProcess(501, vectorOf(600), 600, true), OPENED);
    session({insertDoc(601)}, true);
    ASSERT_TRUE(journalIsThere());
    EXPECT_EQ(openInAFreshProcess(502, vectorOf(601), 601), INDEX_REBUILT);
}

// Le rejeu d'une mise à jour ou d'une suppression doit atteindre un index chargé : le journal
// porte encore le chargement de l'extension, et l'index doit suivre la table.
TEST_F(VectorIndexCrashReopenTest, AReplayedVectorUpdateReachesALoadedIndex) {
    indexedBaseFromAnEarlierSession();
    session({"MATCH (d:Doc {id: 7}) SET d.vec = " + updatedVector + ";"}, true);
    ASSERT_TRUE(journalIsThere());
    EXPECT_EQ(openInAFreshProcess(500, updatedVector, 7), INDEX_INTACT);
}

TEST_F(VectorIndexCrashReopenTest, AReplayedDeleteReachesALoadedIndex) {
    indexedBaseFromAnEarlierSession();
    session({"MATCH (d:Doc {id: 7}) DELETE d;"}, true);
    ASSERT_TRUE(journalIsThere());
    EXPECT_EQ(openInAFreshProcess(499, vectorOf(7), -7), INDEX_INTACT);
}

// Et après un point de reprise par-dessus l'état « en retard » (une session qui s'ouvre et se
// ferme proprement) : le journal est vide, seule la base dit encore que l'index est à rebâtir.
TEST_F(VectorIndexCrashReopenTest, TheLateStateSurvivesACheckpoint) {
    indexedBaseFromAnEarlierSession();
    session({"CHECKPOINT;", insertDoc(600)}, true);
    ASSERT_TRUE(journalIsThere());
    session({"CHECKPOINT;"}, false);
    EXPECT_EQ(openInAFreshProcess(501, vectorOf(600), 600), INDEX_REBUILT);
}

// Hors de tout rejeu : un processus qui ouvre la base sans charger l'extension et écrit dans
// la table indexée. Il est refusé en le disant, et rien n'est écrit.
TEST_F(VectorIndexCrashReopenTest, AWriteWithoutTheExtensionIsRefusedByName) {
    indexedBaseFromAnEarlierSession();
    EXPECT_EQ(openInAFreshProcess(500, vectorOf(0), 0, false, true), WRITE_REFUSED);
}

} // namespace
