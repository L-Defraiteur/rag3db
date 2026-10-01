#include <fstream>
#include <map>

#include "api_test/api_test.h"
#include "common/exception/runtime.h"
#include "common/exception/storage.h"
#include "gmock/gmock.h"
#include "storage/storage_utils.h"

using namespace rag3db::common;
using namespace rag3db::testing;
using namespace rag3db::transaction;

class WalTest : public ApiTest {
protected:
    void SetUp() override {
        ApiTest::SetUp();
        // Most of these tests are checking if partial recovery works correctly
        systemConfig->throwOnWalReplayFailure = false;
    }

    void testStrayWALFile(const std::function<void()>& setupNewDBFunc);
    void setupChecksumMismatchTest(std::function<void(std::ofstream&)> corruptFunc);
};

TEST_F(WalTest, NoWALFile) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("BEGIN TRANSACTION;");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("COMMIT;");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 0);
    std::filesystem::remove(walFilePath);
    // No WAL file, so no replay.
    createDBAndConn();
    auto res = conn->query("CALL show_tables() WHERE name='test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 0);
}

TEST_F(WalTest, EmptyWALFile) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("BEGIN TRANSACTION;");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("COMMIT;");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 0);
    std::filesystem::resize_file(walFilePath, 0);
    // Empty WAL file, so no replay.
    createDBAndConn();
    auto res = conn->query("CALL show_tables() WHERE name='test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 0);
}

TEST_F(WalTest, NoWALAfterCheckpoint) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("BEGIN TRANSACTION;");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("COMMIT;");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 0);
    // Checkpoint should remove the WAL file.
    conn->query("checkpoint;");
    ASSERT_FALSE(std::filesystem::exists(walFilePath));
}

TEST_F(WalTest, ShadowFileExistsWithoutWAL) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("BEGIN TRANSACTION;");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("COMMIT;");
    auto shadowFilePath = rag3db::storage::StorageUtils::getShadowFilePath(databasePath);
    // Create a shadow file that is corrupted.
    std::ofstream file(shadowFilePath);
    file << "This is not a valid Rag3db database file.";
    file.close();
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 0);
    std::filesystem::remove(walFilePath);
    // No WAL file, but the shadow file exists. Should not replay the shadow file, and remove wal
    // and shadow files.
    createDBAndConn();
    auto res = conn->query("CALL show_tables() WHERE name='test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 0);
    ASSERT_FALSE(std::filesystem::exists(walFilePath));
    ASSERT_FALSE(std::filesystem::exists(shadowFilePath));
}

TEST_F(WalTest, ShadowFileExistsWithEmptyWAL) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("BEGIN TRANSACTION;");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("COMMIT;");
    auto shadowFilePath = rag3db::storage::StorageUtils::getShadowFilePath(databasePath);
    // Create a shadow file that is corrupted.
    std::ofstream file(shadowFilePath);
    file << "This is not a valid Rag3db database file.";
    file.close();
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 0);
    std::filesystem::resize_file(walFilePath, 0);
    // Empty WAL file, but the shadow file exists. Should not replay the shadow file, and remove wal
    // and shadow files.
    createDBAndConn();
    auto res = conn->query("CALL show_tables() WHERE name='test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 0);
    ASSERT_FALSE(std::filesystem::exists(walFilePath));
    ASSERT_FALSE(std::filesystem::exists(shadowFilePath));
}

