// La version de format et le moteur écrivain (ticket du 18 octobre 2026, « une base ne dit pas
// quel moteur l'a écrite ») : l'en-tête de base porte la version de format
// (StorageVersionInfo::STORAGE_FORMAT_VERSION, posée à la main) et le bâti du dernier écrivain ;
// le journal commence par un enregistrement de format. Une base ou un journal plus récents que
// ce moteur sont refusés en nommant les deux versions et les deux bâtis, en lecture comme en
// écriture ; une base plus ancienne s'ouvre et son premier point de reprise l'estampille ; une
// base plus ancienne que la plus ancienne lisible est refusée en le disant.
//
// Le témoin lit et forge les octets de la page 0 et du journal lui-même, sans l'API de ce lot :
// il compile sur un moteur d'avant, et y rougit à l'exécution (pas de bâti écrivain, refus sans
// les deux bâtis, journal sans enregistrement de format).
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

#include "common/system_config.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/storage_version_info.h"
#include "test_helper/test_helper.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::storage;
using namespace rag3db::testing;

namespace {

constexpr const char* MAGIC = "RAG3DB";
constexpr uint64_t VERSION_WITH_WRITER = 41; // depuis cette version, le bâti suit la version
constexpr uint64_t TOO_OLD = 38;             // la plus ancienne lisible est 39

// Ce que la page 0 dit : la version, le bâti écrivain (vide avant 41), et où commence le reste.
struct Header {
    uint64_t version = 0;
    std::string writer;
    size_t versionAt = 0; // décalage de la version dans la page
    size_t tailAt = 0;    // décalage de ce qui suit (étendues, identifiant, pages de données)
};

uint64_t readU64(const std::string& bytes, size_t at) {
    uint64_t v = 0;
    std::memcpy(&v, bytes.data() + at, 8);
    return v;
}

std::string u64(uint64_t v) {
    return std::string(reinterpret_cast<const char*>(&v), 8);
}

Header parseHeader(const std::string& page) {
    Header h;
    // En Debug, une clé « storage_version » (chaîne préfixée) précède la version.
    const auto key = page.find("storage_version", 0);
    h.versionAt = (key != std::string::npos && key < 64) ? key + std::strlen("storage_version") :
                                                           std::strlen(MAGIC);
    h.version = readU64(page, h.versionAt);
    h.tailAt = h.versionAt + 8;
    if (h.version >= VERSION_WITH_WRITER) {
        const auto writerKey = page.find("writer", h.tailAt);
        if (writerKey != std::string::npos && writerKey < h.tailAt + 16) {
            h.tailAt = writerKey + std::strlen("writer");
        }
        const auto len = readU64(page, h.tailAt);
        if (len < 512) {
            h.writer = page.substr(h.tailAt + 8, len);
            h.tailAt += 8 + len;
        }
    }
    return h;
}

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

    std::string readPage0() const {
        std::ifstream in(databasePath, std::ios::binary);
        std::string page(RAG3DB_PAGE_SIZE, '\0');
        in.read(page.data(), static_cast<std::streamsize>(page.size()));
        return page;
    }
    Header readHeader() const { return parseHeader(readPage0()); }

    // Réécrit la page 0 sous une autre version et un autre bâti, comme un autre moteur l'aurait
    // écrite : même magie, même queue (étendues, identifiant, pages de données).
    void forgeHeader(uint64_t version, const std::string& writer) const {
        const auto page = readPage0();
        const auto h = parseHeader(page);
        std::string forged = page.substr(0, h.versionAt) + u64(version);
        if (version >= VERSION_WITH_WRITER) {
            forged += u64(writer.size()) + writer;
        }
        forged += page.substr(h.tailAt);
        forged.resize(RAG3DB_PAGE_SIZE, '\0');
        std::fstream out(databasePath, std::ios::binary | std::ios::in | std::ios::out);
        out.seekp(0);
        out.write(forged.data(), static_cast<std::streamsize>(forged.size()));
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
};

TEST_F(StorageFormatVersion, ANewHeaderCarriesTheFormatVersionAndTheWriter) {
    writeADatabase();
    const auto header = readHeader();
    EXPECT_EQ(header.version, current);
    EXPECT_GE(header.version, VERSION_WITH_WRITER) << "this engine writes a format without writer";
    EXPECT_EQ(header.writer.rfind("rag3db-build-", 0), 0u)
        << "the header does not name its writer: « " << header.writer << " »";
}

TEST_F(StorageFormatVersion, ANewerDatabaseIsRefusedWithBothVersionsAndBothBuilds) {
    writeADatabase();
    forgeHeader(current + 1, "rag3db-build-demain-Release");
    EXPECT_EQ(readHeader().writer, "rag3db-build-demain-Release");
    const auto writing = openError(false);
    expectNames(writing, {"rag3db-build-demain-Release", "format " + std::to_string(current + 1),
                             "rag3db-build-", "format " + std::to_string(current),
                             "at least as recent"});
    const auto reading = openError(true);
    EXPECT_EQ(reading, writing) << "a read-only open must be refused by the same words";
}

TEST_F(StorageFormatVersion, ATooOldDatabaseIsRefusedByName) {
    writeADatabase();
    forgeHeader(TOO_OLD, "");
    EXPECT_EQ(readHeader().version, TOO_OLD);
    const auto error = openError(false);
    expectNames(error,
        {"format " + std::to_string(TOO_OLD), "older than the oldest", "rag3db-build-", "export it"});
}

// La vraie base d'un moteur d'avant (format 39, test/transaction/database_before_extent, fabriquée
// par un moteur d'avant) : elle s'ouvre, se lit, et son premier point de reprise l'estampille à la
// version et au bâti de ce moteur.
TEST_F(StorageFormatVersion, AnOlderDatabaseOpensAndItsFirstCheckpointStampsIt) {
    const auto fixture = TestHelper::appendRag3dbRootPath(
        "test/transaction/database_before_extent/base.rag3db.gz");
    std::filesystem::remove(databasePath);
    std::filesystem::remove(databasePath + ".wal");
    ASSERT_EQ(std::system(("gzip -dc '" + fixture + "' > '" + databasePath + "'").c_str()), 0);
    const auto old = readHeader();
    EXPECT_EQ(old.version, StorageVersionInfo::VERSION_BEFORE_DATA_FILE_EXTENT);
    EXPECT_TRUE(old.writer.empty());
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
    EXPECT_EQ(stamped.version, current);
    EXPECT_EQ(stamped.writer.rfind("rag3db-build-", 0), 0u)
        << "the first checkpoint did not stamp the writer: « " << stamped.writer << " »";
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
    const auto at = wal.find(u64(current));
    ASSERT_NE(at, std::string::npos) << "the journal does not start with a format record";
    ASSERT_LT(at, 64u) << "the format record is not at the head of the journal";
    const auto forged = u64(current + 1);
    std::memcpy(wal.data() + at, forged.data(), 8);
    {
        std::ofstream out(walPath, std::ios::binary | std::ios::trunc);
        out.write(wal.data(), static_cast<std::streamsize>(wal.size()));
    }
    const auto error = openError(false);
    expectNames(error, {"journal", "rag3db-build-", "format " + std::to_string(current + 1),
                           "format " + std::to_string(current), "at least as recent"});
    // Rien n'a été rejoué ni effacé : le journal est encore là, intact.
    EXPECT_TRUE(std::filesystem::exists(walPath));
    EXPECT_EQ(std::filesystem::file_size(walPath), wal.size());
}

} // namespace
