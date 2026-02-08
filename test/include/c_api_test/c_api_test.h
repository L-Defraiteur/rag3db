#pragma once

#include "c_api/rag3db.h"
#include "graph_test/base_graph_test.h"

namespace rag3db {
namespace testing {

// This class starts database in on-disk mode.
class APIDBTest : public BaseGraphTest {
public:
    void SetUp() override {
        BaseGraphTest::SetUp();
        createDBAndConn();
        initGraph();
    }
};

class CApiTest : public APIDBTest {
public:
    rag3db_database _database;
    rag3db_connection connection;

    void SetUp() override {
        APIDBTest::SetUp();
        auto* connCppPointer = conn.release();
        auto* databaseCppPointer = database.release();
        connection = rag3db_connection{connCppPointer};
        _database = rag3db_database{databaseCppPointer};
    }

    std::string getDatabasePath() { return databasePath; }

    rag3db_database* getDatabase() { return &_database; }

    rag3db_connection* getConnection() { return &connection; }

    void TearDown() override {
        rag3db_connection_destroy(&connection);
        rag3db_database_destroy(&_database);
        APIDBTest::TearDown();
    }
};

} // namespace testing
} // namespace rag3db