void WalTest::setupChecksumMismatchTest(std::function<void(std::ofstream&)> corruptFunc) {
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("BEGIN TRANSACTION;");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test2(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test3(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test4(id INT64 PRIMARY KEY, name STRING);");
    conn->query("COMMIT;");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    // rewrite part of the wal
    std::ofstream file(walFilePath, std::ios_base::in | std::ios_base::out | std::ios_base::ate);
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 13);
    corruptFunc(file);
    file.close();
}

// Simulation of a corrupted WAL tail by changing some data, this should trigger a checksum failure
TEST_F(WalTest, CorruptedWALChecksumMismatchInHeader) {
    if (inMemMode || systemConfig->checkpointThreshold == 0 || !systemConfig->enableChecksums) {
        GTEST_SKIP();
    }
    systemConfig->throwOnWalReplayFailure = true;
    createDBAndConn();
    setupChecksumMismatchTest([](std::ofstream& walFileToCorrupt) {
        walFileToCorrupt.seekp(10);
        // 10 bytes in will be the database ID's checksum
        walFileToCorrupt << "abc";
    });
    EXPECT_THROW(createDBAndConn();, rag3db::common::StorageException);
}

TEST_F(WalTest, CorruptedWALChecksumMismatchInHeaderNoThrow) {
    if (inMemMode || systemConfig->checkpointThreshold == 0 || !systemConfig->enableChecksums) {
        GTEST_SKIP();
    }
    setupChecksumMismatchTest([](std::ofstream& walFileToCorrupt) {
        walFileToCorrupt.seekp(10);
        // 10 bytes in will be the database ID's checksum
        walFileToCorrupt << "abc";
    });

    // The replay shouldn't complete but shouldn't throw either
    createDBAndConn();
    auto result = conn->query("match (t:test) return count(*)");
    EXPECT_FALSE(result->isSuccess());
}

TEST_F(WalTest, CorruptedWALChecksumMismatchInBody) {
    if (inMemMode || systemConfig->checkpointThreshold == 0 || !systemConfig->enableChecksums) {
        GTEST_SKIP();
    }
    systemConfig->throwOnWalReplayFailure = true;
    createDBAndConn();
    setupChecksumMismatchTest([](std::ofstream& walFileToCorrupt) {
        walFileToCorrupt.seekp(30);
        walFileToCorrupt << "abc";
    });
    EXPECT_THROW(createDBAndConn();, rag3db::common::StorageException);
}

TEST_F(WalTest, CorruptedWALChecksumMismatchInBodyNoThrow) {
    if (inMemMode || systemConfig->checkpointThreshold == 0 || !systemConfig->enableChecksums) {
        GTEST_SKIP();
    }
    setupChecksumMismatchTest([](std::ofstream& walFileToCorrupt) {
        walFileToCorrupt.seekp(10);
        // 10 bytes in will be the database ID's checksum
        walFileToCorrupt << "abc";
    });
    // The replay shouldn't complete but shouldn't throw either
    createDBAndConn();
    auto result = conn->query("match (t:test) return count(*)");
    EXPECT_FALSE(result->isSuccess());
    EXPECT_STREQ("Binder exception: Table test does not exist.", result->getErrorMessage().c_str());
}

TEST_F(WalTest, WALChecksumConfigMismatch) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    ASSERT_TRUE(conn->query("call force_checkpoint_on_close=false")->isSuccess());
    ASSERT_TRUE(conn->query("call auto_checkpoint=false")->isSuccess());
    ASSERT_TRUE(conn->query("create node table test1(id int64 primary key)")->isSuccess());
    ASSERT_TRUE(conn->query("create node table test2(id int64 primary key)")->isSuccess());
    systemConfig->enableChecksums = !systemConfig->enableChecksums;
    systemConfig->throwOnWalReplayFailure = true;
    EXPECT_THROW(
        {
            try {
                createDBAndConn();
            } catch (std::exception& e) {
                EXPECT_THAT(e.what(),
                    testing::AnyOf(testing::StartsWith(
                                       "Runtime exception: The database you are trying to open "
                                       "was serialized with enableChecksums=True but you are "
                                       "trying to open it with enableChecksums=False."),
                        testing::StartsWith(
                            "Runtime exception: The database you are trying to open "
                            "was serialized with enableChecksums=False but you are "
                            "trying to open it with enableChecksums=True.")));
                throw;
            }
        },
        rag3db::common::RuntimeException);
}

TEST_F(WalTest, WALChecksumConfigMismatchNoThrow) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    ASSERT_TRUE(conn->query("call force_checkpoint_on_close=false")->isSuccess());
    ASSERT_TRUE(conn->query("call auto_checkpoint=false")->isSuccess());
    ASSERT_TRUE(conn->query("create node table test1(id int64 primary key)")->isSuccess());
    ASSERT_TRUE(conn->query("create node table test2(id int64 primary key)")->isSuccess());
    systemConfig->enableChecksums = !systemConfig->enableChecksums;
    systemConfig->throwOnWalReplayFailure = false;
    // We have throwOnWalReplayFailure=false so we essentially skip the replay
    createDBAndConn();
    auto res = conn->query("CALL show_tables() WHERE name STARTS WITH 'test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 0);
}

