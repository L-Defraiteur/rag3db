#include <filesystem>
#include <fstream>

#include "api_test/private_api_test.h"
#include "common/exception/runtime.h"
#include "flaky_checkpointer.h"
#include "storage/checkpointer.h"
#include "storage/storage_manager.h"
#include "storage/wal/wal.h"
#include "test_helper/flaky_buffer_manager.h"
#include "transaction/transaction_manager.h"

using namespace rag3db::common;
using namespace rag3db::testing;
using namespace rag3db::transaction;
using namespace rag3db::storage;

namespace rag3db {
namespace testing {

class FlakyCheckpointerTest : public PrivateApiTest {
public:
    std::string getInputDir() override { return "empty"; }

    void runFlakyCheckpoint(const FlakyCheckpointer& flakyCheckpointer) {
        conn->query("CALL force_checkpoint_on_close=false;");
        conn->query("CALL auto_checkpoint=false");
        conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
        for (auto i = 0; i < 5000; i++) {
            conn->query(stringFormat("CREATE (a:test {id: {}, name: 'name_{}'});", i, i));
        }
        auto context = getClientContext(*conn);
        flakyCheckpointer.setCheckpointer(*context);
        auto res = conn->query("CHECKPOINT;");
        ASSERT_FALSE(res->isSuccess());
    }

    void runTest(const FlakyCheckpointer& flakyCheckpointer) {
        runFlakyCheckpoint(flakyCheckpointer);
        createDBAndConn();
        auto res = conn->query("MATCH (a:test) RETURN COUNT(a);");
        ASSERT_TRUE(res->isSuccess());
        ASSERT_EQ(res->getNext()->getValue(0)->getValue<int64_t>(), 5000);
    }
};

class FlakyCheckpointerFailsOnCheckpointStorage final : public Checkpointer {
public:
    explicit FlakyCheckpointerFailsOnCheckpointStorage(main::ClientContext& clientContext)
        : Checkpointer(clientContext) {}

    bool checkpointStorage() override { throw RuntimeException("checkpoint failed."); }
};

TEST_F(FlakyCheckpointerTest, RecoverFromCheckpointStorageFailure) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    auto initFlakyCheckpointer = [](main::ClientContext& context) {
        return std::make_unique<FlakyCheckpointerFailsOnCheckpointStorage>(context);
    };
    FlakyCheckpointer flakyCheckpointer(initFlakyCheckpointer);
    runTest(flakyCheckpointer);
}

class FlakyCheckpointerFailsOnSerialization final : public Checkpointer {
public:
    explicit FlakyCheckpointerFailsOnSerialization(main::ClientContext& context)
        : Checkpointer(context) {}

    void serializeCatalogAndMetadata(DatabaseHeader&, bool) override {
        throw RuntimeException("checkpoint failed.");
    }
};

TEST_F(FlakyCheckpointerTest, RecoverFromCheckpointSerializeFailure) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    auto initFlakyCheckpointer = [](main::ClientContext& context) {
        return std::make_unique<FlakyCheckpointerFailsOnSerialization>(context);
    };
    FlakyCheckpointer flakyCheckpointer(initFlakyCheckpointer);
    runTest(flakyCheckpointer);
}

