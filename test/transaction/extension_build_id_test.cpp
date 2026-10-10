#include <algorithm>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <string>
#include <vector>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/storage_utils.h"
#include "test_helper/test_helper.h"

namespace rag3db {
namespace testing {

// Une extension d'un autre bâti que le moteur ne doit pas se charger sans un mot : ses structures
// ne sont pas celles du moteur (ticket 2026-10-05-extension-chargee-au-rejeu-sans-controle-de-bati :
// une ouverture qui ne rendait jamais la main). Le moteur et chaque extension portent un
// identifiant de bâti, comparé entre le nom et l'initialisation de l'extension.
class ExtensionBuildIdTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "needs an on-disk database: the extension is noted beside it";
        }
        if (!std::filesystem::exists(vectorExtension())) {
            GTEST_SKIP() << "the vector extension is not built";
        }
    }

    static std::string vectorExtension() {
        return TestHelper::appendRag3dbRootPath(
            "extension/vector/build/libvector.rag3db_extension");
    }

    // Une copie de l'extension vector dont l'identifiant de bâti est altéré : les octets qui
    // suivent le préfixe « rag3db-build- » sont réécrits. Si le fichier ne porte pas
    // d'identifiant (une extension d'avant le contrôle), la copie reste telle quelle : c'est alors
    // une extension sans identifiant, que le moteur doit refuser aussi.
    std::string aCopyFromAnotherBuild() {
        const auto copy = databasePath + ".extension-d-un-autre-bati";
        std::vector<char> bytes;
        {
            std::ifstream in(vectorExtension(), std::ios::binary);
            bytes.assign(std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>());
        }
        const std::string marker = "rag3db-build-";
        auto it = std::search(bytes.begin(), bytes.end(), marker.begin(), marker.end());
        if (it != bytes.end()) {
            for (auto pos = it + marker.size(); pos != bytes.end() && pos < it + marker.size() + 8;
                 ++pos) {
                *pos = *pos == 'f' ? 'e' : 'f';
            }
        }
        {
            std::ofstream out(copy, std::ios::binary | std::ios::trunc);
            out.write(bytes.data(), static_cast<std::streamsize>(bytes.size()));
        }
        return copy;
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
};

// LOAD EXTENSION d'une extension d'un autre bâti : refus nommé, qui dit les deux bâtis.
TEST_F(ExtensionBuildIdTest, AnExtensionFromAnotherBuildIsRefusedByName) {
    createDBAndConn();
    const auto copy = aCopyFromAnotherBuild();
    auto refused = conn->query("LOAD EXTENSION '" + copy + "';");
    EXPECT_FALSE(refused->isSuccess()) << "une extension d'un autre bâti s'est chargée sans un mot";
    if (!refused->isSuccess()) {
        EXPECT_NE(refused->getErrorMessage().find("was built from"), std::string::npos)
            << refused->getErrorMessage();
    }
    // L'extension de ce bâti se charge, elle.
    conn.reset();
    database.reset();
    createDBAndConn();
    ok("LOAD EXTENSION '" + vectorExtension() + "';");
    std::filesystem::remove(copy);
}

// Au rejeu, l'extension notée à côté de la base est d'un autre bâti : elle n'est pas chargée,
// l'index se dit en retard, et la raison nomme le refus.
TEST_F(ExtensionBuildIdTest, AtRecoveryAnExtensionFromAnotherBuildLeavesTheIndexBehind) {
    createDBAndConn();
    ok("LOAD EXTENSION '" + vectorExtension() + "';");
    ok("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);");
    ok("UNWIND range(0, 99) AS i CREATE (:Doc {id: i, vec: [CAST(i AS FLOAT), 1.0, 2.0, 3.0]});");
    ok("CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');");
    ok("CALL force_checkpoint_on_close=false;");
    ok("CALL auto_checkpoint=false;");
    ok("CREATE (:Doc {id: 1000, vec: [1000.0, 1.0, 2.0, 3.0]});");
    conn.reset();
    database.reset();
    ASSERT_GT(std::filesystem::file_size(storage::StorageUtils::getWALFilePath(databasePath)), 0u)
        << "le journal doit porter l'écriture dans la table indexée";

    // La liste des extensions notées désigne maintenant une copie d'un autre bâti.
    const auto copy = aCopyFromAnotherBuild();
    {
        std::ofstream noted(storage::StorageUtils::getExtensionsFilePath(databasePath),
            std::ios::trunc);
        noted << "vector\t" << copy << "\n";
    }
    createDBAndConn();
    auto search = conn->query(
        "CALL QUERY_VECTOR_INDEX('Doc', 'doc_index', [1000.0, 1.0, 2.0, 3.0], 1) RETURN node.id;");
    EXPECT_FALSE(search->isSuccess())
        << "l'index répond alors que son extension, d'un autre bâti, a été chargée au rejeu";
    if (!search->isSuccess()) {
        EXPECT_NE(search->getErrorMessage().find("was built from"), std::string::npos)
            << search->getErrorMessage();
    }
    conn.reset();
    database.reset();
    std::filesystem::remove(copy);
}

} // namespace testing
} // namespace rag3db
