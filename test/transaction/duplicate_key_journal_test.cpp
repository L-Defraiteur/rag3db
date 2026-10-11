// A3′, la part reprise : un journal validé qui porte un doublon de clé primaire (fabriqué par le
// moteur d'avant le verrou de clé, sous le mode multi-écrivains — journal_with_duplicate_key/,
// trois fixtures et leurs fabriques) ne rend plus la base inouvrable. Au rejeu, la ligne en double
// est rejouée (elle garde son décalage : les enregistrements qui suivent désignent les lignes par
// décalage) ; à la fin du rejeu, s'il reste une autre ligne vivante de la même clé, elle reçoit
// une pierre tombale (relations détachées, supprimée par décalage, l'index non touché), sinon elle
// est gardée avec son entrée d'index ; la clé est nommée sur stderr. La base s'ouvre et sert.

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
        walPath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    }

    void useFixture(const std::string& name) {
        const auto fixture = TestHelper::appendRag3dbRootPath(
            "test/transaction/journal_with_duplicate_key/" + name);
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

    void reopens() {
        std::string reopenError;
        try {
            createDBAndConn();
        } catch (const std::exception& e) {
            reopenError = e.what();
        }
        ASSERT_TRUE(reopenError.empty()) << reopenError;
    }

    std::string walPath;
};

// Fixture 1 : deux transactions, chacune CREATE (:Item {id: 7}), chacune COMMIT.
TEST_F(DuplicateKeyJournalTest, TheDuplicateRowGetsATombstoneAndTheDatabaseOpens) {
    useFixture("base.rag3db");
    reopens();
    EXPECT_EQ(count("MATCH (n:Item) WHERE n.id = 7 RETURN count(n);"), 1);
    EXPECT_EQ(count("MATCH (n:Item) RETURN count(n);"), 1);
    // La base sert : une écriture, un point de reprise, une réouverture.
    ASSERT_TRUE(conn->query("CREATE (:Item {id: 8, v: 0});")->isSuccess());
    ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
    createDBAndConn();
    EXPECT_EQ(count("MATCH (n:Item) WHERE n.id = 7 RETURN count(n);"), 1);
    EXPECT_EQ(count("MATCH (n:Item) WHERE n.id = 8 RETURN count(n);"), 1);
    EXPECT_EQ(count("MATCH (n:Item) RETURN count(n);"), 2);
}

// Fixture 2 (relecture d'hotfixs) : A : 1, 7 ; B (avant le COMMIT de A) : 7, 8 ; C (après) : 9,
// SET 9.v = 99, SET 1.v = 11, (1)-[:Link {w: 1}]->(9). Les lignes de C sont désignées par
// décalage au journal : sauter B les aurait fait tomber sur la mauvaise ligne.
TEST_F(DuplicateKeyJournalTest, RowsAfterTheDuplicateKeepTheirOffsetsUpdatesAndRelationships) {
    useFixture("base2.rag3db");
    reopens();
    EXPECT_EQ(count("MATCH (n:Item) RETURN count(n);"), 4);
    EXPECT_EQ(count("MATCH (n:Item) WHERE n.id = 7 RETURN count(n);"), 1);
    EXPECT_EQ(count("MATCH (n:Item {id: 7}) RETURN n.v;"), 0);
    EXPECT_EQ(count("MATCH (n:Item {id: 8}) RETURN count(n);"), 1);
    EXPECT_EQ(count("MATCH (n:Item {id: 9}) RETURN n.v;"), 99);
    EXPECT_EQ(count("MATCH (n:Item {id: 1}) RETURN n.v;"), 11);
    EXPECT_EQ(count("MATCH (a:Item {id: 1})-[l:Link]->(b:Item {id: 9}) RETURN l.w;"), 1);
    EXPECT_EQ(count("MATCH ()-[l:Link]->() RETURN count(l);"), 1);
    // Par la clé comme par le balayage.
    EXPECT_EQ(count("MATCH (n:Item {id: 9}) RETURN count(n);"), 1);
    ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
    createDBAndConn();
    EXPECT_EQ(count("MATCH (n:Item) RETURN count(n);"), 4);
    EXPECT_EQ(count("MATCH (a:Item {id: 1})-[l:Link]->(b:Item {id: 9}) RETURN l.w;"), 1);
}

// Fixture 3 : comme la 2, puis D supprime la 7 de A (v = 0). La 7 de B (v = 1) est la seule
// vivante à la fin : gardée, avec son entrée d'index.
TEST_F(DuplicateKeyJournalTest, TheSurvivingDuplicateIsKeptWhenTheFirstRowWasDeletedLater) {
    useFixture("base3.rag3db");
    reopens();
    EXPECT_EQ(count("MATCH (n:Item) RETURN count(n);"), 4);
    EXPECT_EQ(count("MATCH (n:Item) WHERE n.id = 7 RETURN count(n);"), 1);
    EXPECT_EQ(count("MATCH (n:Item) WHERE n.id = 7 RETURN n.v;"), 1);
    EXPECT_EQ(count("MATCH (n:Item {id: 7}) RETURN n.v;"), 1); // par la clé : l'entrée donnée
    EXPECT_EQ(count("MATCH (n:Item {id: 9}) RETURN n.v;"), 99);
    EXPECT_EQ(count("MATCH (a:Item {id: 1})-[l:Link]->(b:Item {id: 9}) RETURN l.w;"), 1);
    ASSERT_TRUE(conn->query("MATCH (n:Item {id: 7}) SET n.v = 77;")->isSuccess());
    ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
    createDBAndConn();
    EXPECT_EQ(count("MATCH (n:Item {id: 7}) RETURN n.v;"), 77);
    EXPECT_EQ(count("MATCH (n:Item) RETURN count(n);"), 4);
}

} // namespace
