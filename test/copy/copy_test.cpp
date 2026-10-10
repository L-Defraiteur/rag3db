#include <atomic>

#include "catalog/catalog.h"
#include "catalog/catalog_entry/rel_group_catalog_entry.h"
#include "common/file_system/virtual_file_system.h"
#include "common/string_format.h"
#include "graph_test/base_graph_test.h"
#include "main/database.h"
#include "storage/buffer_manager/buffer_manager.h"
#include "storage/checkpointer.h"
#include "storage/storage_manager.h"
#include "storage/table/rel_table.h"
#include "test_helper/flaky_buffer_manager.h"
#include "test_runner/fsm_leak_checker.h"
#include "test_runner/test_parser.h"
#include "transaction/transaction_manager.h"

namespace rag3db {
namespace testing {

class CopyTestHelper {
public:
    static std::vector<std::unique_ptr<storage::FileHandle>>& getBMFileHandles(
        storage::BufferManager* bm) {
        return bm->fileHandles;
    }
};

struct BMExceptionRecoveryTestConfig {
    bool canFailDuringExecute;
    bool canFailDuringCheckpoint;
    bool canFailDuringCommit;
    std::function<void(main::Connection*)> initFunc;
    std::function<std::unique_ptr<main::QueryResult>(main::Connection*, int)> executeFunc;
    std::function<bool(main::QueryResult*)> earlyExitOnFailureFunc;
    std::function<std::unique_ptr<main::QueryResult>(main::Connection*)> checkFunc;
    uint64_t checkResult;
};

// Après un COPY dont le point de reprise a échoué, puis une réouverture : le COPY est
// atomique — soit rien n'en reste, soit tout y est (si aucune allocation n'a été refusée).
// S'il n'en reste rien, on le rejoue, et il doit tout rendre.
static std::unique_ptr<main::QueryResult> redoCopyIfItLeftNothing(main::Connection* conn,
    const std::string& countQuery, const std::string& copyQuery, int64_t fullCount) {
    auto result = conn->query(countQuery);
    EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
    const auto count = result->getNext()->getValue(0)->getValue<int64_t>();
    EXPECT_TRUE(count == 0 || count == fullCount) << "un COPY à moitié là : " << count;
    if (count == 0) {
        auto redone = conn->query(copyQuery);
        EXPECT_TRUE(redone->isSuccess()) << redone->getErrorMessage();
    }
    return conn->query(countQuery);
}

class CopyTest : public BaseGraphTest {
public:
    void TearDown() override {
        database.reset();
        conn.reset();
    }

    void SetUp() override {
        BaseGraphTest::SetUp();
        failureFrequency = 32;
    }