// La même panne, sur une table qui a déjà vécu. Les autres tests de ce fichier
// interrompent le PREMIER point de reprise d'une table neuve. Ici la table en a
// onze derrière elle : des pages ont été libérées puis réutilisées, et les
// pages de l'index de clé primaire ne sont plus dans l'ordre du fichier. Le
// point de reprise s'arrête après sa phase de stockage, avant d'avoir rien
// journalisé : le fichier de données doit encore être celui du point de
// reprise précédent, et le journal se rejouer par-dessus.
TEST_F(FlakyCheckpointerTest, RecoverFromCheckpointSerializeFailureAfterEarlierCheckpoints) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    conn->query("CALL force_checkpoint_on_close=false;");
    conn->query("CALL auto_checkpoint=false");
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");
    auto numRows = 0;
    for (auto cycle = 0; cycle < 12; cycle++) {
        for (auto i = 0; i < 25; i++, numRows++) {
            auto res = conn->query(
                stringFormat("CREATE (a:test {id: {}, name: 'name_{}'});", numRows, numRows));
            ASSERT_TRUE(res->isSuccess()) << res->getErrorMessage();
        }
        if (cycle < 11) {
            ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
        }
    }
    FlakyCheckpointer flakyCheckpointer([](main::ClientContext& context) {
        return std::make_unique<FlakyCheckpointerFailsOnSerialization>(context);
    });
    flakyCheckpointer.setCheckpointer(*getClientContext(*conn));
    ASSERT_FALSE(conn->query("CHECKPOINT;")->isSuccess());

    // La « panne » : on rouvre. Le journal porte les 25 dernières lignes, et le
    // contenu doit être exactement celui du dernier commit acquitté — ni une
    // ligne de moins, ni une clé en double, ni une valeur d'une autre ligne.
    createDBAndConn();
    const auto single = [&](const std::string& query) {
        auto res = conn->query(query);
        EXPECT_TRUE(res->isSuccess()) << query << " : " << res->getErrorMessage();
        return res->isSuccess() && res->hasNext() ?
                   res->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    };
    EXPECT_EQ(single("MATCH (a:test) RETURN COUNT(a);"), numRows);
    EXPECT_EQ(single("MATCH (a:test) RETURN COUNT(DISTINCT a.id);"), numRows);
    EXPECT_EQ(single("MATCH (a:test) RETURN SUM(a.id);"), numRows * (numRows - 1) / 2);
    EXPECT_EQ(
        single("MATCH (a:test) WHERE a.name <> 'name_' + cast(a.id AS STRING) RETURN COUNT(a);"),
        0);
    // Chaque clé se retrouve par l'index, une à une : c'est lui qui était abîmé.
    for (auto id = 0; id < numRows; id++) {
        ASSERT_EQ(single(stringFormat("MATCH (a:test) WHERE a.id = {} RETURN COUNT(a);", id)), 1)
            << "clé " << id;
    }
    // Et la base rouverte sert encore : une ligne de plus, un point de reprise
    // complet, une réouverture.
    ASSERT_TRUE(
        conn->query(stringFormat("CREATE (a:test {id: {}, name: 'name_{}'});", numRows, numRows))
            ->isSuccess());
    ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
    createDBAndConn();
    EXPECT_EQ(single("MATCH (a:test) RETURN COUNT(a);"), numRows + 1);
    EXPECT_EQ(single("MATCH (a:test) RETURN COUNT(DISTINCT a.id);"), numRows + 1);
}

// Un point de reprise ordinaire, qui note s'il est dans sa phase de stockage.
class CheckpointerTellingItsStoragePhase final : public Checkpointer {
public:
    CheckpointerTellingItsStoragePhase(main::ClientContext& context, bool& inStoragePhase)
        : Checkpointer(context), inStoragePhase{inStoragePhase} {}

    bool checkpointStorage() override {
        inStoragePhase = true;
        const auto hasChanges = Checkpointer::checkpointStorage();
        // Non atteint si la phase échoue : le drapeau reste levé, c'est ce qu'on lit.
        inStoragePhase = false;
        return hasChanges;
    }

private:
    bool& inStoragePhase;
};

