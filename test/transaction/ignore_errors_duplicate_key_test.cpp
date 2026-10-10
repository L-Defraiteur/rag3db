#include <filesystem>
#include <fstream>
#include <functional>
#include <string>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

namespace rag3db {
namespace testing {

// Un COPY sous IGNORE_ERRORS qui porte deux fois une même clé doit écarter le doublon, et lui
// seul. Ticket 2026-10-10-ignore-errors-supprime-une-ligne-innocente : quand l'index
// de clé avait déjà des entrées sur disque, une autre ligne était supprimée à sa place et le
// doublon restait, sans entrée d'index. Un seul fil (PARALLEL=false) : la recette est
// déterministe.
class IgnoreErrorsDuplicateKeyTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "needs an on-disk database: the subject is the on-disk key index";
        }
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    int64_t single(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        if (!result->isSuccess() || !result->hasNext()) {
            return -1;
        }
        return result->getNext()->getValue(0)->getValue<int64_t>();
    }

    // 200 lignes 0 … 199, éventuellement passées par un point de reprise (l'index de clé sur
    // disque), puis un COPY de 1000 … 2999 qui porte « <dup>,dup » tôt dans le fichier.
    void copyWithAnEarlyDuplicate(bool keysOnDisk, const std::string& keyType, int64_t dup) {
        createDBAndConn();
        ok("CREATE NODE TABLE Doc(id " + keyType + " PRIMARY KEY, name STRING);");
        ok("UNWIND range(0, 199) AS i CREATE (:Doc {id: CAST(i AS " + keyType +
           "), name: 'old ' + CAST(i AS STRING)});");
        if (keysOnDisk) {
            ok("CHECKPOINT;");
        }
        const auto csvPath = databasePath + ".ignore_errors.csv";
        {
            std::ofstream csv(csvPath);
            for (int64_t id = 1000; id <= 1010; id++) {
                csv << id << ",name " << id << "\n";
            }
            csv << dup << ",dup\n";
            for (int64_t id = 1011; id < 3000; id++) {
                csv << id << ",name " << id << "\n";
            }
        }
        ok("COPY Doc FROM '" + csvPath + "' (IGNORE_ERRORS=true, PARALLEL=false);");
        std::filesystem::remove(csvPath);
    }

    // Chaque ligne du COPY, une fois, retrouvée par sa clé, avec son propre nom ; le doublon
    // écarté, et l'avertissement nomme sa clé.
    void expectOnlyTheDuplicateSkipped(const std::string& keyType, int64_t dup) {
        const auto key = [&](int64_t id) {
            return keyType == "STRING" ? "'" + std::to_string(id) + "'" : std::to_string(id);
        };
        EXPECT_EQ(single("MATCH (d:Doc) RETURN count(*);"), 200 + 2000);
        EXPECT_EQ(single("MATCH (d:Doc) RETURN count(DISTINCT d.id);"), 200 + 2000);
        EXPECT_EQ(single("MATCH (d:Doc) WHERE d.name = 'dup' RETURN count(*);"), 0);
        int64_t notFoundByKey = 0;
        for (int64_t id = 1000; id < 3000; id++) {
            const auto found =
                single("MATCH (d:Doc {id: " + key(id) + "}) WHERE d.name = 'name " +
                       std::to_string(id) + "' RETURN count(*);");
            notFoundByKey += found == 1 ? 0 : 1;
        }
        EXPECT_EQ(notFoundByKey, 0) << "lignes du COPY introuvables par leur clé";
        auto warnings = conn->query("CALL show_warnings() RETURN message;");
        ASSERT_TRUE(warnings->isSuccess()) << warnings->getErrorMessage();
        int64_t numWarnings = 0;
        while (warnings->hasNext()) {
            const auto message = warnings->getNext()->getValue(0)->toString();
            numWarnings++;
            EXPECT_NE(message.find(std::to_string(dup)), std::string::npos)
                << "l'avertissement ne nomme pas le doublon : " << message;
        }
        EXPECT_EQ(numWarnings, 1);
    }
};

TEST_F(IgnoreErrorsDuplicateKeyTest, KeysOnDiskSkipOnlyTheDuplicate) {
    copyWithAnEarlyDuplicate(true /* keysOnDisk */, "INT64", 1005);
    expectOnlyTheDuplicateSkipped("INT64", 1005);
}

TEST_F(IgnoreErrorsDuplicateKeyTest, KeysOnDiskSkipOnlyTheDuplicateStringKey) {
    copyWithAnEarlyDuplicate(true /* keysOnDisk */, "STRING", 1005);
    expectOnlyTheDuplicateSkipped("STRING", 1005);
}

// Le témoin de contrôle : l'index tout en mémoire écartait déjà le bon doublon.
TEST_F(IgnoreErrorsDuplicateKeyTest, KeysInMemorySkipOnlyTheDuplicate) {
    copyWithAnEarlyDuplicate(false /* keysOnDisk */, "INT64", 1005);
    expectOnlyTheDuplicateSkipped("INT64", 1005);
}

} // namespace testing
} // namespace rag3db