// Simulation of a corrupted WAL tail by truncating the WAL file. Note that in this case, there
// would only be a single write transaction. This would cause the last wal record to be corrupted
// and the database should ignore the last record when recovering.
TEST_F(WalTest, CorruptedWALTailTruncated) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("BEGIN TRANSACTION;");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test2(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test3(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test4(id INT64 PRIMARY KEY, name STRING);");
    conn->query("COMMIT;");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 10);
    // Truncate the last 10 bytes of the WAL file.
    std::filesystem::resize_file(walFilePath, std::filesystem::file_size(walFilePath) - 10);
    createDBAndConn();
    auto res = conn->query("CALL show_tables() WHERE name STARTS WITH 'test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 0);
}

// Simulation of a corrupted WAL tail by truncating the WAL file, but then continuing to write to
// the WAL, and recover from the same WAL file again. This should recover the tables that were
// created after the database's first recovering from the corrupted WAL.
TEST_F(WalTest, CorruptedWALTailTruncatedAndRecoverTwice) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("BEGIN TRANSACTION;");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test2(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test3(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test4(id INT64 PRIMARY KEY, name STRING);");
    conn->query("COMMIT;");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 10);
    // Truncate the last 10 bytes of the WAL file.
    std::filesystem::resize_file(walFilePath, std::filesystem::file_size(walFilePath) - 10);
    // Restart to recover from the corrupted WAL file.
    createDBAndConn();
    auto res = conn->query("CALL show_tables() WHERE name STARTS WITH 'test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 0);
    // Continue with some more writes to the WAL.
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test2(id INT64 PRIMARY KEY, name STRING);");
    // Recover again from the same WAL file.
    createDBAndConn();
    res = conn->query("CALL show_tables() WHERE name STARTS WITH 'test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 2);
}

// Similar to CorruptedWALTailTruncated, but with multiple transactions.
TEST_F(WalTest, CorruptedWALTailTruncated2) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test2(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test3(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test4(id INT64 PRIMARY KEY, name STRING);");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 10);
    // Truncate the last 10 bytes of the WAL file.
    std::filesystem::resize_file(walFilePath, std::filesystem::file_size(walFilePath) - 10);
    createDBAndConn();
    auto res = conn->query("CALL show_tables() WHERE name STARTS WITH 'test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 3);
}

TEST_F(WalTest, WALFileLeftoverFromPreviousDBNewReadOnlyDB) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test2(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test3(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test4(id INT64 PRIMARY KEY, name STRING);");

    // Delete the DB file but keep the WAL
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    conn.reset();
    database.reset();
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::exists(databasePath));
    std::filesystem::remove(databasePath);

    // Recreate the DB then close it
    ASSERT_FALSE(std::filesystem::exists(databasePath));
    auto createEmptyDB = [&]() {
        database = std::make_unique<rag3db::main::Database>(databasePath, *systemConfig);
    };
    // When opening an empty read-only DB we don't write the header
    // This shouldn't cause any crashes when deserializing
    systemConfig->readOnly = true;
    // Creating a new empty DB file bypasses the empty read-only DB check
    std::ofstream ofs(databasePath);
    ofs.close();

    // When loading the DB, replaying should fail
    EXPECT_THROW(createEmptyDB(), RuntimeException);
}

void WalTest::testStrayWALFile(const std::function<void()>& setupNewDBFunc) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }

    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test2(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test3(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test4(id INT64 PRIMARY KEY, name STRING);");

    // Delete the DB file but keep the WAL
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    conn.reset();
    database.reset();
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::exists(databasePath));
    std::filesystem::remove(databasePath);

    // temporarily rename the WAL file so that replay doesn't immediately trigger
    auto tmpWALPath = walFilePath + "__";
    std::filesystem::rename(walFilePath, tmpWALPath);

    // Recreate the DB then close it
    createDBAndConn();
    setupNewDBFunc();
    conn.reset();
    database.reset();

    // Rename the WAL to the original
    ASSERT_FALSE(std::filesystem::exists(walFilePath));
    std::filesystem::rename(tmpWALPath, walFilePath);

    // When loading the DB, replaying should fail
    EXPECT_THROW(createDBAndConn(), RuntimeException);
}

TEST_F(WalTest, WALFileLeftoverFromPreviousDBExistingDB) {
    testStrayWALFile([]() {});
}

TEST_F(WalTest, WALFileLeftoverFromPreviousDBNewDBCOPYWithoutCheckpoint) {
    testStrayWALFile([this]() {
        conn->query("CALL force_checkpoint_on_close=false");
        conn->query("create node table Comment (id int64, creationDate INT64, locationIP STRING, "
                    "browserUsed STRING, content STRING, length INT32, PRIMARY KEY (id));");
        conn->query(stringFormat("COPY Comment FROM '{}/dataset/ldbc-sf01/Comment.csv'",
            RAG3DB_ROOT_DIRECTORY));
    });
}

TEST_F(WalTest, WALFileLeftoverFromPreviousDBNewDBCOPYWithoutCheckpointReadOnly) {
    testStrayWALFile([this]() {
        conn->query("CALL force_checkpoint_on_close=false");
        conn->query("create node table Comment (id int64, creationDate INT64, locationIP STRING, "
                    "browserUsed STRING, content STRING, length INT32, PRIMARY KEY (id));");
        conn->query(stringFormat("COPY Comment FROM '{}/dataset/ldbc-sf01/Comment.csv'",
            RAG3DB_ROOT_DIRECTORY));
        systemConfig->readOnly = true;
    });
}

// Similar to CorruptedWALTailTruncated2, but with multiple transactions and then recovering from
// the WAL file again. This should recover the tables that were created after the database's first
// recovering from the corrupted WAL.
TEST_F(WalTest, CorruptedWALTailTruncated2RecoverTwice) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test2(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test3(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test4(id INT64 PRIMARY KEY, name STRING);");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 10);
    // Truncate the last 10 bytes of the WAL file.
    std::filesystem::resize_file(walFilePath, std::filesystem::file_size(walFilePath) - 10);
    // Restart to recover from the corrupted WAL file.
    createDBAndConn();
    auto res = conn->query("CALL show_tables() WHERE name STARTS WITH 'test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 3);
    // Continue with some more writes to the WAL.
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("CREATE NODE TABLE test5(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE NODE TABLE test6(id INT64 PRIMARY KEY, name STRING);");
    // Recover again from the same WAL file.
    createDBAndConn();
    res = conn->query("CALL show_tables() WHERE name STARTS WITH 'test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 5);
}

TEST_F(WalTest, ReadOnlyRecoveryFromExistingWAL) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE (:test {id: 1, name: 'Alice'});");
    conn->query("CREATE (:test {id: 2, name: 'Bob'});");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 0);

    // Restart in read-only mode
    systemConfig->readOnly = true;
    createDBAndConn();
    auto res = conn->query("MATCH (n:test) RETURN n.id ORDER BY n.id;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 2);
    // WAL file should still exist in read-only mode
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
}

TEST_F(WalTest, ReadOnlyRecoveryFromCorruptedWALTail) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE (:test {id: 1, name: 'Alice'});");
    conn->query("CREATE (:test {id: 2, name: 'Bob'});");
    conn->query("CREATE (:test {id: 3, name: 'Charlie'});");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 10);

    // Truncate the last 10 bytes of the WAL file to simulate corruption
    std::filesystem::resize_file(walFilePath, std::filesystem::file_size(walFilePath) - 10);

    // Restart in read-only mode
    systemConfig->readOnly = true;
    createDBAndConn();
    auto res = conn->query("MATCH (n:test) RETURN n.id ORDER BY n.id;");
    ASSERT_TRUE(res->isSuccess());
    // Should still recover up to the last valid record
    ASSERT_GE(res->getNumTuples(), 0);
    // WAL file should remain unchanged in read-only mode
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
}

TEST_F(WalTest, ReadOnlyRecoveryWithShadowFile) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    conn->query("CREATE (:test {id: 1, name: 'Alice'});");
    conn->query("COMMIT;");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    auto shadowFilePath = rag3db::storage::StorageUtils::getShadowFilePath(databasePath);

    // Create a shadow file (simulating checkpoint in progress)
    std::ofstream file(shadowFilePath);
    file << "shadow file content";
    file.close();
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::exists(shadowFilePath));

    // Restart in read-only mode
    systemConfig->readOnly = true;
    createDBAndConn();
    auto res = conn->query("MATCH (n:test) RETURN n.id ORDER BY n.id;");
    ASSERT_TRUE(res->isSuccess());
    // Should handle WAL recovery correctly
    ASSERT_GE(res->getNumTuples(), 0);
    // Files should remain unchanged in read-only mode
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::exists(shadowFilePath));
}

TEST_F(WalTest, ReadOnlyRecoveryEmptyWALFile) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 0);

    // Make WAL file empty
    std::filesystem::resize_file(walFilePath, 0);

    // Restart in read-only mode
    systemConfig->readOnly = true;
    createDBAndConn();
    auto res = conn->query("CALL show_tables() WHERE name='test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 0);
    // Empty WAL file should still exist in read-only mode
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
}

