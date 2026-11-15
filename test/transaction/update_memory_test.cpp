// Une mise à jour garde un tampon à la taille de ce qu'elle a changé (ticket
// docs/coeur-cpp/tickets/2026-11-13-sous-vingt-ecrivains-le-pool-sature-et-ne-se-vide-plus.md).
// Chaque transaction qui met à jour une ligne déjà écrite crée une version de son vecteur de 2 048
// lignes (VectorUpdateInfo) ; son tampon était créé à pleine capacité, 2 048 lignes, pour une seule
// changée, et gardé jusqu'au point de reprise : 8,5 Ko de pool pour une chaîne, 25 Ko pour un
// FLOAT[64]. Sous vingt écrivains, le pool de 2 Gio était plein bien avant que le journal déclenche
// un point de reprise.
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "storage/buffer_manager/buffer_manager.h"
#include "storage/buffer_manager/memory_manager.h"

using namespace rag3db::main;

namespace rag3db {
namespace testing {

namespace {

std::string vectorLiteral(double x) {
    std::string out = "[";
    for (int i = 0; i < 64; i++) {
        out += (i ? ", " : "") + std::to_string(x);
    }
    return out + "]";
}

} // namespace

class UpdateMemory : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        run("CALL auto_checkpoint=false;");
        run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, titre STRING, vec FLOAT[64]);");
        run("COPY Doc FROM (UNWIND range(0, 49999) AS i RETURN i, 'titre ' + CAST(i AS STRING), " +
            vectorLiteral(0.1) + ");");
        ASSERT_EQ(one("MATCH (d:Doc) RETURN count(*);"), "50000\n") << "[check: setup]";
        run("CHECKPOINT;");
    }
    void run(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    std::string one(const std::string& query) {
        auto result = conn->query(query);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        return result->hasNext() ? result->getNext()->toString() : "AUCUNE";
    }
    uint64_t usedMemory() const {
        return database->getMemoryManager()->getBufferManager()->getUsedMemory();
    }
    // Le pool gardé par mise à jour d'une ligne, chacune dans sa transaction, sur des lignes
    // réparties dans les 25 vecteurs de la table.
    uint64_t bytesPerUpdate(const std::string& set) {
        const auto before = usedMemory();
        constexpr int NUM_UPDATES = 1000;
        for (int i = 0; i < NUM_UPDATES; i++) {
            run("MATCH (d:Doc {id: " + std::to_string((i * 7919) % 50000) + "}) SET " + set + ";");
        }
        return (usedMemory() - before) / NUM_UPDATES;
    }
};

TEST_F(UpdateMemory, AVectorUpdateKeepsABufferOfItsSize) {
    const auto perUpdate = bytesPerUpdate("d.vec = " + vectorLiteral(0.5));
    // 256 octets de donnée par ligne : la borne laisse place à l'en-tête de la version.
    EXPECT_LT(perUpdate, 4096u) << "[check: an-update-keeps-a-buffer-of-its-size] FLOAT[64] : "
                                << perUpdate << " octets de pool par mise à jour d'une ligne";
    EXPECT_EQ(one("MATCH (d:Doc {id: 7919}) RETURN d.vec[1];"), "0.500000\n");
}

TEST_F(UpdateMemory, AStringUpdateKeepsABufferOfItsSize) {
    const auto perUpdate = bytesPerUpdate("d.titre = 'réécrite'");
    EXPECT_LT(perUpdate, 1024u) << "[check: an-update-keeps-a-buffer-of-its-size] STRING : "
                                << perUpdate << " octets de pool par mise à jour d'une ligne";
    EXPECT_EQ(one("MATCH (d:Doc {id: 7919}) RETURN d.titre;"), "réécrite\n");
}

// Une transaction qui met à jour un vecteur entier (2 048 lignes) agrandit le tampon de sa version
// jusqu'à la pleine capacité : chaque valeur se relit, avant et après un point de reprise et une
// réouverture.
TEST_F(UpdateMemory, AnUpdateOfAWholeVectorReadsBack) {
    run("BEGIN TRANSACTION;");
    run("MATCH (d:Doc) WHERE d.id >= 2048 AND d.id < 4096 SET d.titre = 'v' + CAST(d.id AS "
        "STRING), d.vec = " +
        vectorLiteral(0.75) + ";");
    EXPECT_EQ(one("MATCH (d:Doc) WHERE d.id >= 2048 AND d.id < 4096 AND d.titre = 'v' + CAST(d.id "
                  "AS STRING) AND d.vec[64] = 0.75 RETURN count(*);"),
        "2048\n")
        << "[check: a-whole-vector-update-reads-back] in its transaction";
    run("COMMIT;");
    EXPECT_EQ(one("MATCH (d:Doc) WHERE d.id >= 2048 AND d.id < 4096 AND d.titre = 'v' + CAST(d.id "
                  "AS STRING) AND d.vec[64] = 0.75 RETURN count(*);"),
        "2048\n")
        << "[check: a-whole-vector-update-reads-back] after its commit";
    run("CHECKPOINT;");
    createDBAndConn();
    EXPECT_EQ(one("MATCH (d:Doc) WHERE d.id >= 2048 AND d.id < 4096 AND d.titre = 'v' + CAST(d.id "
                  "AS STRING) AND d.vec[64] = 0.75 RETURN count(*);"),
        "2048\n")
        << "[check: a-whole-vector-update-reads-back] after a reopening";
    EXPECT_EQ(one("MATCH (d:Doc {id: 2047}) RETURN d.titre;"), "titre 2047\n");
}

} // namespace testing
} // namespace rag3db
