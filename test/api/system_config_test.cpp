#ifndef _WIN32
#include <dlfcn.h>
#endif

#include "api_test/api_test.h"
#include "common/exception/buffer_manager.h"
#include "common/system_config.h"

using namespace rag3db::common;
using namespace rag3db::testing;
using namespace rag3db::main;

class SystemConfigTest : public ApiTest {
    void SetUp() override { BaseGraphTest::SetUp(); }
};

void assertQuery(QueryResult& result) {
    auto a = result.toString();
    ASSERT_TRUE(result.isSuccess()) << result.toString();
}

TEST_F(SystemConfigTest, testAccessMode) {
    systemConfig->readOnly = false;
    auto db = std::make_unique<Database>(databasePath, *systemConfig);
    auto con = std::make_unique<Connection>(db.get());
    assertQuery(
        *con->query("CREATE NODE TABLE Person1(name STRING, age INT64, PRIMARY KEY(name))"));
    assertQuery(*con->query("CREATE (:Person1 {name: 'Alice', age: 25})"));
    assertQuery(*con->query("MATCH (:Person1) RETURN COUNT(*)"));
    db.reset();
    systemConfig->readOnly = true;
    if (databasePath == "" || databasePath == ":memory:") {
        EXPECT_THROW(auto db2 = std::make_unique<Database>("", *systemConfig), Exception);
        EXPECT_THROW(auto db2 = std::make_unique<Database>(":memory:", *systemConfig), Exception);
        return;
    }
    std::unique_ptr<Database> db2;
    std::unique_ptr<Connection> con2;
    EXPECT_NO_THROW(db2 = std::make_unique<Database>(databasePath, *systemConfig));
    EXPECT_NO_THROW(con2 = std::make_unique<Connection>(db2.get()));
    ASSERT_FALSE(con2->query("DROP TABLE Person")->isSuccess());
    EXPECT_NO_THROW(con2->query("MATCH (:Person) RETURN COUNT(*)"));
}

TEST_F(SystemConfigTest, testSpillToDisk) {
    systemConfig->readOnly = false;
    auto db = std::make_unique<Database>(databasePath, *systemConfig);
    auto con = std::make_unique<Connection>(db.get());
    if (databasePath == "" || databasePath == ":memory:") {
        auto res = con->query("CALL spill_to_disk=true;");
        ASSERT_FALSE(res->isSuccess());
        ASSERT_EQ(res->toString(),
            "Runtime exception: Cannot set spill_to_disk to true for an in-memory database!");
    } else {
        ASSERT_TRUE(con->query("CALL spill_to_disk=true;"));
        ASSERT_TRUE(con->query("CALL spill_to_disk=false;"));
    }
    db.reset();
    systemConfig->readOnly = true;
    if (databasePath == "" || databasePath == ":memory:") {
        EXPECT_THROW(auto db2 = std::make_unique<Database>("", *systemConfig), Exception);
        EXPECT_THROW(auto db2 = std::make_unique<Database>(":memory:", *systemConfig), Exception);
        return;
    }
    std::unique_ptr<Database> db2;
    std::unique_ptr<Connection> con2;
    EXPECT_NO_THROW(db2 = std::make_unique<Database>(databasePath, *systemConfig));
    EXPECT_NO_THROW(con2 = std::make_unique<Connection>(db2.get()));
    auto result = con2->query("CALL spill_to_disk=true;");
    ASSERT_FALSE(result->isSuccess());
    ASSERT_EQ(result->toString(),
        "Runtime exception: Cannot set spill_to_disk to true for a read only database!");
}

TEST_F(SystemConfigTest, testMaxDBSize) {
    systemConfig->maxDBSize = 1024;
    try {
        auto db = std::make_unique<Database>(databasePath, *systemConfig);
    } catch (const BufferManagerException& e) {
        ASSERT_EQ(std::string(e.what()),
            "Buffer manager exception: The given max db size should be at least " +
                std::to_string(2 * RAG3DB_PAGE_SIZE * StorageConstants::PAGE_GROUP_SIZE) + " bytes.");
    }
    systemConfig->maxDBSize = 2 * RAG3DB_PAGE_SIZE * StorageConstants::PAGE_GROUP_SIZE + 1;
    try {
        auto db = std::make_unique<Database>(databasePath, *systemConfig);
    } catch (const BufferManagerException& e) {
        ASSERT_EQ(std::string(e.what()),
            "Buffer manager exception: The given max db size should be a power of 2.");
    }
    systemConfig->maxDBSize = 2 * RAG3DB_PAGE_SIZE * StorageConstants::PAGE_GROUP_SIZE;
    try {
        auto db = std::make_unique<Database>(databasePath, *systemConfig);
    } catch (const BufferManagerException& e) {
        ASSERT_EQ(std::string(e.what()),
            "Buffer manager exception: No more frame groups can be added to the allocator.");
    }
}

TEST_F(SystemConfigTest, testBufferPoolSize) {
    systemConfig->bufferPoolSize = 1024;
    try {
        auto db = std::make_unique<Database>(databasePath, *systemConfig);
    } catch (const BufferManagerException& e) {
        ASSERT_EQ(std::string(e.what()),
            "Buffer manager exception: The given buffer pool size should be at least " +
                std::to_string(RAG3DB_PAGE_SIZE) + " bytes.");
    }
    systemConfig->bufferPoolSize = TestHelper::DEFAULT_BUFFER_POOL_SIZE_FOR_TESTING;
    EXPECT_NO_THROW(auto db = std::make_unique<Database>(databasePath, *systemConfig));
}

// La signature d'avant B2 du constructeur de SystemConfig reste, en surcharge : un binaire lié à la
// lib d'avant (l'hôte de luciepc a bouclé sur « undefined symbol …SystemConfigC1Emmbbmbmbbb » le
// 11 octobre 2026) continue de se lier, et reçoit l'attente au fichier par défaut.
TEST_F(SystemConfigTest, TheOldConstructorStillLinksAndGivesTheDefaultFileLockWait) {
    const SystemConfig old(1024ull * 1024 * 1024, 0, true, false, uint64_t(-1), true, 16777216,
        true, true, true);
    EXPECT_EQ(old.fileLockWaitMs, SystemConfig::DEFAULT_FILE_LOCK_WAIT_MS)
        << "[check: old-signature-default-wait] ";
    const SystemConfig fresh(1024ull * 1024 * 1024, 0, true, false, uint64_t(-1), true, 16777216,
        true, true, true, 0);
    EXPECT_EQ(fresh.fileLockWaitMs, 0u) << "[check: full-signature-wait] ";
#ifndef _WIN32
    // Le symbole exporté d'avant, tel que le binaire d'avant le cherche (Itanium).
    EXPECT_NE(dlsym(RTLD_DEFAULT, "_ZN6rag3db4main12SystemConfigC1Emmbbmbmbbb"), nullptr)
        << "[check: old-symbol-exported] ";
#endif
}