TEST_F(WalTest, ReadOnlyRecoveryNoWALFile) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    ASSERT_TRUE(std::filesystem::file_size(walFilePath) > 0);

    // Remove WAL file
    std::filesystem::remove(walFilePath);
    ASSERT_FALSE(std::filesystem::exists(walFilePath));

    // Restart in read-only mode
    systemConfig->readOnly = true;
    createDBAndConn();
    auto res = conn->query("CALL show_tables() WHERE name='test' RETURN *;");
    ASSERT_TRUE(res->isSuccess());
    ASSERT_EQ(res->getNumTuples(), 0);
}

// Un texte reconnaissable à chaque position : un début perdu ne peut pas
// passer pour une valeur juste.
static std::string longText(char seed, size_t size) {
    std::string text(size, ' ');
    for (size_t i = 0; i < size; ++i) {
        text[i] = static_cast<char>('a' + (i * 7 + seed) % 26);
    }
    return text;
}

// Des enregistrements de plus de 4096 octets se rejouent avec leurs valeurs.
// ChecksumWriter::resizeBufferIfNeeded remplaçait le tampon sans recopier le
// début déjà écrit : l'enregistrement perdait son octet de type, sa somme
// était calculée sur le tampon faux, et le rejeu échouait (wal_record.cpp,
// type inconnu ou nul) — vu sur la base MTG le 27 septembre 2026. On compare
// les valeurs relues, pas un message : le début perdu est du tas non
// initialisé, l'erreur varie d'une exécution à l'autre.
TEST_F(WalTest, LongRecordsReplayWithTheirValues) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    systemConfig->throwOnWalReplayFailure = true;
    conn->query("CALL force_checkpoint_on_close=false");
    ASSERT_TRUE(conn->query("CREATE NODE TABLE t(id INT64 PRIMARY KEY, s STRING);")->isSuccess());
    const std::map<int64_t, std::string> expected{{1, longText(1, 20000)}, {2, longText(2, 4097)},
        {3, longText(3, 8193)}, {4, longText(4, 100000)}};
    auto create = conn->prepare("CREATE (:t {id: $id, s: $s});");
    ASSERT_TRUE(create->isSuccess()) << create->getErrorMessage();
    auto run = [&](int64_t id, const std::string& s) {
        auto result = conn->execute(create.get(), std::make_pair(std::string("id"), id),
            std::make_pair(std::string("s"), s));
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    };
    // Seul dans sa transaction, puis plusieurs gros enregistrements dans une même.
    run(1, longText(9, 4097));
    ASSERT_TRUE(conn->query("BEGIN TRANSACTION;")->isSuccess());
    run(2, expected.at(2));
    run(3, expected.at(3));
    run(4, expected.at(4));
    ASSERT_TRUE(conn->query("COMMIT;")->isSuccess());
    auto set = conn->prepare("MATCH (n:t) WHERE n.id = 1 SET n.s = $s;");
    auto updated = conn->execute(set.get(), std::make_pair(std::string("s"), expected.at(1)));
    ASSERT_TRUE(updated->isSuccess()) << updated->getErrorMessage();
    auto walFilePath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
    ASSERT_GT(std::filesystem::file_size(walFilePath), 100000u);

    createDBAndConn();
    auto result = conn->query("MATCH (n:t) RETURN n.id, n.s ORDER BY n.id;");
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    std::map<int64_t, std::string> replayed;
    while (result->hasNext()) {
        auto row = result->getNext();
        replayed[row->getValue(0)->getValue<int64_t>()] = row->getValue(1)->getValue<std::string>();
    }
    ASSERT_EQ(replayed.size(), expected.size());
    for (const auto& [id, text] : expected) {
        ASSERT_EQ(replayed[id].size(), text.size()) << "id " << id;
        ASSERT_TRUE(replayed[id] == text) << "id " << id << " : la valeur relue diffère";
    }
}

