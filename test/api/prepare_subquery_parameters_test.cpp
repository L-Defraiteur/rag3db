// Un paramètre écrit dans une sous-requête (EXISTS, COUNT) doit être vu par la préparation même
// quand un autre paramètre, dehors, fait remplacer l'expression entière avant sa liaison.
// Avant le correctif : la préparation n'enregistrait pas le paramètre de dedans, l'exécution
// l'ignorait, et le rebind le remplaçait par NULL en silence (aucune ligne, aucune erreur).
#include "api_test/api_test.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class PreparedSubqueryParameters : public ApiTest {
protected:
    void SetUp() override {
        ApiTest::SetUp();
        for (auto q : {"CREATE NODE TABLE Memory(_uuid STRING, kind STRING, PRIMARY KEY(_uuid));",
                 "CREATE NODE TABLE Ref(_uuid STRING, valeur STRING, PRIMARY KEY(_uuid));",
                 "CREATE REL TABLE CITE(FROM Memory TO Ref);",
                 "CREATE (:Memory {_uuid: 'f2', kind: 'rule'});",
                 "CREATE (:Memory {_uuid: 'f3', kind: 'note'});",
                 "CREATE (:Ref {_uuid: 'r2', valeur: 'd:src/a.rs#m::f'});",
                 "MATCH (m:Memory {_uuid: 'f2'}), (r:Ref {_uuid: 'r2'}) CREATE (m)-[:CITE]->(r);",
                 "MATCH (m:Memory {_uuid: 'f3'}), (r:Ref {_uuid: 'r2'}) CREATE (m)-[:CITE]->(r);"}) {
            auto result = conn->query(q);
            ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
        }
    }

    static std::vector<std::string> column(QueryResult& result) {
        std::vector<std::string> values;
        while (result.hasNext()) {
            values.push_back(result.getNext()->getValue(0)->toString());
        }
        return values;
    }
};

// La recette du ticket : un paramètre dans le WHERE du EXISTS, un autre dehors.
TEST_F(PreparedSubqueryParameters, ExistsWithAParameterInsideAndAnotherOutside) {
    auto prepared = conn->prepare(
        "MATCH (m:Memory) WHERE EXISTS { MATCH (m)-[:CITE]->(t:Ref) WHERE t.valeur STARTS WITH "
        "$value_dir } AND m.kind = $k RETURN m._uuid ORDER BY m._uuid;");
    ASSERT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
    auto result = conn->execute(prepared.get(), std::make_pair(std::string("value_dir"), "d:src/"),
        std::make_pair(std::string("k"), "rule"));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    EXPECT_EQ(column(*result), std::vector<std::string>{"f2"});

    // La même instruction préparée, rejouée avec d'autres valeurs : les deux paramètres comptent.
    result = conn->execute(prepared.get(), std::make_pair(std::string("value_dir"), "x:"),
        std::make_pair(std::string("k"), "rule"));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    EXPECT_TRUE(column(*result).empty());
    result = conn->execute(prepared.get(), std::make_pair(std::string("value_dir"), "d:src/"),
        std::make_pair(std::string("k"), "note"));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    EXPECT_EQ(column(*result), std::vector<std::string>{"f3"});
}

// Le paramètre de dedans est enregistré par la préparation : l'oublier à l'exécution est refusé,
// jamais remplacé par NULL en silence.
TEST_F(PreparedSubqueryParameters, AMissingInnerParameterIsRefused) {
    auto prepared = conn->prepare(
        "MATCH (m:Memory) WHERE EXISTS { MATCH (m)-[:CITE]->(t:Ref) WHERE t.valeur STARTS WITH "
        "$value_dir } AND m.kind = $k RETURN m._uuid;");
    ASSERT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
    auto result = conn->execute(prepared.get(), std::make_pair(std::string("k"), "rule"));
    ASSERT_FALSE(result->isSuccess());
    EXPECT_EQ(result->getErrorMessage(), "Parameter value_dir not found.");
}

// Le paramètre de dedans est dans les propriétés d'un motif du EXISTS, pas dans son WHERE.
TEST_F(PreparedSubqueryParameters, ExistsWithAParameterInAPatternAndAnotherOutside) {
    auto prepared = conn->prepare("MATCH (m:Memory) WHERE EXISTS { MATCH (m)-[:CITE]->(t:Ref "
                                  "{valeur: $v}) } AND m.kind = $k RETURN m._uuid;");
    ASSERT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
    auto result = conn->execute(prepared.get(),
        std::make_pair(std::string("v"), "d:src/a.rs#m::f"), std::make_pair(std::string("k"), "rule"));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    EXPECT_EQ(column(*result), std::vector<std::string>{"f2"});
}

// Le même chemin avec une sous-requête COUNT.
TEST_F(PreparedSubqueryParameters, CountSubqueryWithAParameterInsideAndAnotherOutside) {
    auto prepared = conn->prepare(
        "MATCH (m:Memory) WHERE m.kind = $k RETURN COUNT { MATCH (m)-[:CITE]->(t:Ref) WHERE "
        "t.valeur STARTS WITH $value_dir } + $zero;");
    ASSERT_TRUE(prepared->isSuccess()) << prepared->getErrorMessage();
    auto result = conn->execute(prepared.get(), std::make_pair(std::string("value_dir"), "d:src/"),
        std::make_pair(std::string("k"), "rule"), std::make_pair(std::string("zero"), (int64_t)0));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    EXPECT_EQ(column(*result), std::vector<std::string>{"1"});
}

// Les trois cas qui passaient déjà, gardés comme contrôle.
TEST_F(PreparedSubqueryParameters, TheOtherFormsStillAnswer) {
    auto inside = conn->prepare("MATCH (m:Memory) WHERE EXISTS { MATCH (m)-[:CITE]->(t:Ref) WHERE "
                                "t.valeur STARTS WITH $value_dir } RETURN m._uuid ORDER BY m._uuid;");
    auto result = conn->execute(inside.get(), std::make_pair(std::string("value_dir"), "d:src/"));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    EXPECT_EQ(column(*result), (std::vector<std::string>{"f2", "f3"}));

    auto literal = conn->prepare("MATCH (m:Memory) WHERE EXISTS { MATCH (m)-[:CITE]->(t:Ref) WHERE "
                                 "t.valeur STARTS WITH 'd:src/' } AND m.kind = $k RETURN m._uuid;");
    result = conn->execute(literal.get(), std::make_pair(std::string("k"), "rule"));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    EXPECT_EQ(column(*result), std::vector<std::string>{"f2"});

    auto flat = conn->prepare("MATCH (m:Memory)-[:CITE]->(t:Ref) WHERE t.valeur STARTS WITH "
                              "$value_dir AND m.kind = $k RETURN m._uuid;");
    result = conn->execute(flat.get(), std::make_pair(std::string("value_dir"), "d:src/"),
        std::make_pair(std::string("k"), "rule"));
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    EXPECT_EQ(column(*result), std::vector<std::string>{"f2"});
}

} // namespace
