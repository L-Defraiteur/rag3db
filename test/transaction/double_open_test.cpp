// Deux exemplaires de la même base ouverts en écriture dans un même processus.
//
// Le verrou du fichier de la base (fcntl) n'exclut qu'un autre processus. Dans le même
// processus, une seconde ouverture en écriture était acceptée : elle rejouait le journal vivant
// de la première, puis chacune écrivait avec son propre gestionnaire de pages libres, et elles
// se prenaient les mêmes pages. C'est la corruption qui tuait la suite e2e_code de rag3weaver
// (une base rouverte pendant que l'exemplaire précédent finissait de se fermer) : une page de
// l'index de clé primaire à la place d'une page de colonne, sur disque. La seconde ouverture
// est maintenant refusée par son nom, Database::ALREADY_OPEN_FOR_WRITING.

#include <memory>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class DoubleOpenTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "needs an on-disk database: two in-memory databases share nothing";
        }
        createDBAndConn();
        ok("CREATE NODE TABLE T(id INT64 PRIMARY KEY, s STRING);");
        ok("UNWIND range(0, 999) AS i CREATE (:T {id: i, s: 'row ' + CAST(i AS STRING)});");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    static int64_t count(Connection& connection) {
        auto result = connection.query("MATCH (t:T) RETURN count(*);");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    std::string openError(bool readOnly) const {
        auto config = *systemConfig;
        config.readOnly = readOnly;
        try {
            Database second(databasePath, config);
        } catch (const std::exception& e) {
            return e.what();
        }
        return "";
    }
};

TEST_F(DoubleOpenTest, ASecondWritingInstanceIsRefusedByName) {
    const auto error = openError(false /* readOnly */);
    EXPECT_NE(error.find(Database::ALREADY_OPEN_FOR_WRITING), std::string::npos) << error;
    // Le refus n'a rien rejoué ni rien écrit : la première instance sert toujours.
    ok("CREATE (:T {id: 1000, s: 'after'});");
    EXPECT_EQ(count(*conn), 1001);
}

TEST_F(DoubleOpenTest, TheSamePathWrittenAnotherWayIsRefusedToo) {
    auto config = *systemConfig;
    const auto slash = databasePath.find_last_of('/');
    ASSERT_NE(slash, std::string::npos);
    const auto otherSpelling =
        databasePath.substr(0, slash) + "/./" + databasePath.substr(slash + 1);
    try {
        Database second(otherSpelling, config);
        FAIL() << "the second writing instance was accepted";
    } catch (const std::exception& e) {
        EXPECT_NE(std::string(e.what()).find(Database::ALREADY_OPEN_FOR_WRITING),
            std::string::npos)
            << e.what();
    }
}

TEST_F(DoubleOpenTest, ItOpensAgainOnceTheFirstInstanceIsClosed) {
    conn.reset();
    database.reset();
    EXPECT_EQ(openError(false /* readOnly */), "");
    createDBAndConn();
    EXPECT_EQ(count(*conn), 1000);
}

// Une ouverture refusée pour une autre raison ne laisse pas la marque derrière elle.
TEST_F(DoubleOpenTest, AFailedOpenLeavesNoMark) {
    ASSERT_NE(openError(false /* readOnly */), "");
    conn.reset();
    database.reset();
    EXPECT_EQ(openError(false /* readOnly */), "");
}

// Un lecteur du même processus reste permis : il n'écrit pas et ne prend aucun verrou.
TEST_F(DoubleOpenTest, AReadOnlyInstanceBesideTheWriterStillOpens) {
    ok("CHECKPOINT;");
    auto config = *systemConfig;
    config.readOnly = true;
    Database reader(databasePath, config);
    Connection readerConn(&reader);
    EXPECT_EQ(count(readerConn), 1000);
}

} // namespace