// ─── Fin de journal déchirée (décision de Lucie, 1er octobre 2026) ───────────
//
// Un arrêt brutal pendant l'écriture d'un commit laisse un journal dont la fin
// est coupée. Avec les réglages par défaut, la base rouvre au dernier COMMIT
// complet ; ce qui suit est écarté, copié à côté du journal, et dit. Une
// corruption au milieu d'un journal qui continue reste un refus d'ouvrir.

class WalTornEndTest : public WalTest {
protected:
    // Trois transactions validées ; la troisième est longue (plusieurs pages du
    // lecteur). `ends` reçoit la taille du journal après chacune.
    void writeThreeTransactions(std::vector<uint64_t>& ends) {
        conn->query("CALL force_checkpoint_on_close=false");
        ASSERT_TRUE(
            conn->query("CREATE NODE TABLE t(id INT64 PRIMARY KEY, s STRING);")->isSuccess());
        walPath = rag3db::storage::StorageUtils::getWALFilePath(databasePath);
        ends.push_back(std::filesystem::file_size(walPath));
        auto create = conn->prepare("CREATE (:t {id: $id, s: $s});");
        for (int64_t id = 1; id <= 3; ++id) {
            auto result = conn->execute(create.get(), std::make_pair(std::string("id"), id),
                std::make_pair(std::string("s"), longText(static_cast<char>(id), id == 3 ? 9000 : 100)));
            ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
            ends.push_back(std::filesystem::file_size(walPath));
        }
        conn.reset();
        database.reset();
        std::filesystem::copy_file(databasePath, databasePath + ".pristine",
            std::filesystem::copy_options::overwrite_existing);
        std::filesystem::copy_file(walPath, walPath + ".pristine",
            std::filesystem::copy_options::overwrite_existing);
    }

