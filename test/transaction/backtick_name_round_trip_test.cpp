#include <filesystem>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

// Un accent grave doublé dans un nom (ticket docs/tickets/2026-10-04-accent-grave-double-dans-un-nom.md).
// En Cypher, `a``b` nomme a`b. L'analyseur ne réduisait pas l'accent grave doublé : le nom stocké
// était a``b. Le réduire seul ne suffit pas : un EXPORT écrit les noms entre accents graves dans du
// Cypher qu'un IMPORT relit, et il doit alors doubler l'accent grave d'un nom. Le témoin : une table de
// nœuds, sa clé, une propriété, une table de relations et sa propriété, tous nommés avec un accent grave ;
// les noms stockés ; puis EXPORT, DROP et IMPORT, et les mêmes noms et les mêmes lignes relus.

namespace rag3db {
namespace testing {

class BacktickNameRoundTripTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "needs an on-disk database: EXPORT and IMPORT";
        }
        createDBAndConn();
    }
    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    std::string one(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        if (!result->isSuccess() || !result->hasNext()) {
            return "<none>";
        }
        return result->getNext()->getValue(0)->toString();
    }
    std::string tableNames() {
        auto result = conn->query("CALL show_tables() RETURN name ORDER BY name;");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        std::string names;
        while (result->isSuccess() && result->hasNext()) {
            names += "[" + result->getNext()->getValue(0)->toString() + "]";
        }
        return names;
    }
    void expectTheGraph(const std::string& when) {
        EXPECT_EQ(tableNames(), "[a`b][l`m]") << "[check: backtick-names-" << when << "] tables";
        EXPECT_EQ(one("MATCH (n:`a``b` {`p``q`: 1}) RETURN n.`r``s`;"), "one")
            << "[check: backtick-names-" << when << "] a node by its key and property";
        EXPECT_EQ(one("MATCH (:`a``b`)-[e:`l``m`]->(:`a``b`) RETURN e.`w``x`;"), "7")
            << "[check: backtick-names-" << when << "] a relationship and its property";
        EXPECT_EQ(one("CALL table_info('a`b') RETURN name ORDER BY name LIMIT 1;"), "p`q")
            << "[check: backtick-names-" << when << "] a property name";
    }
};

TEST_F(BacktickNameRoundTripTest, NamesWithABacktickSurviveExportAndImport) {
    ok("CREATE NODE TABLE `a``b`(`p``q` INT64 PRIMARY KEY, `r``s` STRING);");
    ok("CREATE REL TABLE `l``m`(FROM `a``b` TO `a``b`, `w``x` INT64);");
    ok("CREATE (:`a``b` {`p``q`: 1, `r``s`: 'one'}), (:`a``b` {`p``q`: 2, `r``s`: 'two'});");
    ok("MATCH (x:`a``b` {`p``q`: 1}), (y:`a``b` {`p``q`: 2}) CREATE (x)-[:`l``m` {`w``x`: 7}]->(y);");
    if (HasFatalFailure()) {
        return;
    }
    expectTheGraph("stored");
    const auto dir = databasePath + ".export";
    std::filesystem::remove_all(dir);
    ok("EXPORT DATABASE '" + dir + "';");
    ok("DROP TABLE `l``m`;");
    ok("DROP TABLE `a``b`;");
    ok("IMPORT DATABASE '" + dir + "';");
    if (HasFatalFailure()) {
        return;
    }
    expectTheGraph("after-import");
    std::filesystem::remove_all(dir);
}

} // namespace testing
} // namespace rag3db
