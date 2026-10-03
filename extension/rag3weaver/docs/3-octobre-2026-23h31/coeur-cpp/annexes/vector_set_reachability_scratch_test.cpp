// Brouillon de mesure, non livré : combien de lignes restent joignables dans un index HNSW
// après des SET de vecteur en place, en service, sans arrêt ni rejeu. Trois variantes : SET
// depuis NULL (la forme de rag3weaver : les morceaux entrent sans vecteur, puis on les pose),
// SET d'un vecteur vers un autre, tous distincts, et SET vers un vecteur déjà porté par une
// autre ligne. Chacune ligne à ligne, puis par lots de 512.

#include <cstdio>
#include <filesystem>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "test_helper/test_helper.h"

using namespace rag3db::testing;

namespace {

class VectorSetReachabilityScratch : public EmptyDBTest {
protected:
    static constexpr int64_t NUM_ROWS = 1000;

    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        ok("LOAD EXTENSION '" +
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension") +
            "';");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query.substr(0, 200) << " : "
                                         << result->getErrorMessage();
    }

    int64_t count(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        return result->isSuccess() ? static_cast<int64_t>(result->getNumTuples()) : -1;
    }

    // Deux familles de vecteurs distincts, en Cypher, à partir de l'identifiant.
    static std::string first(const std::string& id) {
        return "[CAST(" + id + " % 17 AS FLOAT), CAST(" + id + " % 23 AS FLOAT), CAST(" + id +
               " % 29 AS FLOAT), CAST(" + id + " AS FLOAT)]";
    }
    static std::string second(const std::string& id) {
        return "[CAST(" + id + " % 13 AS FLOAT) + 0.5, CAST(" + id + " % 19 AS FLOAT), CAST(" +
               id + " % 31 AS FLOAT), CAST(" + id + " AS FLOAT) + 0.25]";
    }

    void table(bool withVectors) {
        ok("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
        ok("UNWIND range(0, " + std::to_string(NUM_ROWS - 1) + ") AS i CREATE (:Doc {id: i" +
            (withVectors ? ", vec: " + first("i") : std::string()) + "});");
        ok("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    }

    // vector : une fonction de n.id, en Cypher. batch : 1 = une instruction par ligne.
    void setAll(const std::string& vector, int64_t from, int64_t to, int64_t batch) {
        for (auto start = from; start < to; start += batch) {
            ok("MATCH (n:Doc) WHERE n.id >= " + std::to_string(start) + " AND n.id < " +
                std::to_string(std::min(start + batch, to)) + " SET n.vec = " + vector + ";");
        }
    }

    void report(const std::string& label) {
        const auto withVector = count("MATCH (n:Doc) WHERE n.vec IS NOT NULL RETURN n.id;");
        const auto reachable =
            count("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [8.0, 11.0, 14.0, 500.0], " +
                  std::to_string(NUM_ROWS) + ", efs := " + std::to_string(NUM_ROWS) +
                  ") RETURN node.id;");
        fprintf(stderr, "[set] %-58s %4ld joignables sur %4ld lignes à vecteur\n", label.c_str(),
            static_cast<long>(reachable), static_cast<long>(withVector));
        ok("CALL DROP_VECTOR_INDEX('Doc', 'doc_index');");
        ok("DROP TABLE Doc;");
    }
};

TEST_F(VectorSetReachabilityScratch, Measure) {
    for (const int64_t batch : {1, 512}) {
        const auto how = batch == 1 ? std::string("ligne à ligne") : std::string("lots de 512");
        table(true);
        report("témoin : index bâti sur les vecteurs, aucun SET");

        table(false);
        setAll(first("n.id"), 0, NUM_ROWS, batch);
        report("SET depuis NULL, " + how);

        table(true);
        setAll(second("n.id"), 0, NUM_ROWS, batch);
        report("SET vers un autre vecteur, tous distincts, " + how);

        table(true);
        setAll(second("n.id"), 100, 120, batch);
        report("vingt SET vers un autre vecteur, distincts, " + how);

        table(true);
        setAll("[7.0, 7.0, 7.0, 7.0]", 100, 120, batch);
        report("vingt SET vers le même vecteur, " + how);

        table(true);
        // Chaque ligne reçoit le vecteur que porte déjà la ligne id + 500 : un doublon chacun.
        setAll(first("(n.id + 500)"), 100, 120, batch);
        report("vingt SET vers le vecteur d'une autre ligne, " + how);
    }
}

} // namespace