    // La base d'avant, avec un journal coupé à `length` octets.
    void restoreWithWALCutAt(uint64_t length) {
        conn.reset();
        database.reset();
        for (const auto& entry : std::filesystem::directory_iterator(
                 std::filesystem::path(databasePath).parent_path())) {
            if (entry.path().string().find(".ecarte-") != std::string::npos ||
                entry.path().string().ends_with(".shadow")) {
                std::filesystem::remove(entry.path());
            }
        }
        std::filesystem::copy_file(databasePath + ".pristine", databasePath,
            std::filesystem::copy_options::overwrite_existing);
        std::filesystem::copy_file(walPath + ".pristine", walPath,
            std::filesystem::copy_options::overwrite_existing);
        std::filesystem::resize_file(walPath, length);
    }

    std::vector<int64_t> idsAfterReopen() {
        createDBAndConn();
        std::vector<int64_t> ids;
        auto result = conn->query("MATCH (n:t) RETURN n.id ORDER BY n.id;");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        while (result->hasNext()) {
            ids.push_back(result->getNext()->getValue(0)->getValue<int64_t>());
        }
        return ids;
    }

    uint64_t discardedBytes() const {
        uint64_t total = 0;
        for (const auto& entry : std::filesystem::directory_iterator(
                 std::filesystem::path(databasePath).parent_path())) {
            if (entry.path().string().find(".ecarte-") != std::string::npos) {
                total += std::filesystem::file_size(entry.path());
            }
        }
        return total;
    }

