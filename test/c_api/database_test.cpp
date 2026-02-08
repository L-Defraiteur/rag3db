#include "c_api/rag3db.h"
#include "graph_test/base_graph_test.h"
#include "gtest/gtest.h"

using namespace rag3db::main;
using namespace rag3db::testing;

// This class starts database without initializing graph.
class APIEmptyDBTest : public BaseGraphTest {
    std::string getInputDir() override { KU_UNREACHABLE; }
};

class CApiDatabaseTest : public APIEmptyDBTest {
public:
    void SetUp() override {
        APIEmptyDBTest::SetUp();
        defaultSystemConfig = rag3db_default_system_config();

        // limit memory usage by keeping max number of threads small
        defaultSystemConfig.max_num_threads = 2;
        auto maxDBSizeEnv = TestHelper::getSystemEnv("MAX_DB_SIZE");
        if (!maxDBSizeEnv.empty()) {
            defaultSystemConfig.max_db_size = std::stoull(maxDBSizeEnv);
        }
    }

    rag3db_system_config defaultSystemConfig;
};

TEST_F(CApiDatabaseTest, CreationAndDestroy) {
    rag3db_database database;
    rag3db_state state;
    auto databasePathCStr = databasePath.c_str();
    auto systemConfig = defaultSystemConfig;
    state = rag3db_database_init(databasePathCStr, systemConfig, &database);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(database._database, nullptr);
    auto databaseCpp = static_cast<Database*>(database._database);
    ASSERT_NE(databaseCpp, nullptr);
    rag3db_database_destroy(&database);
}

TEST_F(CApiDatabaseTest, CreationReadOnly) {
    rag3db_database database;
    rag3db_connection connection;
    rag3db_query_result queryResult;
    rag3db_state state;
    auto databasePathCStr = databasePath.c_str();
    auto systemConfig = defaultSystemConfig;
    // First, create a read-write database.
    state = rag3db_database_init(databasePathCStr, systemConfig, &database);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(database._database, nullptr);
    auto databaseCpp = static_cast<Database*>(database._database);
    ASSERT_NE(databaseCpp, nullptr);
    rag3db_database_destroy(&database);
    // Now, access the same database read-only.
    systemConfig.read_only = true;
    state = rag3db_database_init(databasePathCStr, systemConfig, &database);
    if (databasePath == "" || databasePath == ":memory:") {
        ASSERT_EQ(state, Rag3dbError);
        ASSERT_EQ(database._database, nullptr);
        return;
    }
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(database._database, nullptr);
    databaseCpp = static_cast<Database*>(database._database);
    ASSERT_NE(databaseCpp, nullptr);
    // Try to write to the database.
    state = rag3db_connection_init(&database, &connection);
    ASSERT_EQ(state, Rag3dbSuccess);
    state = rag3db_connection_query(&connection,
        "CREATE NODE TABLE User(name STRING, age INT64, reg_date DATE, PRIMARY KEY (name))",
        &queryResult);
    ASSERT_EQ(state, Rag3dbError);
    ASSERT_FALSE(rag3db_query_result_is_success(&queryResult));
    rag3db_query_result_destroy(&queryResult);
    rag3db_connection_destroy(&connection);
    rag3db_database_destroy(&database);
}

TEST_F(CApiDatabaseTest, CreationInMemory) {
    rag3db_database database;
    rag3db_state state;
    auto databasePathCStr = (char*)"";
    state = rag3db_database_init(databasePathCStr, defaultSystemConfig, &database);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_database_destroy(&database);
    databasePathCStr = (char*)":memory:";
    state = rag3db_database_init(databasePathCStr, defaultSystemConfig, &database);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_database_destroy(&database);
}

#ifndef __WASM__ // home directory is not available in WASM
TEST_F(CApiDatabaseTest, CreationHomeDir) {
    rag3db_database database;
    rag3db_connection connection;
    rag3db_state state;
    auto databasePathCStr = (char*)"~/ku_test.db";
    state = rag3db_database_init(databasePathCStr, defaultSystemConfig, &database);
    ASSERT_EQ(state, Rag3dbSuccess);
    state = rag3db_connection_init(&database, &connection);
    ASSERT_EQ(state, Rag3dbSuccess);
    auto homePath =
        getClientContext(*(Connection*)(connection._connection))->getClientConfig()->homeDirectory;
    rag3db_connection_destroy(&connection);
    rag3db_database_destroy(&database);
    std::filesystem::remove_all(homePath + "/ku_test.db");
}
#endif

TEST_F(CApiDatabaseTest, CloseQueryResultAndConnectionAfterDatabaseDestroy) {
    rag3db_database database;
    auto databasePathCStr = (char*)":memory:";
    auto systemConfig = rag3db_default_system_config();
    systemConfig.buffer_pool_size = 10 * 1024 * 1024; // 10MB
    systemConfig.max_db_size = 1 << 30;               // 1GB
    systemConfig.max_num_threads = 2;
    rag3db_state state = rag3db_database_init(databasePathCStr, systemConfig, &database);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(database._database, nullptr);
    rag3db_connection conn;
    rag3db_query_result queryResult;
    state = rag3db_connection_init(&database, &conn);
    ASSERT_EQ(state, Rag3dbSuccess);
    state = rag3db_connection_query(&conn, "RETURN 1+1", &queryResult);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&queryResult));
    rag3db_flat_tuple tuple;
    rag3db_state resultState = rag3db_query_result_get_next(&queryResult, &tuple);
    ASSERT_EQ(resultState, Rag3dbSuccess);
    rag3db_value value;
    rag3db_state valueState = rag3db_flat_tuple_get_value(&tuple, 0, &value);
    ASSERT_EQ(valueState, Rag3dbSuccess);
    int64_t valueInt = INT64_MAX;
    rag3db_state valueIntState = rag3db_value_get_int64(&value, &valueInt);
    ASSERT_EQ(valueIntState, Rag3dbSuccess);
    ASSERT_EQ(valueInt, 2);
    // Destroy database first, this should not crash
    rag3db_database_destroy(&database);
    // Call rag3db_connection_query should not crash, but return an error
    state = rag3db_connection_query(&conn, "RETURN 1+1", &queryResult);
    ASSERT_EQ(state, Rag3dbError);
    // Call rag3db_query_result_get_next should not crash, but return an error
    resultState = rag3db_query_result_get_next(&queryResult, &tuple);
    ASSERT_EQ(resultState, Rag3dbError);
    // Now destroy everything, this should not crash
    rag3db_query_result_destroy(&queryResult);
    rag3db_connection_destroy(&conn);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&tuple);
}

TEST_F(CApiDatabaseTest, UseConnectionAfterDatabaseDestroy) {
    rag3db_database db;
    rag3db_connection conn;
    rag3db_query_result result;

    auto systemConfig = rag3db_default_system_config();
    systemConfig.buffer_pool_size = 10 * 1024 * 1024; // 10MB
    systemConfig.max_db_size = 1 << 30;               // 1GB
    systemConfig.max_num_threads = 2;
    auto state = rag3db_database_init("", systemConfig, &db);
    ASSERT_EQ(state, Rag3dbSuccess);
    state = rag3db_connection_init(&db, &conn);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_database_destroy(&db);
    state = rag3db_connection_query(&conn, "RETURN 0", &result);
    ASSERT_EQ(state, Rag3dbError);

    rag3db_connection_destroy(&conn);
}