// La panne PENDANT la phase de stockage, et non plus après elle. Le douzième
// point de reprise est interrompu à chacune de ses allocations, l'une après
// l'autre : à la k-ième, pour tout k, jusqu'à ce qu'il n'y en ait plus et qu'il
// réussisse. Deux tables, parce qu'une table écrit son index en dernier : quand
// la seconde échoue, les pages de l'index de la première sont déjà sur disque.
// Après chaque panne on rouvre, et le contenu doit être celui du dernier commit.
TEST_F(FlakyCheckpointerTest, RecoverFromFailureAtEveryAllocationOfALaterCheckpoint) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    static constexpr auto NUM_CYCLES = 12;
    static constexpr auto NUM_ROWS_PER_CYCLE = 25;
    static constexpr int64_t NUM_ROWS = NUM_CYCLES * NUM_ROWS_PER_CYCLE;
    const std::vector<std::string> tables = {"test", "other"};
    const auto single = [&](const std::string& query) {
        auto res = conn->query(query);
        EXPECT_TRUE(res->isSuccess()) << query << " : " << res->getErrorMessage();
        return res->isSuccess() && res->hasNext() ?
                   res->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    };
    auto numFailures = 0u;
    auto numFailuresInStoragePhase = 0u;
    for (uint64_t k = 1;; k++) {
        ASSERT_LT(k, 5000u) << "le point de reprise n'en finit pas d'allouer";
        // Une base neuve, sur un gestionnaire de tampons qu'on peut faire échouer.
        conn.reset();
        database.reset();
        removeParentDirectoryOfDBPath(databasePath);
        std::filesystem::create_directories(std::filesystem::path(databasePath).parent_path());
        std::atomic<uint64_t> failureFrequency = UINT64_MAX;
        FlakyBufferManager* bufferManager = nullptr;
        database =
            constructDB(databasePath, *systemConfig, [&](const main::Database& db) {
                auto bm = std::make_unique<FlakyBufferManager>(databasePath,
                    databasePath + ".checkpoint.tmp", systemConfig->bufferPoolSize,
                    systemConfig->maxDBSize, getFileSystem(db), systemConfig->readOnly,
                    failureFrequency, false /* canFailDuringExecute */,
                    true /* canFailDuringCheckpoint */, false /* canFailDuringCommit */);
                bufferManager = bm.get();
                return bm;
            });
        conn = std::make_unique<main::Connection>(database.get());
        bufferManager->setClientContext(getClientContext(*conn));
        conn->query("CALL force_checkpoint_on_close=false;");
        conn->query("CALL auto_checkpoint=false");
        for (const auto& table : tables) {
            ASSERT_TRUE(conn->query(stringFormat(
                                        "CREATE NODE TABLE {}(id INT64 PRIMARY KEY, name STRING);",
                                        table))
                            ->isSuccess());
        }
        auto numRows = 0;
        for (auto cycle = 0; cycle < NUM_CYCLES; cycle++) {
            for (auto i = 0; i < NUM_ROWS_PER_CYCLE; i++, numRows++) {
                for (const auto& table : tables) {
                    auto res = conn->query(stringFormat("CREATE (a:{} {id: {}, name: 'name_{}'});",
                        table, numRows, numRows));
                    ASSERT_TRUE(res->isSuccess()) << res->getErrorMessage();
                }
            }
            if (cycle < NUM_CYCLES - 1) {
                ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
            }
        }
        auto inStoragePhase = false;
        FlakyCheckpointer tellingCheckpointer([&](main::ClientContext& context) {
            return std::make_unique<CheckpointerTellingItsStoragePhase>(context, inStoragePhase);
        });
        tellingCheckpointer.setCheckpointer(*getClientContext(*conn));
        // La k-ième allocation à partir d'ici est refusée.
        bufferManager->reserveCount = 0;
        failureFrequency = k;
        const auto checkpointed = conn->query("CHECKPOINT;")->isSuccess();
        const auto allocationWasRefused = failureFrequency != k;
        failureFrequency = UINT64_MAX;
        if (checkpointed && !allocationWasRefused) {
            // Moins de k allocations : le balayage a couvert tout le point de reprise.
            break;
        }
        if (!checkpointed) {
            numFailures++;
            if (inStoragePhase) {
                numFailuresInStoragePhase++;
            }
        }
        // La « panne » : on rouvre, sur un gestionnaire de tampons ordinaire.
        conn.reset();
        createDBAndConn();
        for (const auto& table : tables) {
            const auto where = stringFormat(" (panne à l'allocation {}, table {}{})", k, table,
                inStoragePhase ? ", pendant la phase de stockage" : "");
            ASSERT_EQ(single(stringFormat("MATCH (a:{}) RETURN COUNT(a);", table)), NUM_ROWS)
                << where;
            ASSERT_EQ(single(stringFormat("MATCH (a:{}) RETURN COUNT(DISTINCT a.id);", table)),
                NUM_ROWS)
                << where;
            ASSERT_EQ(single(stringFormat("MATCH (a:{}) RETURN SUM(a.id);", table)),
                NUM_ROWS * (NUM_ROWS - 1) / 2)
                << where;
            ASSERT_EQ(single(stringFormat("MATCH (a:{}) WHERE a.name <> 'name_' + "
                                          "cast(a.id AS STRING) RETURN COUNT(a);",
                          table)),
                0)
                << where;
            for (auto id = 0; id < NUM_ROWS; id++) {
                ASSERT_EQ(single(stringFormat("MATCH (a:{}) WHERE a.id = {} RETURN COUNT(a);",
                              table, id)),
                    1)
                    << "clé " << id << where;
            }
        }
    }
    // Un balayage qui n'aurait rien interrompu ne prouverait rien.
    EXPECT_GT(numFailuresInStoragePhase, 0u);
    std::cout << "  points de reprise interrompus : " << numFailures << ", dont "
              << numFailuresInStoragePhase << " pendant la phase de stockage" << std::endl;
}

