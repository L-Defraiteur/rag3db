// Le mot NULL convenu d'un COPY (null_strings) vaut pour toutes les colonnes, quel que soit
// leur type (header=false : une ligne seule serait prise pour un en-tête). Avant le correctif, il n'était consulté que pour une colonne texte : dans une
// colonne INT64, le moteur tentait la conversion et refusait le fichier entier (« Could not
// convert "__nul__" to INT64 »).
#include <filesystem>
#include <fstream>
#include <string>
#include <unistd.h>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class CopyNullStrings : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        csvPath = (std::filesystem::temp_directory_path() /
                   ("copy_null_strings_" + std::to_string(::getpid()) + ".csv"))
                      .string();
    }
    void TearDown() override {
        std::filesystem::remove(csvPath);
        EmptyDBTest::TearDown();
    }
    void writeCsv(const std::string& content) const {
        std::ofstream csv(csvPath);
        csv << content;
    }
    void run(const std::string& q) const {
        auto result = conn->query(q);
        ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
    }
    std::string row(const std::string& q) const {
        auto result = conn->query(q);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        if (!result->hasNext()) {
            return "AUCUNE LIGNE";
        }
        auto line = result->getNext()->toString();
        return result->hasNext() ? "PLUSIEURS LIGNES" : line;
    }

    std::string csvPath;
};

// La recette du ticket : une relation à colonne INT64, nommée seule dans le COPY.
TEST_F(CopyNullStrings, TheNullWordIsReadInAnIntegerColumnOfARelation) {
    run("CREATE NODE TABLE A(_uuid STRING, PRIMARY KEY(_uuid));");
    run("CREATE NODE TABLE B(_uuid STRING, PRIMARY KEY(_uuid));");
    run("CREATE REL TABLE R(FROM A TO B, raison STRING, poids INT64);");
    run("CREATE (:A {_uuid: 'a'}); CREATE (:B {_uuid: 'b'});");
    writeCsv("a,b,__nul__\n");
    run("COPY R (poids) FROM '" + csvPath + "' (from='A', to='B', header=false, null_strings=['__nul__']);");
    EXPECT_EQ(row("MATCH (:A)-[r:R]->(:B) RETURN count(*), r.poids IS NULL, r.raison IS NULL;"),
        "1|True|True\n");
}

// Le mot vaut pour chaque type de colonne d'un nœud ; entre guillemets, c'est une valeur.
TEST_F(CopyNullStrings, TheNullWordIsReadInEveryColumnOfANode) {
    run("CREATE NODE TABLE N(id INT64, n INT64, d DOUBLE, b BOOLEAN, s STRING, PRIMARY KEY(id));");
    writeCsv("1,__nul__,__nul__,__nul__,__nul__\n2,2,2.5,true,\"__nul__\"\n");
    run("COPY N FROM '" + csvPath + "' (header=false, null_strings=['__nul__']);");
    EXPECT_EQ(row("MATCH (x:N {id: 1}) RETURN x.n IS NULL, x.d IS NULL, x.b IS NULL, x.s IS NULL;"),
        "True|True|True|True\n");
    EXPECT_EQ(row("MATCH (x:N {id: 2}) RETURN x.n, x.d, x.b, x.s;"), "2|2.500000|True|__nul__\n");
}

// Entre guillemets, le mot reste une valeur, et une colonne entière la refuse par son nom.
TEST_F(CopyNullStrings, TheQuotedWordStaysAValue) {
    run("CREATE NODE TABLE N(id INT64, n INT64, PRIMARY KEY(id));");
    writeCsv("1,\"__nul__\"\n");
    auto result = conn->query("COPY N FROM '" + csvPath + "' (header=false, null_strings=['__nul__']);");
    ASSERT_FALSE(result->isSuccess());
    EXPECT_NE(result->getErrorMessage().find("__nul__"), std::string::npos)
        << result->getErrorMessage();
}

// Sans l'option, ce qui se lisait NULL se lit toujours NULL : la cellule vide et le mot NULL.
TEST_F(CopyNullStrings, WithoutTheOptionTheEmptyCellAndTheWordNullStillRead) {
    run("CREATE NODE TABLE N(id INT64, n INT64, d DOUBLE, s STRING, PRIMARY KEY(id));");
    writeCsv("1,,NULL,\n2,null,\"NULL\",NULL\n");
    run("COPY N FROM '" + csvPath + "' (header=false);");
    EXPECT_EQ(row("MATCH (x:N {id: 1}) RETURN x.n IS NULL, x.d IS NULL, x.s IS NULL;"),
        "True|True|True\n");
    EXPECT_EQ(row("MATCH (x:N {id: 2}) RETURN x.n IS NULL, x.d IS NULL, x.s;"),
        "True|True|NULL\n");
}

} // namespace
