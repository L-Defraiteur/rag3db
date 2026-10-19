// La version de format et le moteur écrivain (ticket du 18 octobre 2026, « une base ne dit pas
// quel moteur l'a écrite ») : l'en-tête de base porte la version de format
// (StorageVersionInfo::STORAGE_FORMAT_VERSION, posée à la main) et le bâti du dernier écrivain ;
// le journal commence par un enregistrement de format. Une base ou un journal plus récents que
// ce moteur sont refusés en nommant les deux versions et les deux bâtis, en lecture comme en
// écriture ; une base plus ancienne s'ouvre et son premier point de reprise l'estampille ; une
// base plus ancienne que la plus ancienne lisible est refusée en le disant.
#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <string>

#include "common/file_system/local_file_system.h"
#include "common/serializer/buffered_file.h"
#include "common/serializer/serializer.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/database_header.h"
#include "storage/storage_version_info.h"
#include "test_helper/test_helper.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::storage;
using namespace rag3db::testing;

namespace {

class StorageFormatVersion : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed: the subject is its header";
        }
    }

    // Une base écrite, fermée, avec un point de reprise : son en-tête est celui de ce moteur.
    void writeADatabase() {
        Database database(databasePath, *systemConfig);
        Connection connection(&database);
        ok(connection, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, titre STRING);");
        ok(connection, "CREATE (:Doc {id: 1, titre: 'a'});");
        ok(connection, "CHECKPOINT;");
    }

    static void ok(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    DatabaseHeader readHeader() const {
        LocalFileSystem fs{""};
        auto file = fs.openFile(databasePath, FileOpenFlags(FileFlags::READ_ONLY));
        auto header = DatabaseHeader::readDatabaseHeader(*file);
        EXPECT_TRUE(header.has_value());
        return header.value_or(DatabaseHeader{});
    }

    // Réécrit l'en-tête (page 0) sous une autre version et un autre bâti, comme un autre moteur
    // l'aurait écrit.
    void forgeHeader(uint64_t version, const std::string& writer) const {
        const auto header = readHeader();
        LocalFileSystem fs{""};
        auto file = fs.openFile(databasePath, FileOpenFlags(FileFlags::READ_ONLY | FileFlags::WRITE));
        auto writerPtr = std::make_shared<BufferedFileWriter>(*file);
        writerPtr->setFileOffset(0);
        Serializer serializer(writerPtr);
        header.serializeAs(serializer, version, writer);
        writerPtr->flush();
        writerPtr->sync();
    }

    // L'erreur d'une ouverture, vide si elle passe.
    std::string openError(bool readOnly) const {
        auto config = *systemConfig;
        config.readOnly = readOnly;
        try {
            Database database(databasePath, config);
            Connection connection(&database);
            auto result = connection.query("MATCH (d:Doc) RETURN count(*);");
            if (!result->isSuccess()) {
                return result->getErrorMessage();
            }
            return "";
        } catch (const std::exception& e) {
            return e.what();
        }
    }

    static void expectNames(const std::string& error, const std::vector<std::string>& words) {
        for (auto& word : words) {
            EXPECT_NE(error.find(word), std::string::npos)
                << "the refusal does not say « " << word << " » : " << error;
        }
    }

    const uint64_t current = StorageVersionInfo::getStorageVersion();
    const std::string build = StorageVersionInfo::getBuildID();
};

TEST_F(StorageFormatVersion, ANewHeaderCarriesTheFormatVersionAndTheWriter) {
    writeADatabase();
    const auto header = readHeader();
    EXPECT_EQ(header.storageVersion, current);
    EXPECT_EQ(header.writerBuildID, build);
}

TEST_F(StorageFormatVersion, ANewerDatabaseIsRefusedWithBothVersionsAndBothBuilds) {
    writeADatabase();
    forgeHeader(current + 1, "rag3db-build-demain-Release");
    const auto writing = openError(false);
    expectNames(writing, {"rag3db-build-demain-Release", "format " + std::to_string(current + 1),
                             build, "format " + std::to_string(current), "at least as recent"});
    const auto reading = openError(true);
    EXPECT_EQ(reading, writing) << "a read-only open must be refused by the same words";
}