class FlakyCheckpointerFailsOnWritingHeader final : public Checkpointer {
public:
    explicit FlakyCheckpointerFailsOnWritingHeader(main::ClientContext& context)
        : Checkpointer(context) {}

    void writeDatabaseHeader(const DatabaseHeader&) override {
        throw RuntimeException("checkpoint failed.");
    }
};

TEST_F(FlakyCheckpointerTest, RecoverFromCheckpointWriteHeaderFailure) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    auto initFlakyCheckpointer = [](main::ClientContext& context) {
        return std::make_unique<FlakyCheckpointerFailsOnWritingHeader>(context);
    };
    FlakyCheckpointer flakyCheckpointer(initFlakyCheckpointer);
    runTest(flakyCheckpointer);
}

class FlakyCheckpointerFailsOnFlushingShadow final : public Checkpointer {
public:
    explicit FlakyCheckpointerFailsOnFlushingShadow(main::ClientContext& context)
        : Checkpointer(context) {}

    void logCheckpointAndApplyShadowPages() override {
        // Simulate a failure during flushing the shadow pages.
        throw RuntimeException("checkpoint failed.");
    }
};

TEST_F(FlakyCheckpointerTest, RecoverFromCheckpointFlushingShadowFailure) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    auto initFlakyCheckpointer = [](main::ClientContext& context) {
        return std::make_unique<FlakyCheckpointerFailsOnFlushingShadow>(context);
    };
    FlakyCheckpointer flakyCheckpointer(initFlakyCheckpointer);
    runTest(flakyCheckpointer);
}

class FlakyCheckpointerFailsOnLoggingCheckpoint final : public Checkpointer {
public:
    explicit FlakyCheckpointerFailsOnLoggingCheckpoint(main::ClientContext& context)
        : Checkpointer(context) {}

    void logCheckpointAndApplyShadowPages() override {
        const auto storageManager = StorageManager::Get(clientContext);
        auto& shadowFile = storageManager->getShadowFile();
        // Flush the shadow file.
        shadowFile.flushAll(clientContext);
        // Simulate a failure during logging the checkpoint.
        throw RuntimeException("checkpoint failed.");
    }
};

TEST_F(FlakyCheckpointerTest, RecoverFromCheckpointLoggingCheckpointFailure) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    auto initFlakyCheckpointer = [](main::ClientContext& context) {
        return std::make_unique<FlakyCheckpointerFailsOnLoggingCheckpoint>(context);
    };
    FlakyCheckpointer flakyCheckpointer(initFlakyCheckpointer);
    runTest(flakyCheckpointer);
}

class FlakyCheckpointerFailsOnApplyingShadow final : public Checkpointer {
public:
    explicit FlakyCheckpointerFailsOnApplyingShadow(main::ClientContext& context)
        : Checkpointer(context) {}

    void logCheckpointAndApplyShadowPages() override {
        const auto storageManager = StorageManager::Get(clientContext);
        auto& shadowFile = storageManager->getShadowFile();
        // Flush the shadow file.
        shadowFile.flushAll(clientContext);
        auto wal = WAL::Get(clientContext);
        // Log the checkpoint to the WAL and flush WAL. This indicates that all shadow pages and
        // files (snapshots of catalog and metadata) have been written to disk. The part that is not
        // done is to replace them with the original pages or catalog and metadata files. If the
        // system crashes before this point, the WAL can still be used to recover the system to a
        // state where the checkpoint can be redone.
        wal->logAndFlushCheckpoint(&clientContext);
        // Simulate a failure during applying shadow pages.
        throw RuntimeException("checkpoint failed.");
    }
};

TEST_F(FlakyCheckpointerTest, RecoverFromCheckpointApplyingShadowFailure) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    auto initFlakyCheckpointer = [](main::ClientContext& context) {
        return std::make_unique<FlakyCheckpointerFailsOnApplyingShadow>(context);
    };
    FlakyCheckpointer flakyCheckpointer(initFlakyCheckpointer);
    runTest(flakyCheckpointer);
}

class FlakyCheckpointerFailsOnClearingFiles final : public Checkpointer {
public:
    explicit FlakyCheckpointerFailsOnClearingFiles(main::ClientContext& context)
        : Checkpointer(context) {}

