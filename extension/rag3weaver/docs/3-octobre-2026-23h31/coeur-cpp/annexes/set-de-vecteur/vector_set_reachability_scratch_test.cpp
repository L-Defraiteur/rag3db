// Brouillon de mesure, non livré : combien de lignes restent joignables dans un index HNSW
// après des SET de vecteur en place, en service, sans arrêt ni rejeu. Trois variantes : SET
// depuis NULL (la forme de rag3weaver : les morceaux entrent sans vecteur, puis on les pose),
// SET d'un vecteur vers un autre, tous distincts, et SET vers un vecteur déjà porté par une
// autre ligne. Chacune ligne à ligne, puis par lots de 512.

#include <cstdio>
#include <filesystem>
#include <string>
#include <vector>

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

TEST_F(VectorSetReachabilityScratch, BatchSizes) {
    for (const int64_t batch : {2, 8, 32, 128, 512, 1000}) {
        table(true);
        setAll(second("n.id"), 0, NUM_ROWS, batch);
        report("SET vers un autre vecteur, lots de " + std::to_string(batch));
    }
    // Les mêmes mille mises à jour, dans une seule transaction, une instruction par ligne.
    table(true);
    ok("BEGIN TRANSACTION;");
    setAll(second("n.id"), 0, NUM_ROWS, 1);
    ok("COMMIT;");
    report("SET ligne à ligne dans un seul BEGIN … COMMIT");
}

TEST_F(VectorSetReachabilityScratch, OneBatch) {
    table(true);
    setAll(second("n.id"), 0, 40, 40);
    report("quarante SET en une instruction");
}

// Les contournements possibles côté appelant, pour mille lignes dont le vecteur change.
TEST_F(VectorSetReachabilityScratch, Workarounds) {
    for (const int64_t batch : {1, 512}) {
        const auto how = batch == 1 ? std::string("ligne à ligne") : std::string("lots de 512");
        // Supprimer puis réinsérer les lignes.
        table(true);
        for (int64_t start = 0; start < NUM_ROWS; start += batch) {
            const auto end = std::min(start + batch, NUM_ROWS);
            ok("MATCH (n:Doc) WHERE n.id >= " + std::to_string(start) + " AND n.id < " +
                std::to_string(end) + " DELETE n;");
            ok("UNWIND range(" + std::to_string(start) + ", " + std::to_string(end - 1) +
                ") AS i CREATE (:Doc {id: i, vec: " + second("i") + "});");
        }
        report("supprimer puis réinsérer, " + how);
        // Repasser par NULL : SET à NULL, puis SET au nouveau vecteur, deux instructions.
        table(true);
        for (int64_t start = 0; start < NUM_ROWS; start += batch) {
            const auto range = "MATCH (n:Doc) WHERE n.id >= " + std::to_string(start) +
                               " AND n.id < " + std::to_string(std::min(start + batch, NUM_ROWS));
            ok(range + " SET n.vec = NULL;");
            ok(range + " SET n.vec = " + second("n.id") + ";");
        }
        report("repasser par NULL, " + how);
    }
    // Retirer l'index, poser les vecteurs, recréer l'index.
    table(true);
    ok("CALL DROP_VECTOR_INDEX('Doc', 'doc_index');");
    setAll(second("n.id"), 0, NUM_ROWS, 512);
    ok("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    report("retirer l'index, SET par lots de 512, recréer l'index");
}

// Le plus petit cas qui perd : des lots de deux lignes, sur les K premières lignes.
TEST_F(VectorSetReachabilityScratch, SmallestLoss) {
    for (const int64_t rows : {2, 4, 10, 20, 50, 100, 200}) {
        table(true);
        setAll(second("n.id"), 0, rows, 2);
        auto result = conn->query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [8.0, 11.0, 14.0, "
                                  "500.0], 1000, efs := 1000) RETURN node.id;");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        std::vector<bool> seen(NUM_ROWS, false);
        while (result->hasNext()) {
            seen[result->getNext()->getValue(0)->getValue<int64_t>()] = true;
        }
        std::string missing;
        int64_t numMissing = 0;
        for (int64_t id = 0; id < NUM_ROWS; ++id) {
            if (!seen[id]) {
                numMissing++;
                if (numMissing <= 12) {
                    missing += " " + std::to_string(id);
                }
            }
        }
        fprintf(stderr, "[set] lots de 2 sur les %ld premières lignes : %ld manquantes :%s\n",
            static_cast<long>(rows), static_cast<long>(numMissing), missing.c_str());
        ok("CALL DROP_VECTOR_INDEX('Doc', 'doc_index');");
        ok("DROP TABLE Doc;");
    }
}

TEST_F(VectorSetReachabilityScratch, EdgesInsideOneStatement) {
    table(true);
    setAll(second("n.id"), 0, 6, 6);
    report("six SET en une instruction");
}

TEST_F(VectorSetReachabilityScratch, EdgesAfterCommit) {
    table(true);
    setAll(second("n.id"), 500, 501, 1); // avant : les arêtes d'origine du nœud 0
    setAll(second("n.id"), 0, 1, 1);     // la mise à jour du nœud 0, vue de l'intérieur
    setAll(second("n.id"), 501, 502, 1); // après le commit
    ok("CHECKPOINT;");
    setAll(second("n.id"), 502, 503, 1); // après un point de reprise
    report("quatre SET, un par instruction");
}

TEST_F(VectorSetReachabilityScratch, EdgesAfterAMultiRowStatement) {
    table(true);
    setAll(second("n.id"), 0, 3, 3);     // trois lignes en une instruction : 0, 1, 2
    setAll(second("n.id"), 500, 501, 1); // puis une autre instruction : le nœud 0, validé
    report("trois SET en une instruction, puis un");
}

} // namespace
