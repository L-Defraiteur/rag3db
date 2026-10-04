// Un COPY journalisé dans une table qui porte déjà son index vectoriel, puis la mort base
// ouverte : à la réouverture les lignes reviennent par le rejeu du journal, et l'index doit
// les porter — chacune retrouvée par son vecteur exact — et supporter ensuite la mise à jour
// de leurs vecteurs. C'est la forme de la reprise de rag3weaver après un paquet défait (index
// posé avant les lignes, morceaux par COPY, vecteurs reposés par SET à la reprise).
// Se saute si l'extension vector n'est pas bâtie.

#ifndef __SINGLE_THREADED__

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <filesystem>
#include <fstream>
#include <functional>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/storage_utils.h"
#include "test_helper/test_helper.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class VectorIndexJournaledCopyTest : public EmptyDBTest {
protected:
    static constexpr int64_t NUM_ROWS = 128;

    void SetUp() override {
        EmptyDBTest::SetUp();
        extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        if (inMemMode || !std::filesystem::exists(extension)) {
            GTEST_SKIP() << "needs an on-disk database and the vector extension";
        }
        csvPath = (std::filesystem::temp_directory_path() /
                   ("vector_index_journaled_copy_" + std::to_string(::getpid()) + ".csv"))
                      .string();
    }

    void TearDown() override {
        std::filesystem::remove(csvPath);
        EmptyDBTest::TearDown();
    }

    static std::string vectorOf(int64_t id) {
        return "[" + std::to_string(id % 17) + ".0," + std::to_string(id % 23) + ".0," +
               std::to_string(id % 29) + ".0," + std::to_string(id) + ".0]";
    }

    void writeCsv(int64_t first, int64_t count) const {
        std::ofstream csv(csvPath);
        for (int64_t id = first; id < first + count; id++) {
            csv << id << ",\"" << vectorOf(id) << "\"\n";
        }
    }

    static void must(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        if (!result->isSuccess()) {
            fprintf(stderr, "[child] %s : %s\n", query.c_str(), result->getErrorMessage().c_str());
            _exit(5);
        }
    }

    // Le processus enfant écrit puis se tue base ouverte.
    void writeThenDie(const std::function<void(Connection&)>& body) {
        const auto child = fork();
        ASSERT_GE(child, 0);
        if (child == 0) {
            auto status = 3;
            try {
                Database childDatabase(databasePath, *systemConfig);
                Connection childConn(&childDatabase);
                must(childConn, "LOAD EXTENSION '" + extension + "';");
                must(childConn, "CALL auto_checkpoint=false;");
                must(childConn, "CALL force_checkpoint_on_copy=false;");
                body(childConn);
                kill(getpid(), SIGKILL);
            } catch (const std::exception& e) {
                fprintf(stderr, "[child] %s\n", e.what());
                status = 4;
            }
            _exit(status);
        }
        int status = 0;
        ASSERT_EQ(waitpid(child, &status, 0), child);
        ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
            << "the child must die by SIGKILL with the database open; exit code "
            << (WIFEXITED(status) ? WEXITSTATUS(status) : -1);
    }

    void schemaAndIndex(Connection& child) {
        must(child, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
        must(child, "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
        must(child, "CHECKPOINT;");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    uint64_t journalSize() const {
        std::error_code ec;
        const auto size = std::filesystem::file_size(
            rag3db::storage::StorageUtils::getWALFilePath(databasePath), ec);
        return ec ? 0 : size;
    }

    void reopen() {
        createDBAndConn();
        ok("LOAD EXTENSION '" + extension + "';");
    }

    // Chaque ligne de [first, first + count) se retrouve en tête par le vecteur donné.
    void expectEachFound(int64_t first, int64_t count,
        const std::function<std::string(int64_t)>& vector, const std::string& when) {
        int64_t numMissed = 0;
        std::string firstMiss;
        for (int64_t id = first; id < first + count; id++) {
            auto result = conn->query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', " +
                                      vector(id) + ", 1) RETURN node.id ORDER BY distance;");
            const int64_t found =
                !result->isSuccess() ? -2 :
                result->hasNext() ? result->getNext()->getValue(0)->getValue<int64_t>() : -1;
            if (found != id && numMissed++ == 0) {
                firstMiss = "row " + std::to_string(id) + " gives " + std::to_string(found) +
                            (result->isSuccess() ? "" : " (" + result->getErrorMessage() + ")");
            }
        }
        EXPECT_EQ(numMissed, 0) << when << " : " << numMissed << " of " << count
                                << " rows are not found by their own vector; first, "
                                << firstMiss;
    }

    void expectUsableAfterReplay() {
        ASSERT_GT(journalSize(), 0u) << "nothing must have checkpointed";
        reopen();
        auto count = conn->query("MATCH (n:Doc) RETURN count(*);");
        ASSERT_TRUE(count->isSuccess()) << count->getErrorMessage();
        EXPECT_EQ(count->getNext()->getValue(0)->getValue<int64_t>(), NUM_ROWS);
        expectEachFound(0, NUM_ROWS, vectorOf, "after the replay");
        // La reprise repose les vecteurs par SET, ligne à ligne puis par lot.
        const auto moved = [](int64_t id) {
            return "[500.0,500.0,500.0," + std::to_string(7000 + id) + ".0]";
        };
        for (int64_t id = 0; id < NUM_ROWS; id += 9) {
            ok("MATCH (n:Doc {id: " + std::to_string(id) + "}) SET n.vec = " + moved(id) + ";");
        }
        ok("MATCH (n:Doc) SET n.vec = [500.0, 500.0, 500.0, CAST(7000 + n.id AS FLOAT)];");
        expectEachFound(0, NUM_ROWS, moved, "after the updates");
        ok("CHECKPOINT;");
        reopen();
        expectEachFound(0, NUM_ROWS, moved, "after the checkpoint and reopening");
    }

    std::string extension;
    std::string csvPath;
};

// Un COPY journalisé validé, puis la mort.
TEST_F(VectorIndexJournaledCopyTest, ACommittedJournaledCopyIsIndexedAfterTheReplay) {
    writeCsv(0, NUM_ROWS);
    writeThenDie([&](Connection& child) {
        schemaAndIndex(child);
        must(child, "COPY Doc FROM '" + csvPath + "';");
    });
    expectUsableAfterReplay();
}

// La forme du paquet défait : un groupe validé, un groupe annulé, puis la mort.
TEST_F(VectorIndexJournaledCopyTest, ACommittedCopyThenARolledBackOneThenTheReplay) {
    writeThenDie([&](Connection& child) {
        schemaAndIndex(child);
        writeCsv(0, NUM_ROWS);
        must(child, "BEGIN TRANSACTION;");
        must(child, "COPY Doc FROM '" + csvPath + "';");
        must(child, "COMMIT;");
        writeCsv(NUM_ROWS, 96);
        must(child, "BEGIN TRANSACTION;");
        must(child, "COPY Doc FROM '" + csvPath + "';");
        must(child, "ROLLBACK;");
    });
    expectUsableAfterReplay();
}

} // namespace

#endif
