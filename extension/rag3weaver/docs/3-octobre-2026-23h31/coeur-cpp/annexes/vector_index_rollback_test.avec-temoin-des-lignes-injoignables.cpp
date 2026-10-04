// L'index vectoriel après l'annulation d'une transaction qui a ajouté des lignes à sa table.
//
// L'index tient en mémoire le compte des lignes qu'il a reliées, et ses points d'entrée. Un
// COPY avance ce compte ; l'annulation retirait les lignes de la table sans le reculer. La
// recherche échouait alors (« vector::reserve » : une taille négative), puis le COPY suivant,
// trouvant « compte = nombre de lignes », ne reliait rien : ses lignes étaient dans la table,
// pas dans l'index, en silence (ticket du 5 octobre 2026). L'annulation prévient maintenant
// les index (Index::rollbackInsert).
//
// Chaque ligne porte un vecteur qui n'est qu'à elle : la recherche de son vecteur exact doit
// la rendre en tête. C'est ce qui dit qu'une ligne est dans l'index, et qu'aucune arête ne
// mène plus à une ligne annulée. Se saute si l'extension vector n'est pas bâtie.

#include <filesystem>
#include <fstream>
#include <functional>
#include <string>

#include <unistd.h>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "test_helper/test_helper.h"

using namespace rag3db::testing;

namespace {

class VectorIndexRollbackTest : public EmptyDBTest {
protected:
    static constexpr int64_t NUM_OLD_ROWS = 200;
    static constexpr int64_t FIRST_COPIED = 2000;
    static constexpr int64_t NUM_COPIED = 100;

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
        csvPath = (std::filesystem::temp_directory_path() /
                   ("vector_index_rollback_" + std::to_string(::getpid()) + ".csv"))
                      .string();
        std::ofstream csv(csvPath);
        for (int64_t id = FIRST_COPIED; id < FIRST_COPIED + NUM_COPIED; id++) {
            csv << id << ",\"" << copiedVector(id) << "\"\n";
        }
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

    static std::string oldVector(int64_t id) {
        return "[" + std::to_string(id % 17) + ".0," + std::to_string(id % 23) + ".0," +
               std::to_string(id % 29) + ".0," + std::to_string(id) + ".0]";
    }
    static std::string copiedVector(int64_t id) {
        return "[1.0,2.0,3.0," + std::to_string(id) + ".0]";
    }