    static std::string readAll(const std::string& path) {
        std::ifstream file(path, std::ios::binary);
        return {std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>()};
    }

    // Le journal d'origine (`original`, avant réouverture) = le journal tronqué
    // + le fichier écarté, à l'octet près : rien n'est perdu.
    void expectNothingLost(const std::string& original) const {
        std::string aside;
        for (const auto& entry : std::filesystem::directory_iterator(
                 std::filesystem::path(databasePath).parent_path())) {
            if (entry.path().string().find(".ecarte-") != std::string::npos) {
                aside += readAll(entry.path().string());
            }
        }
        EXPECT_TRUE(readAll(walPath) + aside == original)
            << "journal " << std::filesystem::file_size(walPath) << " + écarté " << aside.size()
            << " != origine " << original.size();
    }

    // Écrit `value` (little endian, `width` octets) à `offset` dans le journal.
    void patchWAL(uint64_t offset, uint64_t value, int width) const {
        std::fstream wal(walPath, std::ios::in | std::ios::out | std::ios::binary);
        wal.seekp(offset);
        for (int i = 0; i < width; ++i) {
            const char byte = static_cast<char>((value >> (8 * i)) & 0xff);
            wal.write(&byte, 1);
        }
    }

    std::string walPath;
};

TEST_F(WalTornEndTest, ReopensAtTheLastCompleteCommitWithDefaultSettings) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    std::vector<uint64_t> ends;
    writeThreeTransactions(ends);
    ASSERT_EQ(ends.size(), 4u);
    // Les réglages par défaut : un échec du rejeu refuse d'ouvrir.
    systemConfig->throwOnWalReplayFailure = true;
    const auto begin3 = ends[2], end3 = ends[3];
    ASSERT_GT(end3 - begin3, 9000u);
    const std::vector<std::pair<const char*, uint64_t>> cuts{
        {"pile sur la frontière du COMMIT 2", begin3},
        {"juste après l'octet de type du BEGIN", begin3 + 1},
        {"juste après l'octet de type du premier enregistrement", begin3 + 10},
        {"au milieu d'un corps", begin3 + (end3 - begin3) / 2},
        {"entre les données et le COMMIT", end3 - 9},
        {"au milieu de la somme du COMMIT", end3 - 4},
    };
    for (const auto& [where, length] : cuts) {
        SCOPED_TRACE(where);
        restoreWithWALCutAt(length);
        std::vector<int64_t> ids;
        ASSERT_NO_THROW(ids = idsAfterReopen());
        EXPECT_EQ(ids, (std::vector<int64_t>{1, 2}));
        // Ce qui suit le dernier COMMIT est copié à côté, pas perdu en silence.
        EXPECT_EQ(discardedBytes(), length - begin3);
        expectNothingLost(readAll(walPath + ".pristine").substr(0, length));
    }
    restoreWithWALCutAt(end3);
    EXPECT_EQ(idsAfterReopen(), (std::vector<int64_t>{1, 2, 3}));
    EXPECT_EQ(discardedBytes(), 0u);
}

TEST_F(WalTornEndTest, CorruptionBeforeCommittedTransactionsStillRefusesToOpen) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    std::vector<uint64_t> ends;
    writeThreeTransactions(ends);
    systemConfig->throwOnWalReplayFailure = true;
    restoreWithWALCutAt(ends[3]);
    // Un octet retourné dans la transaction 2, suivie de la transaction 3 validée.
    const auto flipAt = ends[1] + (ends[2] - ends[1]) / 2;
    {
        std::fstream wal(walPath, std::ios::in | std::ios::out | std::ios::binary);
        wal.seekg(flipAt);
        char byte = 0;
        wal.read(&byte, 1);
        byte = static_cast<char>(~byte);
        wal.seekp(flipAt);
        wal.write(&byte, 1);
    }
    EXPECT_ANY_THROW(createDBAndConn());
    EXPECT_EQ(discardedBytes(), 0u);
    // Le journal n'a pas été tronqué : rien de validé n'est jeté.
    EXPECT_EQ(std::filesystem::file_size(walPath), ends[3]);
}

