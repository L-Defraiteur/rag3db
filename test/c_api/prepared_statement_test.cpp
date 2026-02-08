#include "c_api_test/c_api_test.h"

using namespace rag3db::main;
using namespace rag3db::testing;

class CApiPreparedStatementTest : public CApiTest {
public:
    std::string getInputDir() override {
        return TestHelper::appendRag3dbRootPath("dataset/tinysnb/");
    }
};

TEST_F(CApiPreparedStatementTest, IsSuccess) {
    rag3db_prepared_statement preparedStatement;
    rag3db_state state;
    auto connection = getConnection();
    auto query = "MATCH (a:person) WHERE a.isStudent = $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(preparedStatement._prepared_statement, nullptr);
    ASSERT_TRUE(rag3db_prepared_statement_is_success(&preparedStatement));
    rag3db_prepared_statement_destroy(&preparedStatement);

    query = "MATCH (a:personnnn) WHERE a.isStudent = $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(preparedStatement._prepared_statement, nullptr);
    ASSERT_FALSE(rag3db_prepared_statement_is_success(&preparedStatement));
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, GetErrorMessage) {
    rag3db_prepared_statement preparedStatement;
    rag3db_state state;
    auto connection = getConnection();
    auto query = "MATCH (a:person) WHERE a.isStudent = $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(preparedStatement._prepared_statement, nullptr);
    ASSERT_EQ(rag3db_prepared_statement_get_error_message(&preparedStatement), nullptr);
    rag3db_prepared_statement_destroy(&preparedStatement);

    query = "MATCH (a:personnnn) WHERE a.isStudent = $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(preparedStatement._prepared_statement, nullptr);
    char* message = rag3db_prepared_statement_get_error_message(&preparedStatement);
    ASSERT_EQ(std::string(message), "Binder exception: Table personnnn does not exist.");
    rag3db_prepared_statement_destroy(&preparedStatement);
    rag3db_destroy_string(message);
}

TEST_F(CApiPreparedStatementTest, BindBool) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query = "MATCH (a:person) WHERE a.isStudent = $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_bool(&preparedStatement, "1", true), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 3);
    rag3db_query_result_destroy(&result);
    // Bind a different parameter
    ASSERT_EQ(rag3db_prepared_statement_bind_bool(&preparedStatement, "1", false), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    resultCpp = static_cast<QueryResult*>(result._query_result);
    tuple = resultCpp->getNext();
    value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 5);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindInt64) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query = "MATCH (a:person) WHERE a.age > $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_int64(&preparedStatement, "1", 30), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 4);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindInt32) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query = "MATCH (a:movies) WHERE a.length > $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_int32(&preparedStatement, "1", 200), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 2);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindInt16) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query =
        "MATCH (a:person) -[s:studyAt]-> (b:organisation) WHERE s.length > $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_int16(&preparedStatement, "1", 10), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 2);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindInt8) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query =
        "MATCH (a:person) -[s:studyAt]-> (b:organisation) WHERE s.level > $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_int8(&preparedStatement, "1", 3), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 2);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindUInt64) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query =
        "MATCH (a:person) -[s:studyAt]-> (b:organisation) WHERE s.code > $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(rag3db_prepared_statement_bind_uint64(&preparedStatement, "1", 100), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 2);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindUInt32) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query =
        "MATCH (a:person) -[s:studyAt]-> (b:organisation) WHERE s.temperature> $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_uint32(&preparedStatement, "1", 10), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 2);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindUInt16) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query =
        "MATCH (a:person) -[s:studyAt]-> (b:organisation) WHERE s.ulength> $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_uint16(&preparedStatement, "1", 100), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 2);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindUInt8) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query =
        "MATCH (a:person) -[s:studyAt]-> (b:organisation) WHERE s.ulevel> $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_uint8(&preparedStatement, "1", 14), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 2);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindDouble) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query = "MATCH (a:person) WHERE a.eyeSight > $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_double(&preparedStatement, "1", 4.5), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 7);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindFloat) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query = "MATCH (a:person) WHERE a.height < $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_float(&preparedStatement, "1", 1.0), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 1);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindString) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query = "MATCH (a:person) WHERE a.fName = $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_prepared_statement_is_success(&preparedStatement));
    ASSERT_EQ(rag3db_prepared_statement_bind_string(&preparedStatement, "1", "Alice"), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 1);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindDate) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query = "MATCH (a:person) WHERE a.birthdate > $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_prepared_statement_is_success(&preparedStatement));
    auto date = rag3db_date_t{0};
    ASSERT_EQ(rag3db_prepared_statement_bind_date(&preparedStatement, "1", date), Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 4);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindTimestamp) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query = "MATCH (a:person) WHERE a.registerTime > $1 and cast(a.registerTime, "
                 "\"timestamp_ns\") > $2 and cast(a.registerTime, \"timestamp_ms\") > "
                 "$3 and cast(a.registerTime, \"timestamp_sec\") > $4 and cast(a.registerTime, "
                 "\"timestamp_tz\") > $5 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_prepared_statement_is_success(&preparedStatement));
    auto timestamp = rag3db_timestamp_t{0};
    auto timestamp_ns = rag3db_timestamp_ns_t{1};
    auto timestamp_ms = rag3db_timestamp_ms_t{2};
    auto timestamp_sec = rag3db_timestamp_sec_t{3};
    auto timestamp_tz = rag3db_timestamp_tz_t{4};
    ASSERT_EQ(rag3db_prepared_statement_bind_timestamp(&preparedStatement, "1", timestamp),
        Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_timestamp_ns(&preparedStatement, "2", timestamp_ns),
        Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_timestamp_ms(&preparedStatement, "3", timestamp_ms),
        Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_timestamp_sec(&preparedStatement, "4", timestamp_sec),
        Rag3dbSuccess);
    ASSERT_EQ(rag3db_prepared_statement_bind_timestamp_tz(&preparedStatement, "5", timestamp_tz),
        Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 7);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindInteval) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query = "MATCH (a:person) WHERE a.lastJobDuration > $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_prepared_statement_is_success(&preparedStatement));
    auto interval = rag3db_interval_t{0, 0, 0};
    ASSERT_EQ(rag3db_prepared_statement_bind_interval(&preparedStatement, "1", interval),
        Rag3dbSuccess);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 8);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}

TEST_F(CApiPreparedStatementTest, BindValue) {
    rag3db_prepared_statement preparedStatement;
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    auto query = "MATCH (a:person) WHERE a.registerTime > $1 RETURN COUNT(*)";
    state = rag3db_connection_prepare(connection, query, &preparedStatement);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_prepared_statement_is_success(&preparedStatement));
    auto timestamp = rag3db_timestamp_t{0};
    auto timestampValue = rag3db_value_create_timestamp(timestamp);
    ASSERT_EQ(rag3db_prepared_statement_bind_value(&preparedStatement, "1", timestampValue),
        Rag3dbSuccess);
    rag3db_value_destroy(timestampValue);
    state = rag3db_connection_execute(connection, &preparedStatement, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_NE(result._query_result, nullptr);
    ASSERT_EQ(rag3db_query_result_get_num_tuples(&result), 1);
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 1);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    auto resultCpp = static_cast<QueryResult*>(result._query_result);
    auto tuple = resultCpp->getNext();
    auto value = tuple->getValue(0)->getValue<int64_t>();
    ASSERT_EQ(value, 7);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&preparedStatement);
}