    void resetDB(uint64_t bufferPoolSize) {
        systemConfig->bufferPoolSize = bufferPoolSize;
        database.reset();
        conn.reset();
        createDBAndConn();
    }
    void resetDBFlaky(bool canFailDuringExecute = true, bool canFailDuringCheckpoint = true,
        bool canFailDuringCommit = true) {
        database.reset();
        conn.reset();
        systemConfig->bufferPoolSize = main::SystemConfig{}.bufferPoolSize;
        auto constructBMFunc = [&](const main::Database& db) {
            auto bm = std::unique_ptr<FlakyBufferManager>(new FlakyBufferManager(databasePath,
                databasePath + ".copy.tmp", systemConfig->bufferPoolSize, systemConfig->maxDBSize,
                getFileSystem(db), systemConfig->readOnly, failureFrequency, canFailDuringExecute,
                canFailDuringCheckpoint, canFailDuringCommit));
            currentBM = bm.get();
            return bm;
        };
        database = BaseGraphTest::constructDB(databasePath, *systemConfig, constructBMFunc);
        conn = std::make_unique<main::Connection>(database.get());
        currentBM->setClientContext(conn->getClientContext());
    }
    std::string getInputDir() override { KU_UNREACHABLE; }
    void BMExceptionRecoveryTest(BMExceptionRecoveryTestConfig cfg);
    void relCopyBMExceptionRecovery(bool journaled);
    void relCopyRefusedOnAPage(bool journaled, uint64_t pageToRefuse);
    std::atomic<uint64_t> failureFrequency;
    FlakyBufferManager* currentBM;
};

void CopyTest::BMExceptionRecoveryTest(BMExceptionRecoveryTestConfig cfg) {
    if (inMemMode) {
        failureFrequency = UINT64_MAX;
        resetDBFlaky(cfg.canFailDuringExecute, cfg.canFailDuringCheckpoint,
            cfg.canFailDuringCommit);
    } else {
        createDBAndConn();
    }

    cfg.initFunc(conn.get());

    if (!inMemMode) {
        resetDBFlaky(cfg.canFailDuringExecute, cfg.canFailDuringCheckpoint,
            cfg.canFailDuringCommit);
    }

    for (int i = 0;; i++) {
        ASSERT_LT(i, 20);
        auto result = cfg.executeFunc(conn.get(), i);
        if (!result->isSuccess()) {
            if (cfg.earlyExitOnFailureFunc(result.get())) {
                break;
            }
            ASSERT_EQ(result->getErrorMessage(), "Buffer manager exception: Unable to allocate "
                                                 "memory! The buffer pool is full and no "
                                                 "memory could be freed!");
        } else {
            break;
        }
    }

    if (inMemMode) {
        failureFrequency = UINT64_MAX;
    } else {
        // Reopen the DB so no spurious errors occur during the query
        resetDB(TestHelper::DEFAULT_BUFFER_POOL_SIZE_FOR_TESTING);
    }
    {
        // Test that the table copied as expected after the query
        auto result = cfg.checkFunc(conn.get());
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        ASSERT_TRUE(result->hasNext());
        ASSERT_EQ(cfg.checkResult, result->getNext()->getValue(0)->getValue<int64_t>());
    }

    // TODO(Royi) prevent leaking of allocated pages during checkpoint rollback
    if (!inMemMode && !cfg.canFailDuringCheckpoint) {
        FSMLeakChecker::checkForLeakedPages(conn.get());
    }
}

TEST_F(CopyTest, NodeCopyBMExceptionRecoverySameConnection) {
    if (inMemMode) {
        GTEST_SKIP();
    }
    BMExceptionRecoveryTestConfig cfg{.canFailDuringExecute = true,
        .canFailDuringCheckpoint = false,
        .canFailDuringCommit = false,
        .initFunc =
            [](main::Connection* conn) {
                conn->query("CREATE NODE TABLE account(ID INT64, PRIMARY KEY(ID))");
            },
        .executeFunc =
            [](main::Connection* conn, int) {
                const auto queryString = common::stringFormat(
                    "COPY account FROM \"{}/dataset/snap/twitter/csv/twitter-nodes.csv\"",
                    RAG3DB_ROOT_DIRECTORY);

                return conn->query(queryString);
            },
        .earlyExitOnFailureFunc = [](main::QueryResult*) { return false; },
        .checkFunc =
            [](main::Connection* conn) { return conn->query("MATCH (a:account) RETURN COUNT(*)"); },
        .checkResult = 81306};
    BMExceptionRecoveryTest(cfg);
}

// Après des COPY échoués par manque de mémoire, la table doit savoir combien de lignes elle
// porte. Un COPY réserve ses décalages avant d'écrire ses lignes ; quand l'écriture échouait,
// l'annulation ne retirait du compte que les lignes réellement écrites, et le compte restait
// trop haut d'un lot par échec. La ligne insérée ensuite par le chemin ordinaire prenait son
// décalage d'après ce compte.
TEST_F(CopyTest, RowCountIsRightAfterCopiesThatRanOutOfMemory) {
    if (inMemMode) {
        GTEST_SKIP();
    }
    createDBAndConn();
    conn->query("CREATE NODE TABLE account(ID INT64, PRIMARY KEY(ID))");
    resetDBFlaky(true /* canFailDuringExecute */, false, false);
    const auto copy = common::stringFormat(
        "COPY account FROM \"{}/dataset/snap/twitter/csv/twitter-nodes.csv\"",
        RAG3DB_ROOT_DIRECTORY);
    auto numFailures = 0;
    for (auto i = 0;; i++) {
        ASSERT_LT(i, 20);
        auto result = conn->query(copy);
        if (result->isSuccess()) {
            break;
        }
        numFailures++;
    }
    if (numFailures == 0) {
        GTEST_SKIP() << "no COPY ran out of memory in this run";
    }
    // Plus de pannes ; la même session continue.
    failureFrequency = UINT64_MAX;
    const auto single = [&](const std::string& query) -> int64_t {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    };
    EXPECT_EQ(single("MATCH (a:account) RETURN count(*);"), 81306);
    ASSERT_TRUE(conn->query("CREATE (:account {ID: 999999999});")->isSuccess());
    // La ligne neuve suit la dernière ligne du COPY.
    EXPECT_EQ(single("MATCH (a:account {ID: 999999999}) RETURN offset(id(a));"), 81306)
        << numFailures << " copies ran out of memory before the one that passed";
    EXPECT_EQ(single("MATCH (a:account) RETURN count(*);"), 81307);
    EXPECT_EQ(single("MATCH (a:account) RETURN max(offset(id(a)));"), 81306);
    ASSERT_TRUE(conn->query("CHECKPOINT;")->isSuccess());
    EXPECT_EQ(single("MATCH (a:account {ID: 999999999}) RETURN count(*);"), 1);
    resetDB(TestHelper::DEFAULT_BUFFER_POOL_SIZE_FOR_TESTING);
    EXPECT_EQ(single("MATCH (a:account) RETURN count(*);"), 81307);
    EXPECT_EQ(single("MATCH (a:account {ID: 999999999}) RETURN offset(id(a));"), 81306);
}

TEST_F(CopyTest, NodeCopyBMExceptionRecoverySameConnectionStringKey) {
    if (inMemMode) {
        GTEST_SKIP();
    }
    BMExceptionRecoveryTestConfig cfg{.canFailDuringExecute = true,
        .canFailDuringCheckpoint = false,
        .canFailDuringCommit = false,
        .initFunc =
            [](main::Connection* conn) {
                conn->query("CREATE NODE TABLE account(ID STRING, PRIMARY KEY(ID))");
            },
        .executeFunc =
            [](main::Connection* conn, int) {
                const auto queryString = common::stringFormat(
                    "COPY account FROM \"{}/dataset/snap/twitter/csv/twitter-nodes.csv\"",
                    RAG3DB_ROOT_DIRECTORY);

                return conn->query(queryString);
            },
        .earlyExitOnFailureFunc = [](main::QueryResult*) { return false; },
        .checkFunc =
            [](main::Connection* conn) { return conn->query("MATCH (a:account) RETURN COUNT(*)"); },
        .checkResult = 81306};
    BMExceptionRecoveryTest(cfg);
}

static void createTwitterAccounts(main::Connection* conn) {
    conn->query("CREATE NODE TABLE account(ID INT64, PRIMARY KEY(ID))");
    conn->query("CREATE REL TABLE follows(FROM account TO account);");
    ASSERT_TRUE(conn->query(
        common::stringFormat("COPY account FROM \"{}/dataset/snap/twitter/csv/twitter-nodes.csv\"",
            RAG3DB_ROOT_DIRECTORY)));
}

static std::string copyTwitterFollows() {
    return common::stringFormat(
        "COPY follows FROM '{}/dataset/snap/twitter/csv/twitter-edges.csv' (DELIM=' ')",
        RAG3DB_ROOT_DIRECTORY);
}

static constexpr int64_t NUM_TWITTER_FOLLOWS = 2420766;
static constexpr auto COUNT_TWITTER_FOLLOWS =
    "MATCH (a:account)-[:follows]->(b:account) RETURN COUNT(*)";
static constexpr auto BUFFER_POOL_FULL = "Buffer manager exception: Unable to allocate memory! "
                                         "The buffer pool is full and no memory could be freed!";

static void expectFollowsCount(main::Connection* conn, int64_t expected) {
    auto result = conn->query(COUNT_TWITTER_FOLLOWS);
    ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
    ASSERT_TRUE(result->hasNext());
    ASSERT_EQ(expected, result->getNext()->getValue(0)->getValue<int64_t>());
}

// Le partitionnement réserve beaucoup ; l'insertion en masse vient ensuite, à la fin du COPY.
// Pour qu'un refus tombe dans l'insertion en masse, la fréquence se règle sur le nombre de
// réservations du COPY, compté une fois sans refus dans une transaction annulée : chacun des
// premiers essais refuse une réservation un peu avant la fin, le suivant passe. Une pente fixe
// (512 × (i + 15), au plus 17 408) ne le permet pas : journalisé, le COPY réserve en plus une
// page de 4 Kio par page de son journal local (~44 000 ici, pour ~9 700 au défaut), et le test
// refusait vingt essais de suite dans le partitionnement.
void CopyTest::relCopyBMExceptionRecovery(bool journaled) {
    static constexpr int NUM_REFUSED_TRIES = 4;
    static constexpr uint64_t RESERVATIONS_BEFORE_THE_END_STEP = 256;
    storage::RelTable* follows = nullptr;
    uint64_t numReservationsOfACopy = 0;
    common::offset_t relOffsetAtTryStart = 0;
    std::atomic<uint64_t> refusalsAfterPartitioning = 0;
    BMExceptionRecoveryTestConfig cfg{.canFailDuringExecute = true,
        .canFailDuringCheckpoint = false,
        .canFailDuringCommit = false,
        .initFunc = createTwitterAccounts,
        .executeFunc =
            [&](main::Connection* conn, int i) -> std::unique_ptr<main::QueryResult> {
                failureFrequency = UINT64_MAX;
                if (journaled) {
                    conn->query("CALL force_checkpoint_on_copy=false");
                }
                if (i == 0) {
                    conn->query("BEGIN TRANSACTION");
                    const auto context = conn->getClientContext();
                    const auto entry = catalog::Catalog::Get(*context)->getTableCatalogEntry(
                        transaction::Transaction::Get(*context), "follows");
                    const auto oid =
                        entry->ptrCast<catalog::RelGroupCatalogEntry>()->getSingleRelEntryInfo().oid;
                    follows = &storage::StorageManager::Get(*context)
                                   ->getTable(oid)
                                   ->cast<storage::RelTable>();
                    // Le partitionneur réserve les identités de toutes les relations avant
                    // l'insertion en masse : un refus qui les trouve toutes réservées tombe
                    // après le partitionnement. reserveRelOffsets(0) lit le compte sous son
                    // verrou sans rien réserver.
                    currentBM->onRefusal = [&]() {
                        if (follows->reserveRelOffsets(0) - relOffsetAtTryStart ==
                            static_cast<common::offset_t>(NUM_TWITTER_FOLLOWS)) {
                            refusalsAfterPartitioning++;
                        }
                    };
                    currentBM->numReservations = 0;
                    auto calibration = conn->query(copyTwitterFollows());
                    numReservationsOfACopy = currentBM->numReservations;
                    conn->query("ROLLBACK");
                    if (!calibration->isSuccess()) {
                        return calibration;
                    }
                }
                failureFrequency =
                    i < NUM_REFUSED_TRIES ?
                        numReservationsOfACopy -
                            RESERVATIONS_BEFORE_THE_END_STEP * (NUM_REFUSED_TRIES - i) :
                        2 * numReservationsOfACopy;
                currentBM->reserveCount = 0;
                relOffsetAtTryStart = follows->reserveRelOffsets(0);
                return conn->query(copyTwitterFollows());
            },
        .earlyExitOnFailureFunc =
            [this](main::QueryResult*) {
                // clear the BM so that the failure frequency isn't messed with by cached pages
                for (auto& fh : CopyTestHelper::getBMFileHandles(currentBM)) {
                    currentBM->removeFilePagesFromFrames(*fh);
                }
                currentBM->removeEvictedCandidates();
                return false;
            },
        .checkFunc = [](main::Connection* conn) { return conn->query(COUNT_TWITTER_FOLLOWS); },
        .checkResult = NUM_TWITTER_FOLLOWS};
    BMExceptionRecoveryTest(cfg);
    EXPECT_GT(refusalsAfterPartitioning.load(), 0u)
        << "aucun refus n'est tombé dans l'insertion en masse (" << numReservationsOfACopy
        << " réservations pour un COPY)";
}

TEST_F(CopyTest, RelCopyBMExceptionRecoverySameConnection) {
    if (inMemMode ||
        common::StorageConfig::NODE_GROUP_SIZE_LOG2 != TestParser::STANDARD_NODE_GROUP_SIZE_LOG_2) {
        GTEST_SKIP();
    }
    relCopyBMExceptionRecovery(false /* journaled */);
}

TEST_F(CopyTest, JournaledRelCopyBMExceptionRecoverySameConnection) {
    if (inMemMode ||
        common::StorageConfig::NODE_GROUP_SIZE_LOG2 != TestParser::STANDARD_NODE_GROUP_SIZE_LOG_2) {
        GTEST_SKIP();
    }
    relCopyBMExceptionRecovery(true /* journaled */);
}

// Le contrat : un COPY de relations refusé pour mémoire sur une réservation d'une page de 4 Kio
// — journalisé, une page de son journal local — est annulé en entier. Rien n'en reste, ni dans
// la connexion ni après une réouverture ; le COPY suivant passe, et aucune page n'est perdue.
void CopyTest::relCopyRefusedOnAPage(bool journaled, uint64_t pageToRefuse) {
    createDBAndConn();
    createTwitterAccounts(conn.get());
    failureFrequency = UINT64_MAX;
    resetDBFlaky(true /* canFailDuringExecute */, false /* canFailDuringCheckpoint */,
        false /* canFailDuringCommit */);
    if (journaled) {
        ASSERT_TRUE(conn->query("CALL force_checkpoint_on_copy=false")->isSuccess());
    }
    currentBM->onlyFailPageReservations = true;
    currentBM->reserveCount = 0;
    failureFrequency = pageToRefuse;
    auto refused = conn->query(copyTwitterFollows());
    failureFrequency = UINT64_MAX;
    currentBM->onlyFailPageReservations = false;
    ASSERT_FALSE(refused->isSuccess());
    ASSERT_EQ(refused->getErrorMessage(), BUFFER_POOL_FULL);
    expectFollowsCount(conn.get(), 0);

    // Le tampon par défaut, celui de la doublure : le tampon des tests (≈ 73 Mio) ne tient pas
    // ce COPY, journalisé ou non.
    resetDB(main::SystemConfig{}.bufferPoolSize);
    expectFollowsCount(conn.get(), 0);
    if (journaled) {
        ASSERT_TRUE(conn->query("CALL force_checkpoint_on_copy=false")->isSuccess());
    }
    auto copied = conn->query(copyTwitterFollows());
    ASSERT_TRUE(copied->isSuccess()) << copied->getErrorMessage();
    expectFollowsCount(conn.get(), NUM_TWITTER_FOLLOWS);
    FSMLeakChecker::checkForLeakedPages(conn.get());
}

TEST_F(CopyTest, RelCopyRefusedOnAPageRollsBack) {
    if (inMemMode) {
        GTEST_SKIP();
    }
    relCopyRefusedOnAPage(false /* journaled */, 500);
}

TEST_F(CopyTest, JournaledRelCopyRefusedOnAJournalPageRollsBack) {
    if (inMemMode) {
        GTEST_SKIP();
    }
    relCopyRefusedOnAPage(true /* journaled */, 5000);
}

TEST_F(CopyTest, NodeInsertBMExceptionDuringCommitRecovery) {
    static constexpr uint64_t numValues = 200000;
    BMExceptionRecoveryTestConfig cfg{.canFailDuringExecute = false,
        .canFailDuringCheckpoint = false,
        .canFailDuringCommit = false,
        .initFunc =
            [this](main::Connection* conn) {
                conn->query("CREATE NODE TABLE account(ID INT64, PRIMARY KEY(ID))");
                failureFrequency = 128;
            },
        .executeFunc =
            [](main::Connection* conn, int) {
                const auto queryString = common::stringFormat(
                    "UNWIND RANGE(1,{}) AS i CREATE (a:account {ID:i})", numValues);
                return conn->query(queryString);
            },
        .earlyExitOnFailureFunc = [](main::QueryResult*) { return false; },
        .checkFunc =
            [](main::Connection* conn) { return conn->query("MATCH (a:account) RETURN COUNT(*)"); },
        .checkResult = numValues};
    BMExceptionRecoveryTest(cfg);
}

TEST_F(CopyTest, RelInsertBMExceptionDuringCommitRecovery) {
    static constexpr auto numNodes = 10000;
    BMExceptionRecoveryTestConfig cfg{.canFailDuringExecute = false,
        .canFailDuringCheckpoint = false,
        .canFailDuringCommit = true,
        .initFunc =
            [this](main::Connection* conn) {
                conn->query("CREATE NODE TABLE account(ID INT64, PRIMARY KEY(ID))");
                conn->query("CREATE REL TABLE follows(FROM account TO account);");
                const auto queryString = common::stringFormat(
                    "UNWIND RANGE(1,{}) AS i CREATE (a:account {ID:i})", numNodes);
                ASSERT_TRUE(conn->query(queryString)->isSuccess());
                failureFrequency = 32;
            },
        .executeFunc =
            [](main::Connection* conn, int) {
                return conn->query(common::stringFormat(
                    "UNWIND RANGE(1,{}) AS i MATCH (a:account), (b:account) WHERE a.ID = i AND "
                    "b.ID = i + 1 CREATE (a)-[f:follows]->(b)",
                    numNodes));
            },
        .earlyExitOnFailureFunc = [](main::QueryResult*) { return false; },
        .checkFunc =
            [](main::Connection* conn) {
                return conn->query("MATCH (a)-[f:follows]->(b) RETURN COUNT(*)");
            },
        .checkResult = numNodes - 1};
    BMExceptionRecoveryTest(cfg);
}

TEST_F(CopyTest, NodeCopyBMExceptionDuringCheckpointRecovery) {
    if (inMemMode || systemConfig->forceCheckpointOnClose == false) {
        GTEST_SKIP();
    }
    static constexpr bool canFailDuringExecute = false;
    static constexpr bool canFailDuringCheckpoint = true;
    BMExceptionRecoveryTestConfig cfg{.canFailDuringExecute = canFailDuringExecute,
        .canFailDuringCheckpoint = canFailDuringCheckpoint,
        .canFailDuringCommit = false,
        .initFunc =
            [this](main::Connection* conn) {
                conn->query("CREATE NODE TABLE account(ID STRING, PRIMARY KEY(ID))");
                failureFrequency = 512;
            },
        .executeFunc =
            [](main::Connection* conn, int) {
                return conn->query(common::stringFormat(
                    "COPY account FROM \"{}/dataset/snap/twitter/csv/twitter-nodes.csv\"",
                    RAG3DB_ROOT_DIRECTORY));
            },
        .earlyExitOnFailureFunc =
            [this](main::QueryResult*) {
                // make sure the checkpoint when closing the DB doesn't fail
                failureFrequency = UINT64_MAX;
                return true;
            },
        .checkFunc =
            [](main::Connection* conn) {
                // Un COPY n'est pas dans le journal : sa durabilité est son point de reprise.
                // Celui-ci a échoué, l'instruction a rendu une erreur, et la base fermée
                // ensuite ne tente plus de point de reprise : rouverte, il ne reste RIEN du
                // COPY, et le rejouer rend tout. Avant, c'est le point de reprise de
                // fermeture qui sauvait ces lignes, sur un état en mémoire qui n'est plus sûr.
                return redoCopyIfItLeftNothing(conn, "MATCH (a:account) RETURN COUNT(*)",
                    common::stringFormat(
                        "COPY account FROM \"{}/dataset/snap/twitter/csv/twitter-nodes.csv\"",
                        RAG3DB_ROOT_DIRECTORY),
                    81306);
            },
        .checkResult = 81306};
    BMExceptionRecoveryTest(cfg);
}

TEST_F(CopyTest, RelCopyCheckpointBMExceptionRecovery) {
    if (inMemMode || systemConfig->forceCheckpointOnClose == false) {
        GTEST_SKIP();
    }
    BMExceptionRecoveryTestConfig cfg{.canFailDuringExecute = false,
        .canFailDuringCheckpoint = true,
        .canFailDuringCommit = false,
        .initFunc =
            [this](main::Connection* conn) {
                conn->query("CREATE NODE TABLE account(ID INT64, PRIMARY KEY(ID))");
                conn->query("CREATE REL TABLE follows(FROM account TO account);");
                ASSERT_TRUE(conn->query(common::stringFormat(
                    "COPY account FROM \"{}/dataset/snap/twitter/csv/twitter-nodes.csv\"",
                    RAG3DB_ROOT_DIRECTORY)));
                failureFrequency = 1024;
            },
        .executeFunc =
            [](main::Connection* conn, int) {
                return conn->query(common::stringFormat(
                    "COPY follows FROM '{}/dataset/snap/twitter/csv/twitter-edges.csv' (DELIM=' ')",
                    RAG3DB_ROOT_DIRECTORY));
            },
        .earlyExitOnFailureFunc =
            [this](main::QueryResult*) {
                // make sure the checkpoint when closing the DB doesn't fail
                failureFrequency = UINT64_MAX;
                return true;
            },
        .checkFunc =
            [this](main::Connection*) {
                // Même contrat que NodeCopyBMExceptionDuringCheckpointRecovery. Rejouer ce
                // COPY demande la mémoire tampon par défaut, pas celle, petite, des tests.
                resetDB(main::SystemConfig{}.bufferPoolSize);
                return redoCopyIfItLeftNothing(conn.get(),
                    "MATCH (a:account)-[:follows]->(b:account) RETURN COUNT(*)",
                    common::stringFormat("COPY follows FROM "
                                         "'{}/dataset/snap/twitter/csv/twitter-edges.csv' "
                                         "(DELIM=' ')",
                        RAG3DB_ROOT_DIRECTORY),
                    2420766);
            },
        .checkResult = 2420766};
    BMExceptionRecoveryTest(cfg);
}

TEST_F(CopyTest, NodeInsertBMExceptionDuringCheckpointRecovery) {
    if (inMemMode ||
        common::StorageConfig::NODE_GROUP_SIZE_LOG2 != TestParser::STANDARD_NODE_GROUP_SIZE_LOG_2) {
        GTEST_SKIP();
    }
    static constexpr uint64_t numValues = 200000;
    static constexpr bool canFailDuringExecute = false;
    static constexpr bool canFailDuringCheckpoint = true;
    BMExceptionRecoveryTestConfig cfg{.canFailDuringExecute = canFailDuringExecute,
        .canFailDuringCheckpoint = canFailDuringCheckpoint,
        .canFailDuringCommit = false,
        .initFunc =
            [this](main::Connection* conn) {
                failureFrequency = 512;
                conn->query("CREATE NODE TABLE account(ID INT64, PRIMARY KEY(ID))");
            },
        .executeFunc =
            [](main::Connection* conn, int) {
                return conn->query(common::stringFormat(
                    "UNWIND RANGE(1,{}) AS i CREATE (a:account {ID:i})", numValues));
            },
        .earlyExitOnFailureFunc = [](main::QueryResult*) { return true; },
        .checkFunc =
            [](main::Connection* conn) { return conn->query("MATCH (a:account) RETURN COUNT(*)"); },
        .checkResult = numValues};
    BMExceptionRecoveryTest(cfg);
}

TEST_F(CopyTest, GracefulBMExceptionHandlingManyThreads) {
#if defined(__has_feature)
#if __has_feature(thread_sanitizer)
    // This test runs too slowly with TSAN enabled since it uses 32 threads and is already slow
    GTEST_SKIP();
#endif
#endif
    systemConfig->maxNumThreads = 32;
    resetDB(TestHelper::DEFAULT_BUFFER_POOL_SIZE_FOR_TESTING);

    static constexpr uint32_t repeatCount = 10;
    for (uint32_t i = 0; i < repeatCount; ++i) {
        conn->query("create node table Comment (id int64, creationDate INT64, locationIP STRING, "
                    "browserUsed STRING, content STRING, length INT32, PRIMARY KEY (id))");
        auto result = conn->query(
            common::stringFormat("COPY Comment FROM ['{}/dataset/ldbc-sf01/Comment.csv', "
                                 "'{}/dataset/ldbc-sf01/Comment.csv'] (delim='|', header=true, "
                                 "parallel=false)",
                RAG3DB_ROOT_DIRECTORY, RAG3DB_ROOT_DIRECTORY));
        ASSERT_FALSE(result->isSuccess());
        conn->query("drop table Comment");
    }

    if (!inMemMode) {
        FSMLeakChecker::checkForLeakedPages(conn.get());
    }
}

TEST_F(CopyTest, OutOfMemoryRecovery) {
    if (inMemMode ||
        common::StorageConfig::NODE_GROUP_SIZE_LOG2 != TestParser::STANDARD_NODE_GROUP_SIZE_LOG_2) {
        GTEST_SKIP();
    }
    // Needs to be small enough that we cannot successfully complete the rel table copy
    resetDB(64 * 1024 * 1024 + TestHelper::HASH_INDEX_MEM / 5);
    conn->query("CREATE NODE TABLE account(ID INT64, PRIMARY KEY(ID))");
    conn->query("CREATE REL TABLE follows(FROM account TO account);");
    {
        auto result = conn->query(common::stringFormat(
            "COPY account FROM \"{}/dataset/snap/twitter/csv/twitter-nodes.csv\"",
            RAG3DB_ROOT_DIRECTORY));
        ASSERT_TRUE(result->isSuccess()) << result->toString();

        result = conn->query(common::stringFormat(
            "COPY follows FROM '{}/dataset/snap/twitter/csv/twitter-edges.csv' (DELIM=' ')",
            RAG3DB_ROOT_DIRECTORY));
        ASSERT_FALSE(result->isSuccess());
        ASSERT_EQ(result->getErrorMessage(),
            "Buffer manager exception: Unable to allocate memory! The buffer pool is full and no "
            "memory could be freed!");
    }
    // Try opening then closing the database
    resetDB(256 * 1024 * 1024 + TestHelper::HASH_INDEX_MEM);
    // Try again with a larger buffer pool size
    resetDB(256 * 1024 * 1024 + TestHelper::HASH_INDEX_MEM);
    {
        auto result = conn->query(common::stringFormat(
            "COPY follows FROM '{}/dataset/snap/twitter/csv/twitter-edges.csv' (DELIM=' ')",
            RAG3DB_ROOT_DIRECTORY));
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        // Test that the table copied as expected after the query
        result = conn->query("MATCH (a:account)-[:follows]->(b:account) RETURN COUNT(*)");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        ASSERT_TRUE(result->hasNext());
        ASSERT_EQ(result->getNext()->getValue(0)->getValue<int64_t>(), 2420766);
    }
}

TEST_F(CopyTest, OutOfMemoryRecoveryDropTable) {
    if (inMemMode ||
        common::StorageConfig::NODE_GROUP_SIZE_LOG2 != TestParser::STANDARD_NODE_GROUP_SIZE_LOG_2) {
        GTEST_SKIP();
    }
    // Needs to be small enough that we cannot successfully complete the rel table copy
    resetDB(64 * 1024 * 1024 + TestHelper::HASH_INDEX_MEM / 5);
    conn->query("CREATE NODE TABLE account(ID INT64, PRIMARY KEY(ID))");
    conn->query("CREATE REL TABLE follows(FROM account TO account);");
    {
        auto result = conn->query(common::stringFormat(
            "COPY account FROM \"{}/dataset/snap/twitter/csv/twitter-nodes.csv\"",
            RAG3DB_ROOT_DIRECTORY));
        ASSERT_TRUE(result->isSuccess()) << result->toString();
        result = conn->query(common::stringFormat(
            "COPY follows FROM '{}/dataset/snap/twitter/csv/twitter-edges.csv' (DELIM=' ')",
            RAG3DB_ROOT_DIRECTORY));
        ASSERT_FALSE(result->isSuccess());
        ASSERT_EQ(result->getErrorMessage(), "Buffer manager exception: Unable to allocate "
                                             "memory! The buffer pool is full and no "
                                             "memory could be freed!");
    }
    // Try dropping the table before trying again with a larger buffer pool size
    resetDB(256 * 1024 * 1024 + TestHelper::HASH_INDEX_MEM);
    {
        auto result = conn->query("DROP TABLE follows;");
        ASSERT_TRUE(result->isSuccess()) << result->toString();
        result = conn->query("CREATE REL TABLE follows(FROM account TO account);");
        ASSERT_TRUE(result->isSuccess()) << result->toString();
        result = conn->query(common::stringFormat(
            "COPY follows FROM '{}/dataset/snap/twitter/csv/twitter-edges.csv' (DELIM=' ')",
            RAG3DB_ROOT_DIRECTORY));
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        // Test that the table copied as expected after the query
        result = conn->query("MATCH (a:account)-[:follows]->(b:account) RETURN COUNT(*)");
        ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
        ASSERT_TRUE(result->hasNext());
        ASSERT_EQ(result->getNext()->getValue(0)->getValue<int64_t>(), 2420766);
    }
}
} // namespace testing
} // namespace rag3db
