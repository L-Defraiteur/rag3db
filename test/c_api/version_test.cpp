#include <filesystem>
#include <fstream>

#include "c_api/rag3db.h"
#include "c_api_test/c_api_test.h"
#include "gtest/gtest.h"

using namespace rag3db::main;
using namespace rag3db::testing;
using namespace rag3db::common;

class CApiVersionTest : public CApiTest {
public:
    std::string getInputDir() override {
        return TestHelper::appendRag3dbRootPath("dataset/tinysnb/");
    }

    void TearDown() override { APIDBTest::TearDown(); }
};

class EmptyCApiVersionTest : public CApiVersionTest {
public:
    std::string getInputDir() override { return "empty"; }
};

TEST_F(EmptyCApiVersionTest, GetVersion) {
    rag3db_connection_destroy(&connection);
    rag3db_database_destroy(&_database);
    auto version = rag3db_get_version();
    ASSERT_NE(version, nullptr);
    ASSERT_STREQ(version, RAG3DB_CMAKE_VERSION);
    rag3db_destroy_string(version);
}

TEST_F(CApiVersionTest, GetStorageVersion) {
    auto storageVersion = rag3db_get_storage_version();
    if (inMemMode) {
        GTEST_SKIP();
    }
    // Reset the database to ensure that the lock on db file is released.
    rag3db_connection_destroy(&connection);
    rag3db_database_destroy(&_database);
    auto data = std::filesystem::path(databasePath);
    std::ifstream dbFile;
    dbFile.open(data, std::ios::binary);
    ASSERT_TRUE(dbFile.is_open());
    char magic[5];
    dbFile.read(magic, 4);
    magic[4] = '\0';
    ASSERT_STREQ(magic, "RAG3DB");
    uint64_t actualVersion;
    dbFile.read(reinterpret_cast<char*>(&actualVersion), sizeof(actualVersion));
    dbFile.close();
    ASSERT_EQ(storageVersion, actualVersion);
}

TEST_F(EmptyCApiVersionTest, GetStorageVersion) {
    auto storageVersion = rag3db_get_storage_version();
    if (inMemMode) {
        GTEST_SKIP();
    }
    // Reset the database to ensure that the lock on db file is released.
    rag3db_connection_destroy(&connection);
    rag3db_database_destroy(&_database);
    auto data = std::filesystem::path(databasePath);
    std::ifstream dbFile;
    dbFile.open(data, std::ios::binary);
    ASSERT_TRUE(dbFile.is_open());
    char magic[5];
    dbFile.read(magic, 4);
    magic[4] = '\0';
    ASSERT_STREQ(magic, "RAG3DB");
    uint64_t actualVersion;
    dbFile.read(reinterpret_cast<char*>(&actualVersion), sizeof(actualVersion));
    dbFile.close();
    ASSERT_EQ(storageVersion, actualVersion);
}
