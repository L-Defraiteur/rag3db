// A3′, la part reprise : un journal validé qui porte un doublon de clé primaire (fabriqué par le
// moteur d'avant le verrou de clé, sous le mode multi-écrivains : deux transactions, chacune
// CREATE (:Item {id: 7}), chacune COMMIT, mort base ouverte — journal_with_duplicate_key/) ne rend
// plus la base inouvrable : au rejeu, la transaction dont une insertion heurte le doublon est
// annulée et sautée jusqu'à son COMMIT, ses octets copiés à côté du journal, la clé nommée sur
// stderr ; la base s'ouvre avec une seule ligne de clé 7, et sert.

#include <filesystem>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/storage_utils.h"
#include "test_helper/test_helper.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class DuplicateKeyJournalTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "needs an on-disk database: the subject is its journal";
        }
        const auto fixture = TestHelper::appendRag3dbRootPath(
            "test/transaction/journal_with_duplicate_key/base.rag3db");
        walPath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
        std::filesystem::remove(databasePath);
        std::filesystem::remove(walPath);
        std::filesystem::copy_file(fixture, databasePath);
        std::filesystem::copy_file(rag3db::storage::StorageUtils::getWALFilePath(fixture), walPath);
        ASSERT_GT(std::filesystem::file_size(walPath), 0u);
    }

    int64_t count(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    // Les fichiers « écartés » à côté du journal, et la somme de leurs tailles.
    std::pair<size_t, uintmax_t> asideFiles() const {
        size_t number = 0;
        uintmax_t bytes = 0;
        const auto walName = std::filesystem::path(walPath).filename().string();
        for (const auto& entry :
            std::filesystem::directory_iterator(std::filesystem::path(walPath).parent_path())) {
            const auto name = entry.path().filename().string();
            if (name.rfind(walName + ".ecarte-", 0) == 0) {
                number++;
                bytes += entry.file_size();
            }
        }
        return {number, bytes};
    }

    std::string walPath;
};

TEST_F(DuplicateKeyJournalTest, TheFaultyTransactionIsSkippedAndTheDatabaseOpens) {
    std::string reopenError;
    try {
        createDBAndConn();
    } catch (const std::exception& e) {
        reopenError = e.what();
    }
    ASSERT_TRUE(reopenError.empty()) << reopenError;
    EXPECT_EQ(count("MATCH (n:Item) WHERE n.id = 7 RETURN count(n);"), 1);
    EXPECT_EQ(count("MATCH (n:Item) RETURN count(n);"), 1);
    // Les octets de la transaction sautée sont à côté du journal, pas perdus.
    const auto [asideNumber, asideBytes] = asideFiles();
    EXPECT_EQ(asideNumber, 1u);
    EXPECT_GT(asideBytes, 0u);
    EXPECT_LT(asideBytes, std::filesystem::file_size(walPath));
    // La base sert : une écriture, un point de reprise, une réouverture.
    ASSERT_TRUE(conn->query("CREATE (:Item {id: 8, v: 0});")->isSuccess());
    ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
    createDBAndConn();
    EXPECT_EQ(count("MATCH (n:Item) WHERE n.id = 7 RETURN count(n);"), 1);
    EXPECT_EQ(count("MATCH (n:Item) WHERE n.id = 8 RETURN count(n);"), 1);
    EXPECT_EQ(count("MATCH (n:Item) RETURN count(n);"), 2);
}

} // namespace