// La limite connue, fixée par un test : sans longueur par enregistrement, une
// longueur abîmée au milieu du journal fait demander plus que le fichier, et se
// lit comme une fin déchirée. La base rouvre alors au dernier COMMIT avant la
// corruption : la transaction 3, pourtant validée, est écartée — mais copiée à
// côté, à l'octet près, et rien n'est supprimé.
TEST_F(WalTornEndTest, CorruptedLengthReadsAsATornEndAndIsSetAsideWhole) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    std::vector<uint64_t> ends;
    writeThreeTransactions(ends);
    systemConfig->throwOnWalReplayFailure = true;
    restoreWithWALCutAt(ends[3]);
    // La longueur de la chaîne de la transaction 2, juste avant ses octets.
    const auto pristine = readAll(walPath);
    const auto text = longText(2, 100);
    const auto at = pristine.find(text, ends[1]);
    ASSERT_NE(at, std::string::npos);
    ASSERT_LT(at, ends[2]);
    uint64_t width = 0;
    for (const uint64_t candidate : {8u, 4u}) {
        uint64_t value = 0;
        for (uint64_t i = 0; i < candidate; ++i) {
            value |= static_cast<uint64_t>(static_cast<uint8_t>(pristine[at - candidate + i]))
                     << (8 * i);
        }
        if (value == text.size()) {
            width = candidate;
            break;
        }
    }
    ASSERT_NE(width, 0u) << "longueur de la chaîne introuvable avant ses octets";
    patchWAL(at - width, pristine.size() + 4096, static_cast<int>(width));
    const auto corrupted = readAll(walPath);
    std::vector<int64_t> ids;
    ASSERT_NO_THROW(ids = idsAfterReopen());
    EXPECT_EQ(ids, (std::vector<int64_t>{1}));
    EXPECT_EQ(std::filesystem::file_size(walPath), ends[1]);
    EXPECT_EQ(discardedBytes(), ends[3] - ends[1]);
    expectNothingLost(corrupted);
}

// throwOnWalReplayFailure=false sur une vraie corruption : le journal est
// tronqué après le dernier COMMIT lisible, et ce qui est retiré est aussi copié
// à côté — plus aucune troncature sans copie.
TEST_F(WalTornEndTest, ReplayFailureWithoutThrowSetsTheCutAside) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    std::vector<uint64_t> ends;
    writeThreeTransactions(ends);
    systemConfig->throwOnWalReplayFailure = false;
    restoreWithWALCutAt(ends[3]);
    const auto flipAt = ends[1] + (ends[2] - ends[1]) / 2;
    const auto pristine = readAll(walPath);
    patchWAL(flipAt, static_cast<uint8_t>(~static_cast<uint8_t>(pristine[flipAt])), 1);
    const auto corrupted = readAll(walPath);
    std::vector<int64_t> ids;
    ASSERT_NO_THROW(ids = idsAfterReopen());
    EXPECT_EQ(ids, (std::vector<int64_t>{1}));
    EXPECT_EQ(discardedBytes(), ends[3] - ends[1]);
    expectNothingLost(corrupted);
}

// En lecture seule, une fin incomplète (un écrivain vivant en train d'ajouter
// au journal) rouvre au dernier COMMIT sans rien écrire ni retirer : journal
// inchangé à l'octet, aucun fichier écarté.
TEST_F(WalTornEndTest, ReadOnlyOpenOnAnIncompleteEndChangesNothing) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    std::vector<uint64_t> ends;
    writeThreeTransactions(ends);
    systemConfig->throwOnWalReplayFailure = true;
    const auto cut = ends[2] + (ends[3] - ends[2]) / 2;
    restoreWithWALCutAt(cut);
    const auto before = readAll(walPath);
    systemConfig->readOnly = true;
    std::vector<int64_t> ids;
    ASSERT_NO_THROW(ids = idsAfterReopen());
    EXPECT_EQ(ids, (std::vector<int64_t>{1, 2}));
    conn.reset();
    database.reset();
    EXPECT_TRUE(readAll(walPath) == before);
    EXPECT_EQ(discardedBytes(), 0u);
}
