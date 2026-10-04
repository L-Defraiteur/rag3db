// Plusieurs COPY dans une même table de nœuds, dans une transaction annulée.
//
// L'annulation retire de l'index de clé primaire les clés des lignes annulées. Avec deux COPY
// dans la transaction, les clés du premier restaient dans l'index : aucune ligne ne les porte
// plus, mais un CREATE de l'une d'elles était refusé (« Found duplicated primary key value »),
// à jamais une fois le point de reprise passé. Trouvé par la session de l'arbre principal
// (4 octobre 2026) sur le chemin d'échec de la transaction par paquet.

#include <filesystem>
#include <fstream>
#include <string>

#include <unistd.h>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::testing;

namespace {

class RolledBackCopiesTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        ok("CREATE NODE TABLE T(k STRING PRIMARY KEY, v INT64);");
        csvPath = (std::filesystem::temp_directory_path() /
                   ("rolled_back_copies_" + std::to_string(::getpid()) + ".csv"))
                      .string();
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
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    // COPY des clés k<first> … k<first + count - 1>.
    void copyKeys(int64_t first, int64_t count) {
        {
            std::ofstream csv(csvPath);
            for (int64_t i = 0; i < count; i++) {
                csv << "k" << first + i << "\n";
            }
        }
        ok("COPY T(k) FROM '" + csvPath + "' (header=false);");
    }

    // Aucune des clés annulées n'est restée : ni ligne, ni entrée d'index.
    void expectNoTraceOfTheRolledBackKeys(int64_t numRowsLeft) {
        EXPECT_EQ(single("MATCH (t:T) RETURN count(t);"), numRowsLeft);
        for (const auto* key : {"k100", "k105", "k109", "k110", "k115", "k119"}) {
            EXPECT_EQ(single(std::string("MATCH (t:T {k: '") + key + "'}) RETURN count(t);"), 0)
                << key;
        }
    }

    void expectTheRolledBackKeysCanBeCreated(int64_t numRowsLeft) {
        for (const auto* key : {"k100", "k105", "k109", "k110", "k115", "k119"}) {
            auto result = conn->query(std::string("CREATE (:T {k: '") + key + "'});");
            EXPECT_TRUE(result->isSuccess()) << key << " : " << result->getErrorMessage();
        }
        EXPECT_EQ(single("MATCH (t:T) RETURN count(t);"), numRowsLeft + 6);
        EXPECT_EQ(single("MATCH (t:T {k: 'k100'}) RETURN count(t);"), 1);
    }

    void twoCopiesRolledBack() {
        ok("BEGIN TRANSACTION;");
        copyKeys(100, 10);
        copyKeys(110, 10);
        ok("ROLLBACK;");
    }

    std::string csvPath;
};

TEST_F(RolledBackCopiesTest, NoKeyIsLeftInTheSameSession) {
    copyKeys(10000, 2500);
    twoCopiesRolledBack();
    expectNoTraceOfTheRolledBackKeys(2500);
    expectTheRolledBackKeysCanBeCreated(2500);
}

TEST_F(RolledBackCopiesTest, NoKeyIsLeftAfterReopening) {
    if (inMemMode) {
        GTEST_SKIP() << "needs an on-disk database to reopen";
    }
    copyKeys(10000, 2500);
    twoCopiesRolledBack();
    createDBAndConn(); // la fermeture écrit un point de reprise
    expectNoTraceOfTheRolledBackKeys(2500);
    expectTheRolledBackKeysCanBeCreated(2500);
}

TEST_F(RolledBackCopiesTest, NoKeyIsLeftInAnEmptyTable) {
    twoCopiesRolledBack();
    expectNoTraceOfTheRolledBackKeys(0);
    expectTheRolledBackKeysCanBeCreated(0);
}

// Trois COPY, et des lignes créées entre eux dans la même transaction.
TEST_F(RolledBackCopiesTest, NoKeyIsLeftWithWritesBetweenTheCopies) {
    copyKeys(10000, 2500);
    ok("BEGIN TRANSACTION;");
    copyKeys(100, 10);
    ok("CREATE (:T {k: 'between'});");
    copyKeys(110, 10);
    copyKeys(200, 10);
    ok("ROLLBACK;");
    expectNoTraceOfTheRolledBackKeys(2500);
    EXPECT_EQ(single("MATCH (t:T {k: 'between'}) RETURN count(t);"), 0);
    ok("CREATE (:T {k: 'between'});");
    ok("CREATE (:T {k: 'k205'});");
    expectTheRolledBackKeysCanBeCreated(2502);
}

} // namespace