    void logCheckpointAndApplyShadowPages() override {
        const auto storageManager = StorageManager::Get(clientContext);
        auto& shadowFile = storageManager->getShadowFile();
        // Flush the shadow file.
        shadowFile.flushAll(clientContext);
        auto wal = WAL::Get(clientContext);
        // Log the checkpoint to the WAL and flush WAL. This indicates that all shadow pages and
        // files (snapshots of catalog and metadata) have been written to disk. The part that is not
        // done is to replace them with the original pages or catalog and metadata files. If the
        // system crashes before this point, the WAL can still be used to recover the system to a
        // state where the checkpoint can be redone.
        wal->logAndFlushCheckpoint(&clientContext);
        shadowFile.applyShadowPages(clientContext);
        // Simulate a failure during clearing the WAL and shadow files.
        throw RuntimeException("checkpoint failed.");
    }
};

TEST_F(FlakyCheckpointerTest, RecoverFromCheckpointClearingFilesFailure) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    auto initFlakyCheckpointer = [](main::ClientContext& context) {
        return std::make_unique<FlakyCheckpointerFailsOnClearingFiles>(context);
    };
    FlakyCheckpointer flakyCheckpointer(initFlakyCheckpointer);
    runTest(flakyCheckpointer);
}

// Simulates a situation where a database attempts to replay a shadow file from an older database
// with the same path
TEST_F(FlakyCheckpointerTest, ShadowFileDatabaseIDMismatchExistingDB) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    auto initFlakyCheckpointer = [](main::ClientContext& context) {
        return std::make_unique<FlakyCheckpointerFailsOnClearingFiles>(context);
    };
    FlakyCheckpointer flakyCheckpointer(initFlakyCheckpointer);
    runFlakyCheckpoint(flakyCheckpointer);

    std::filesystem::remove(databasePath);

    // Temporarily rename the shadow file and wal file
    auto shadowFilePath = StorageUtils::getShadowFilePath(databasePath);
    auto walFilePath = StorageUtils::getWALFilePath(databasePath);
    auto tmpShadowFilePath = shadowFilePath + "1";
    auto tmpWALFilePath = walFilePath + "1";
    ASSERT_TRUE(std::filesystem::exists(shadowFilePath));
    ASSERT_TRUE(std::filesystem::exists(walFilePath));
    std::filesystem::rename(shadowFilePath, tmpShadowFilePath);
    std::filesystem::rename(walFilePath, tmpWALFilePath);

    // Recreate a new DB with the same path as before
    createDBAndConn();
    conn->query("CREATE NODE TABLE test(id INT64 PRIMARY KEY, name STRING);");

    // Close the DB
    conn.reset();
    database.reset();

    // Rename the files to the original names
    std::filesystem::rename(tmpShadowFilePath, shadowFilePath);
    std::filesystem::rename(tmpWALFilePath, walFilePath);

    // The shadow file replay should now fail
    EXPECT_THROW(createDBAndConn(), RuntimeException);
}

TEST_F(FlakyCheckpointerTest, ShadowFileDatabaseIDMismatchNewDB) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    auto initFlakyCheckpointer = [](main::ClientContext& context) {
        return std::make_unique<FlakyCheckpointerFailsOnClearingFiles>(context);
    };
    FlakyCheckpointer flakyCheckpointer(initFlakyCheckpointer);
    runFlakyCheckpoint(flakyCheckpointer);

    std::filesystem::remove(databasePath);

    // The shadow file replay should now fail
    EXPECT_THROW(createDBAndConn(), RuntimeException);
}

TEST_F(FlakyCheckpointerTest, ShadowFileDatabaseIDMismatchCorruptedDB) {
    if (inMemMode || systemConfig->checkpointThreshold == 0) {
        GTEST_SKIP();
    }
    auto initFlakyCheckpointer = [](main::ClientContext& context) {
        return std::make_unique<FlakyCheckpointerFailsOnClearingFiles>(context);
    };
    FlakyCheckpointer flakyCheckpointer(initFlakyCheckpointer);
    runFlakyCheckpoint(flakyCheckpointer);

    std::filesystem::remove(databasePath);

    // Create a new DB file and write garbage bytes to it
    std::ofstream ofs(databasePath);
    ofs << "1a1a1a1a1a1a1a1a1a1a";
    ofs.close();

    // The shadow file replay should now fail
    EXPECT_THROW(createDBAndConn(), InternalException);
}

} // namespace testing
} // namespace rag3db