    void createOldRowsAndIndex() {
        ok("UNWIND range(0, " + std::to_string(NUM_OLD_ROWS - 1) +
            ") AS i CREATE (:Doc {id: i, vec: [CAST(i % 17 AS FLOAT), CAST(i % 23 AS FLOAT), "
            "CAST(i % 29 AS FLOAT), CAST(i AS FLOAT)]});");
        ok("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    }

    // La ligne la plus proche, ou -1 si la recherche ne rend rien, -2 si elle échoue.
    int64_t nearest(const std::string& vec, std::string* error = nullptr) {
        auto result = conn->query("CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', " + vec +
                                  ", 1) RETURN node.id ORDER BY distance;");
        if (!result->isSuccess()) {
            if (error) {
                *error = result->getErrorMessage();
            }
            return -2;
        }
        return result->hasNext() ? result->getNext()->getValue(0)->getValue<int64_t>() : -1;
    }

    // Chaque ligne de [first, first + count) se retrouve en tête par son vecteur exact. Un
    // résumé borné : le nombre de lignes manquées et la première.
    void expectEachFound(int64_t first, int64_t count,
        const std::function<std::string(int64_t)>& vectorOf, const std::string& when) {
        int64_t numMissed = 0;
        std::string firstMiss;
        for (int64_t id = first; id < first + count; id++) {
            std::string error;
            const auto found = nearest(vectorOf(id), &error);
            if (found != id && numMissed++ == 0) {
                firstMiss = "row " + std::to_string(id) + " gives " + std::to_string(found) +
                            (error.empty() ? "" : " (" + error + ")");
            }
        }
        EXPECT_EQ(numMissed, 0) << when << " : " << numMissed << " of " << count
                                << " rows are not found by their own vector; first, "
                                << firstMiss;
    }

    // Aucune recherche ne rend une ligne copiée, et aucune n'échoue.
    void expectNoCopiedRowFound(const std::string& when) {
        int64_t numWrong = 0;
        std::string firstWrong;
        for (int64_t id = FIRST_COPIED; id < FIRST_COPIED + NUM_COPIED; id += 7) {
            std::string error;
            const auto found = nearest(copiedVector(id), &error);
            if ((found >= FIRST_COPIED || found == -2) && numWrong++ == 0) {
                firstWrong = "near row " + std::to_string(id) + " gives " +
                             std::to_string(found) + (error.empty() ? "" : " (" + error + ")");
            }
        }
        EXPECT_EQ(numWrong, 0) << when << " : " << firstWrong;
    }

    void reopen() {
        createDBAndConn();
        ok("LOAD EXTENSION '" + extension + "';");
    }

    std::string copyStatement() const { return "COPY Doc FROM '" + csvPath + "';"; }

    std::string extension;
    std::string csvPath;
};

// Un COPY annulé : la recherche marche, rend les lignes anciennes — chacune par son vecteur
// exact : les arêtes de retour vers les lignes annulées n'ont rien abîmé — et jamais une
// ligne annulée. De même après un point de reprise et une réouverture.
TEST_F(VectorIndexRollbackTest, ARolledBackCopyLeavesTheIndexAsItWas) {
    createOldRowsAndIndex();
    ok("BEGIN TRANSACTION;");
    ok(copyStatement());
    ok("ROLLBACK;");
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), NUM_OLD_ROWS);
    expectEachFound(0, NUM_OLD_ROWS, oldVector, "after the rollback");
    expectNoCopiedRowFound("after the rollback");
    ok("CHECKPOINT;");
    expectEachFound(0, NUM_OLD_ROWS, oldVector, "after the checkpoint");
    reopen();
    expectEachFound(0, NUM_OLD_ROWS, oldVector, "after the reopening");
    expectNoCopiedRowFound("after the reopening");
}

// Un COPY annulé, puis le même validé : chaque ligne copiée est dans l'index.
TEST_F(VectorIndexRollbackTest, TheSameCopyAfterARolledBackOneIsIndexed) {
    createOldRowsAndIndex();
    ok("BEGIN TRANSACTION;");
    ok(copyStatement());
    ok("ROLLBACK;");
    ok(copyStatement());
    EXPECT_EQ(single("MATCH (n:Doc) RETURN count(*);"), NUM_OLD_ROWS + NUM_COPIED);
    expectEachFound(FIRST_COPIED, NUM_COPIED, copiedVector, "after the committed copy");
    expectEachFound(0, NUM_OLD_ROWS, oldVector, "after the committed copy");
    ok("CHECKPOINT;");
    reopen();
    expectEachFound(FIRST_COPIED, NUM_COPIED, copiedVector, "after the reopening");
    expectEachFound(0, NUM_OLD_ROWS, oldVector, "after the reopening");
}

// Sur une table vide : les points d'entrée du graphe étaient des lignes annulées.
TEST_F(VectorIndexRollbackTest, ARolledBackCopyIntoAnEmptyTable) {
    ok("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    ok("BEGIN TRANSACTION;");
    ok(copyStatement());
    ok("ROLLBACK;");
    std::string error;
    EXPECT_EQ(nearest(copiedVector(FIRST_COPIED + 50), &error), -1) << error;
    ok(copyStatement());
    expectEachFound(FIRST_COPIED, NUM_COPIED, copiedVector, "after the committed copy");
    ok("CHECKPOINT;");
    reopen();
    expectEachFound(FIRST_COPIED, NUM_COPIED, copiedVector, "after the reopening");
}

// Des insertions ordinaires puis un COPY, annulés : les lignes versées dans la table pour le
// COPY avaient nourri l'index. Puis la même suite validée.
TEST_F(VectorIndexRollbackTest, InsertsThenACopyRolledBackThenCommitted) {
    createOldRowsAndIndex();
    const auto insertedVector = [](int64_t id) {
        return "[50.0,50.0,50.0," + std::to_string(4000 + id) + ".0]";
    };
    const auto writes = [&] {
        ok("BEGIN TRANSACTION;");
        ok("UNWIND range(1000, 1004) AS i CREATE (:Doc {id: i, vec: [50.0, 50.0, 50.0, "
           "CAST(4000 + i AS FLOAT)]});");
        ok(copyStatement());
    };
    writes();
    ok("ROLLBACK;");
    expectEachFound(0, NUM_OLD_ROWS, oldVector, "after the rollback");
    expectNoCopiedRowFound("after the rollback");
    std::string error;
    EXPECT_LT(nearest(insertedVector(1002), &error), NUM_OLD_ROWS) << error;
    writes();
    ok("COMMIT;");
    expectEachFound(1000, 5, insertedVector, "after the commit");
    expectEachFound(FIRST_COPIED, NUM_COPIED, copiedVector, "after the commit");
    expectEachFound(0, NUM_OLD_ROWS, oldVector, "after the commit");
    ok("CHECKPOINT;");
    reopen();
    expectEachFound(1000, 5, insertedVector, "after the reopening");
    expectEachFound(FIRST_COPIED, NUM_COPIED, copiedVector, "after the reopening");
}

// La mise à jour d'un vecteur, annulée : la ligne se retrouve par son ancien vecteur, les
// autres aussi.
TEST_F(VectorIndexRollbackTest, ARolledBackVectorUpdate) {
    createOldRowsAndIndex();
    ok("BEGIN TRANSACTION;");
    ok("MATCH (n:Doc) WHERE n.id IN [0, 5, 77, 199] SET n.vec = [900.0, 900.0, 900.0, "
       "CAST(9000 + n.id AS FLOAT)];");
    ok("ROLLBACK;");
    expectEachFound(0, NUM_OLD_ROWS, oldVector, "after the rollback");
    std::string error;
    EXPECT_LT(nearest("[900.0,900.0,900.0,9005.0]", &error), NUM_OLD_ROWS) << error;
    ok("CHECKPOINT;");
    reopen();
    expectEachFound(0, NUM_OLD_ROWS, oldVector, "after the reopening");
}

// La reprise de rag3weaver après un paquet défait (sonde de la session de l'arbre principal,
// 5 octobre 2026) : un COPY annulé, la base fermée et rouverte, puis dans une transaction des
// lignes neuves créées sans vecteur et leur vecteur posé par SET avant la validation. Les
// lignes neuves prennent les décalages des lignes annulées.
TEST_F(VectorIndexRollbackTest, NewRowsGivenTheirVectorBySetAfterARolledBackCopyAndAReopening) {
    createOldRowsAndIndex();
    ok("BEGIN TRANSACTION;");
    ok(copyStatement());
    ok("ROLLBACK;");
    reopen();
    const auto newVector = [](int64_t id) {
        return "[60.0,60.0,60.0," + std::to_string(id) + ".0]";
    };
    ok("BEGIN TRANSACTION;");
    ok("UNWIND range(5000, 5031) AS i MERGE (n:Doc {id: i});");
    ok("UNWIND range(5000, 5031) AS i MATCH (n:Doc {id: i}) SET n.vec = [60.0, 60.0, 60.0, "
       "CAST(i AS FLOAT)];");
    ok("COMMIT;");
    expectEachFound(5000, 32, newVector, "after the commit");
    expectEachFound(0, NUM_OLD_ROWS, oldVector, "after the commit");
    ok("CHECKPOINT;");
    reopen();
    expectEachFound(5000, 32, newVector, "after the reopening");
}

// La même sans COPY annulé : le SET du vecteur d'une ligne créée dans la transaction.
TEST_F(VectorIndexRollbackTest, NewRowsGivenTheirVectorBySetInTheirTransaction) {
    createOldRowsAndIndex();
    reopen();
    const auto newVector = [](int64_t id) {
        return "[60.0,60.0,60.0," + std::to_string(id) + ".0]";
    };
    ok("BEGIN TRANSACTION;");
    ok("UNWIND range(5000, 5031) AS i MERGE (n:Doc {id: i});");
    ok("UNWIND range(5000, 5031) AS i MATCH (n:Doc {id: i}) SET n.vec = [60.0, 60.0, 60.0, "
       "CAST(i AS FLOAT)];");
    ok("COMMIT;");
    expectEachFound(5000, 32, newVector, "after the commit");
    expectEachFound(0, NUM_OLD_ROWS, oldVector, "after the commit");
}

// La forme de la reprise du produit (témoin du banc, ProductReloadRecovery, 5 octobre 2026),
// dans un seul processus pour voir où les lignes se perdent : un index cosinus de 64
// dimensions posé sur la table vide ; quatre paquets de 32 lignes par COPY journalisé, validés
// sans point de reprise — leurs arêtes sont donc en mémoire ; puis trois paquets copiés et
// annulés. Chaque ligne validée doit sortir en tête pour son vecteur exact à chaque étape.
class VectorIndexRollbackOfUncheckpointedRowsTest : public VectorIndexRollbackTest {
protected:
    static constexpr int64_t BATCH = 32;
    static constexpr int DIMENSIONS = 64;

    static std::string wideVector(int64_t id) {
        std::string text = "[";
        for (auto j = 0; j < DIMENSIONS; j++) {
            auto value = static_cast<double>((id * 31 + j * 7) % 97) / 97.0;
            if (j == id % DIMENSIONS) {
                value += 2.0;
            }
            text += (j ? "," : "") + std::to_string(value);
        }
        return text + "]";
    }

    std::string batchPath(int64_t batch) const {
        return csvPath + ".batch" + std::to_string(batch) + ".csv";
    }

    void prepare() {
        ok("CREATE NODE TABLE Chunk(id INT64 PRIMARY KEY, emb FLOAT[64]);");
        ok("CALL CREATE_VECTOR_INDEX('Chunk', 'chunk_index', 'emb', mu := 30, ml := 60, pu := "
           "0.05, metric := 'cosine', alpha := 1.1, efc := 200);");
        for (auto batch = 0; batch < 7; batch++) {
            std::ofstream csv(batchPath(batch));
            for (auto id = batch * BATCH; id < (batch + 1) * BATCH; id++) {
                csv << id << ",\"" << wideVector(id) << "\"\n";
            }
        }
        ok("CALL auto_checkpoint=false;");
        ok("CALL force_checkpoint_on_copy=false;");
    }

    void TearDown() override {
        for (auto batch = 0; batch < 7; batch++) {
            std::filesystem::remove(batchPath(batch));
        }
        VectorIndexRollbackTest::TearDown();
    }

    void copyBatch(int64_t batch) { ok("COPY Chunk FROM '" + batchPath(batch) + "';"); }

    // Le nombre de lignes de [0, count) qui ne sortent pas en tête pour leur vecteur exact.
    int64_t missed(int64_t count, std::string* first) {
        int64_t numMissed = 0;
        for (int64_t id = 0; id < count; id++) {
            auto result = conn->query("CALL QUERY_VECTOR_INDEX('Chunk', 'chunk_index', " +
                                      wideVector(id) +
                                      ", 3, efs := 200) RETURN node.id ORDER BY distance;");
            const int64_t found =
                !result->isSuccess() ? -2 :
                result->hasNext() ? result->getNext()->getValue(0)->getValue<int64_t>() : -1;
            if (found != id && numMissed++ == 0) {
                *first = "row " + std::to_string(id) + " gives " + std::to_string(found) +
                         (result->isSuccess() ? "" : " (" + result->getErrorMessage() + ")");
            }
        }
        return numMissed;
    }

    void expectAllFound(const std::string& when) {
        std::string first;
        const auto numMissed = missed(4 * BATCH, &first);
        EXPECT_EQ(numMissed, 0) << when << " : " << numMissed << " of " << 4 * BATCH
                                << " committed rows are not found; first, " << first;
    }
};

TEST_F(VectorIndexRollbackOfUncheckpointedRowsTest, CommittedRowsSurviveRolledBackCopies) {
    prepare();
    for (auto batch = 0; batch < 4; batch++) {
        ok("BEGIN TRANSACTION;");
        copyBatch(batch);
        ok("COMMIT;");
    }
    expectAllFound("before the rolled back copies");
    ok("BEGIN TRANSACTION;");
    for (auto batch = 4; batch < 7; batch++) {
        copyBatch(batch);
    }
    expectAllFound("inside the transaction of the copies");
    ok("ROLLBACK;");
    expectAllFound("after the rollback");
    ok("CHECKPOINT;");
    expectAllFound("after the checkpoint");
    conn.reset();
    database.reset();
    createDBAndConn();
    ok("LOAD EXTENSION '" + extension + "';");
    expectAllFound("after the reopening");
}

} // namespace
