// Un seul écrivain qui insère des vecteurs, un lecteur qui cherche dans l'index (marche
// A5 bis). C'est le régime de rag3weaver : il n'écrit que des tables à vecteurs et le
// démon y cherche en permanence. Le commit de l'écrivain ajoute ses lignes au dernier bloc
// en mémoire de la table ; sur une colonne vecteur, l'ajout réalloue le tampon du bloc. La
// recherche HNSW lit les vecteurs de ce même bloc sans prendre le verrou du groupe
// (NodeTable::lookup<false>, extension/vector/src/index/hnsw_graph.cpp) : elle peut lire
// un tampon que l'écrivain vient de rendre. Le test vérifie que chaque recherche aboutit
// et rend ce qu'elle doit ; c'est sous ThreadSanitizer qu'il prouve la course.

#include <atomic>
#include <cstdio>
#include <filesystem>
#include <string>
#include <thread>
#include <vector>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "test_helper/test_helper.h"

using namespace rag3db::testing;

namespace {

class VectorSearchDuringInsertTest : public EmptyDBTest {
protected:
    static constexpr int64_t NUM_BASE_DOCS = 300;
    static constexpr int64_t NUM_INSERTS = 3000;
    static constexpr int64_t TOP_K = 10;

    void SetUp() override {
        EmptyDBTest::SetUp();
        const auto extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        if (!std::filesystem::exists(extension)) {
            GTEST_SKIP() << "the vector extension is not built (-DBUILD_EXTENSIONS=vector)";
        }
        // Sous ThreadSanitizer, l'espace d'adressage ne suffit pas aux 8 To par défaut.
        systemConfig->maxDBSize = 1024ull * 1024 * 1024 * 1024;
        createDBAndConn();
        ok(*conn, "LOAD EXTENSION '" + extension + "';");
        ok(*conn, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[8]);");
        ok(*conn, "UNWIND range(0, " + std::to_string(NUM_BASE_DOCS - 1) +
                      ") AS i CREATE (:Doc {id: i, vec: " + docVector("i") + "});");
        ok(*conn, "CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    }

    // Le vecteur d'un document, en Cypher, à partir d'une expression entière.
    static std::string docVector(const std::string& variable) {
        static constexpr int multipliers[8] = {37, 53, 71, 89, 97, 13, 29, 41};
        static constexpr int moduli[8] = {101, 103, 107, 109, 113, 127, 131, 137};
        std::string out = "[";
        for (auto i = 0u; i < 8; ++i) {
            out += (i == 0 ? "" : ", ");
            out += "CAST((" + variable + " * " + std::to_string(multipliers[i]) + ") % " +
                   std::to_string(moduli[i]) + " AS FLOAT) / " + std::to_string(moduli[i]);
        }
        return out + "]";
    }

    // Le même vecteur, en littéral : la sonde d'une recherche ne peut pas être une propriété.
    static std::string literalVector(int64_t id) {
        static constexpr int multipliers[8] = {37, 53, 71, 89, 97, 13, 29, 41};
        static constexpr int moduli[8] = {101, 103, 107, 109, 113, 127, 131, 137};
        std::string out = "[";
        for (auto i = 0u; i < 8; ++i) {
            char buffer[32];
            snprintf(buffer, sizeof(buffer), "%.9g",
                static_cast<double>((id * multipliers[i]) % moduli[i]) / moduli[i]);
            out += (i == 0 ? "" : ", ");
            out += buffer;
        }
        return out + "]";
    }

    static void ok(rag3db::main::Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
};

TEST_F(VectorSearchDuringInsertTest, SearchesWhileOneWriterInsertsVectors) {
    std::atomic<bool> writerDone{false};
    std::atomic<int64_t> numFailures{0};
    std::atomic<int64_t> numSearches{0};

    auto searcher = [&](int64_t first) {
        rag3db::main::Connection reader(database.get());
        int64_t turn = first;
        while (!writerDone.load()) {
            // La sonde est le vecteur d'un document de base : il existe depuis le début,
            // la recherche doit le rendre en tête, à distance nulle.
            const auto probe = turn++ % NUM_BASE_DOCS;
            auto result = reader.query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', " +
                                       literalVector(probe) + ", " + std::to_string(TOP_K) +
                                       ") RETURN node.id, distance ORDER BY distance, node.id;");
            if (!result->isSuccess() || result->getNumTuples() != TOP_K) {
                if (numFailures++ == 0) {
                    ADD_FAILURE() << "probe " << probe << " : "
                                  << (result->isSuccess() ?
                                             std::to_string(result->getNumTuples()) + " rows" :
                                             result->getErrorMessage());
                }
                continue;
            }
            auto nearest = result->getNext();
            if (nearest->getValue(0)->getValue<int64_t>() != probe ||
                nearest->getValue(1)->getValue<double>() > 1e-6) {
                numFailures++;
            }
            numSearches++;
        }
    };

    std::vector<std::thread> readers;
    readers.emplace_back(searcher, 0);
    readers.emplace_back(searcher, 150);

    for (int64_t k = NUM_BASE_DOCS; k < NUM_BASE_DOCS + NUM_INSERTS; k++) {
        ok(*conn, "CREATE (:Doc {id: " + std::to_string(k) +
                      ", vec: " + docVector(std::to_string(k)) + "});");
    }
    writerDone.store(true);
    for (auto& reader : readers) {
        reader.join();
    }

    EXPECT_EQ(numFailures.load(), 0);
    EXPECT_GT(numSearches.load(), 0);
}

} // namespace