TEST_F(StorageFormatVersion, ATooOldDatabaseIsRefusedByName) {
    writeADatabase();
    forgeHeader(StorageVersionInfo::OLDEST_READABLE_STORAGE_VERSION - 1, "");
    const auto error = openError(false);
    expectNames(error,
        {"format " + std::to_string(StorageVersionInfo::OLDEST_READABLE_STORAGE_VERSION - 1),
            "older than the oldest", build, "export it"});
}

// La vraie base d'un moteur d'avant (format 39, test/transaction/database_before_extent, fabriquée
// par un moteur d'avant) : elle s'ouvre, se lit, et son premier point de reprise l'estampille à la
// version et au bâti de ce moteur. Un en-tête forgé ne suffirait pas : depuis le format 42 les
// blocs eux-mêmes ont changé de forme, et c'est la version lue qui dit comment les relire.
TEST_F(StorageFormatVersion, AnOlderDatabaseOpensAndItsFirstCheckpointStampsIt) {
    const auto fixture = TestHelper::appendRag3dbRootPath(
        "test/transaction/database_before_extent/base.rag3db.gz");
    std::filesystem::remove(databasePath);
    std::filesystem::remove(databasePath + ".wal");
    ASSERT_EQ(std::system(("gzip -dc '" + fixture + "' > '" + databasePath + "'").c_str()), 0);
    const auto old = readHeader();
    EXPECT_EQ(old.storageVersion, StorageVersionInfo::VERSION_BEFORE_DATA_FILE_EXTENT);
    EXPECT_TRUE(old.writerBuildID.empty());
    {
        Database database(databasePath, *systemConfig);
        Connection connection(&database);
        auto count = connection.query("MATCH (d:Doc) RETURN count(*);");
        ASSERT_TRUE(count->isSuccess()) << count->getErrorMessage();
        EXPECT_EQ(count->getNext()->getValue(0)->getValue<int64_t>(), 501);
        ok(connection, "CREATE (:Doc {id: 1001, name: 'by the new engine'});");
        ok(connection, "CHECKPOINT;");
    }
    const auto stamped = readHeader();
    EXPECT_EQ(stamped.storageVersion, current);
    EXPECT_EQ(stamped.writerBuildID, build);
    Database database(databasePath, *systemConfig);
    Connection connection(&database);
    auto count = connection.query("MATCH (d:Doc) RETURN count(*);");
    ASSERT_TRUE(count->isSuccess()) << count->getErrorMessage();
    EXPECT_EQ(count->getNext()->getValue(0)->getValue<int64_t>(), 502);
}

TEST_F(StorageFormatVersion, AJournalFromANewerEngineIsRefusedBeforeAnyReplay) {
    // Sans sommes de contrôle : le témoin réécrit un octet du journal.
    systemConfig->enableChecksums = false;
    {
        Database database(databasePath, *systemConfig);
        Connection connection(&database);
        ok(connection, "CALL auto_checkpoint=false;");
        ok(connection, "CALL force_checkpoint_on_close=false;");
        ok(connection, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, titre STRING);");
        ok(connection, "CREATE (:Doc {id: 1, titre: 'a'});");
    }
    const auto walPath = databasePath + ".wal";
    ASSERT_TRUE(std::filesystem::exists(walPath)) << "no journal left to forge";
    std::string wal;
    {
        std::ifstream in(walPath, std::ios::binary);
        wal.assign(std::istreambuf_iterator<char>(in), {});
    }
    // Le premier enregistrement est celui du format : sa version (8 octets, petit-boutiste) est
    // le premier entier du journal après l'identifiant de base ; elle devient current + 1.
    uint64_t versionBytes = current;
    const auto at = wal.find(std::string(reinterpret_cast<const char*>(&versionBytes), 8));
    ASSERT_NE(at, std::string::npos) << "the journal does not start with a format record";
    ASSERT_LT(at, 64u) << "the format record is not at the head of the journal";
    uint64_t forged = current + 1;
    std::memcpy(wal.data() + at, &forged, 8);
    {
        std::ofstream out(walPath, std::ios::binary | std::ios::trunc);
        out.write(wal.data(), static_cast<std::streamsize>(wal.size()));
    }
    const auto error = openError(false);
    expectNames(error, {"journal", build, "format " + std::to_string(current + 1),
                           "format " + std::to_string(current), "at least as recent"});
    // Rien n'a été rejoué ni effacé : le journal est encore là, intact.
    EXPECT_TRUE(std::filesystem::exists(walPath));
    EXPECT_EQ(std::filesystem::file_size(walPath), wal.size());
}

} // namespace
