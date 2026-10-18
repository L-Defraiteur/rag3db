// Un COPY après DROP_VECTOR_INDEX (ticket du 18 octobre 2026 : « un COPY après le retrait de
// l'index vectoriel est refusé, la finalisation du COPY touche l'index retiré »).
//
// Depuis la rétention des index (9b861579a), un DROP validé ne retire pas le holder de la liste
// de la table : les lecteurs d'avant le voient encore. La finalisation d'un COPY
// (NodeBatchInsert::finalize) parcourait tous les holders, sans visibilité ; l'index HNSW retiré,
// voyant plus de lignes que reprises, cherchait ses tables internes au catalogue, parties avec le
// DROP : « _UPPER does not exist in catalog », et le COPY était refusé. C'est le chemin de
// l'ingestion en masse du produit (DROP, COPY, CREATE), qu'aucune suite ne jouait. Se saute si
// l'extension vector n'est pas bâtie.

#include <filesystem>
#include <fstream>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "test_helper/test_helper.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class CopyAfterDropVectorIndex : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        extension =
            TestHelper::appendRag3dbRootPath("extension/vector/build/libvector.rag3db_extension");
        if (!std::filesystem::exists(extension)) {
            GTEST_SKIP() << "the vector extension is not built (-DBUILD_EXTENSIONS=vector)";
        }
        createDBAndConn();
        ok("LOAD EXTENSION '" + extension + "';");
        ok("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
        ok("CREATE REL TABLE Link(FROM Doc TO Doc);");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    int64_t scalar(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }
    std::string docsCsv(const std::string& name, int64_t firstId, int64_t count) {
        const auto path = databasePath + "." + name + ".csv";
        std::ofstream csv(path);
        for (int64_t i = 0; i < count; i++) {
            const auto id = firstId + i;
            csv << id << ",\"[" << id << ".0,1.0,2.0," << (id % 7) << ".0]\"\n";
        }
        return path;
    }
    std::string linksCsv(const std::string& name, int64_t firstId, int64_t count) {
        const auto path = databasePath + "." + name + ".csv";
        std::ofstream csv(path);
        for (int64_t i = 1; i < count; i++) {
            csv << firstId + i << "," << firstId + i - 1 << "\n";
        }
        return path;
    }
    void createIndex() {
        ok("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2', "
           "skip_if_exists := true);");
    }
    void dropIndex() {
        ok("CALL DROP_VECTOR_INDEX('Doc', 'doc_index', skip_if_not_exists := true);");
    }
    // La recherche du vecteur exact de `id` le rend en tête : la ligne est dans l'index.
    int64_t nearest(int64_t id) {
        auto result = conn->query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', CAST([" +
                                  std::to_string(id) + ".0,1.0,2.0," + std::to_string(id % 7) +
                                  ".0] AS FLOAT[4]), 1) RETURN node.id;");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }
    // Tout de suite, après un point de reprise, puis après réouverture : la table, les liens et
    // l'index.
    void expectThrough(int64_t numDocs, int64_t numLinks, int64_t probeId) {
        auto check = [&](const std::string& when) {
            EXPECT_EQ(scalar("MATCH (d:Doc) RETURN count(*);"), numDocs) << when;
            EXPECT_EQ(scalar("MATCH ()-[l:Link]->() RETURN count(l);"), numLinks) << when;
            EXPECT_EQ(nearest(probeId), probeId) << when;
        };
        check("tout de suite");
        ok("CHECKPOINT;");
        check("après le point de reprise");
        if (inMemMode) {
            return;
        }
        createDBAndConn();
        ok("LOAD EXTENSION '" + extension + "';");
        check("après réouverture");
    }

    std::string extension;
};

TEST_F(CopyAfterDropVectorIndex, ACopyIntoAnEmptyTableAfterTheDropIsAccepted) {
    createIndex();
    dropIndex();
    ok("COPY Doc FROM '" + docsCsv("docs", 0, 3000) + "' (header=false);");
    ok("COPY Link FROM '" + linksCsv("links", 0, 3000) + "' (header=false);");
    createIndex();
    expectThrough(3000, 2999, 1234);
}

TEST_F(CopyAfterDropVectorIndex, TheBulkPathOfTheProductOnAFilledTable) {
    // Une table déjà remplie par un COPY, avec son index repris ; puis le chemin de masse :
    // DROP, COPY d'un second lot, CREATE.
    ok("COPY Doc FROM '" + docsCsv("docs1", 0, 2000) + "' (header=false);");
    createIndex();
    ok("CHECKPOINT;");
    dropIndex();
    ok("COPY Doc FROM '" + docsCsv("docs2", 2000, 2000) + "' (header=false);");
    ok("COPY Link FROM '" + linksCsv("links", 2000, 2000) + "' (header=false);");
    createIndex();
    expectThrough(4000, 1999, 3210);
}

TEST_F(CopyAfterDropVectorIndex, ACopyInABlockAfterTheDropIsAcceptedToo) {
    createIndex();
    dropIndex();
    ok("BEGIN TRANSACTION;");
    ok("COPY Doc FROM '" + docsCsv("docs", 0, 1500) + "' (header=false);");
    ok("COPY Link FROM '" + linksCsv("links", 0, 1500) + "' (header=false);");
    ok("COMMIT;");
    createIndex();
    expectThrough(1500, 1499, 77);
}

TEST_F(CopyAfterDropVectorIndex, AReaderFromBeforeTheDropStillSearchesTheOldIndex) {
    // La rétention : un lecteur ouvert avant le DROP garde l'ancien index entier ; le COPY
    // d'après le DROP ne doit pas le perturber.
    ok("COPY Doc FROM '" + docsCsv("docs1", 0, 500) + "' (header=false);");
    createIndex();
    ok("CHECKPOINT;");
    auto reader = std::make_unique<Connection>(database.get());
    ASSERT_TRUE(reader->query("BEGIN TRANSACTION READ ONLY;")->isSuccess());
    auto before = reader->query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', CAST([42.0,1.0,2.0,0.0] AS FLOAT[4]), 1) RETURN node.id;");
    ASSERT_TRUE(before->isSuccess()) << before->getErrorMessage();
    dropIndex();
    ok("COPY Doc FROM '" + docsCsv("docs2", 500, 500) + "' (header=false);");
    auto during = reader->query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', CAST([42.0,1.0,2.0,0.0] AS FLOAT[4]), 1) RETURN node.id;");
    EXPECT_TRUE(during->isSuccess()) << during->getErrorMessage();
    if (during->isSuccess() && during->hasNext()) {
        EXPECT_EQ(during->getNext()->getValue(0)->getValue<int64_t>(), 42);
    }
    ASSERT_TRUE(reader->query("COMMIT;")->isSuccess());
    reader.reset();
    createIndex();
    expectThrough(1000, 0, 777);
}

} // namespace
