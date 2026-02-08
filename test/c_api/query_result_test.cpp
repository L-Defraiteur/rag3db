#include <fstream>

#include "c_api_test/c_api_test.h"

using namespace rag3db::main;
using namespace rag3db::common;
using namespace rag3db::processor;
using namespace rag3db::testing;

class CApiQueryResultTest : public CApiTest {
public:
    std::string getInputDir() override {
        return TestHelper::appendRag3dbRootPath("dataset/tinysnb/");
    }
};

static rag3db_value* copy_flat_tuple(rag3db_flat_tuple* tuple, uint32_t tupleLen) {
    rag3db_value* ret = (rag3db_value*)malloc(sizeof(rag3db_value) * tupleLen);
    for (uint32_t i = 0; i < tupleLen; i++) {
        rag3db_flat_tuple_get_value(tuple, i, &ret[i]);
    }
    return ret;
}

TEST_F(CApiQueryResultTest, GetNextExample) {
    auto conn = getConnection();

    rag3db_query_result result;
    rag3db_connection_query(conn, "MATCH (p:person) RETURN p.*", &result);

    uint64_t num_tuples = rag3db_query_result_get_num_tuples(&result);
    rag3db_value** tuples = (rag3db_value**)malloc(sizeof(rag3db_value*) * num_tuples);
    for (uint64_t i = 0; i < num_tuples; ++i) {
        rag3db_flat_tuple tuple;
        rag3db_query_result_get_next(&result, &tuple);
        tuples[i] = copy_flat_tuple(&tuple, rag3db_query_result_get_num_columns(&result));
        rag3db_flat_tuple_destroy(&tuple);
    }

    for (uint64_t i = 0; i < num_tuples; ++i) {
        for (uint64_t j = 0; j < rag3db_query_result_get_num_columns(&result); ++j) {
            ASSERT_FALSE(rag3db_value_is_null(&tuples[i][j]));
            rag3db_value_destroy(&tuples[i][j]);
        }
        free(tuples[i]);
    }

    free((void*)tuples);

    rag3db_query_result_destroy(&result);
}

TEST_F(CApiQueryResultTest, GetErrorMessage) {
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, "MATCH (a:person) RETURN COUNT(*)", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    char* errorMessage = rag3db_query_result_get_error_message(&result);
    rag3db_query_result_destroy(&result);

    state = rag3db_connection_query(connection, "MATCH (a:personnnn) RETURN COUNT(*)", &result);
    ASSERT_EQ(state, Rag3dbError);
    ASSERT_FALSE(rag3db_query_result_is_success(&result));
    errorMessage = rag3db_query_result_get_error_message(&result);
    ASSERT_EQ(std::string(errorMessage), "Binder exception: Table personnnn does not exist.");
    rag3db_query_result_destroy(&result);
    rag3db_destroy_string(errorMessage);
}

TEST_F(CApiQueryResultTest, ToString) {
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, "MATCH (a:person) RETURN COUNT(*)", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    char* str_repr = rag3db_query_result_to_string(&result);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_destroy_string(str_repr);
    rag3db_query_result_destroy(&result);
}

TEST_F(CApiQueryResultTest, GetNumColumns) {
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, "MATCH (a:person) RETURN a.fName, a.age, a.height",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_EQ(rag3db_query_result_get_num_columns(&result), 3);
    rag3db_query_result_destroy(&result);
}

TEST_F(CApiQueryResultTest, GetColumnName) {
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, "MATCH (a:person) RETURN a.fName, a.age, a.height",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    char* columnName;
    ASSERT_EQ(rag3db_query_result_get_column_name(&result, 0, &columnName), Rag3dbSuccess);
    ASSERT_EQ(std::string(columnName), "a.fName");
    rag3db_destroy_string(columnName);
    ASSERT_EQ(rag3db_query_result_get_column_name(&result, 1, &columnName), Rag3dbSuccess);
    ASSERT_EQ(std::string(columnName), "a.age");
    rag3db_destroy_string(columnName);
    ASSERT_EQ(rag3db_query_result_get_column_name(&result, 2, &columnName), Rag3dbSuccess);
    ASSERT_EQ(std::string(columnName), "a.height");
    rag3db_destroy_string(columnName);
    ASSERT_EQ(rag3db_query_result_get_column_name(&result, 222, &columnName), Rag3dbError);
    rag3db_query_result_destroy(&result);
}

TEST_F(CApiQueryResultTest, GetColumnDataType) {
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, "MATCH (a:person) RETURN a.fName, a.age, a.height",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    rag3db_logical_type type;
    ASSERT_EQ(rag3db_query_result_get_column_data_type(&result, 0, &type), Rag3dbSuccess);
    auto typeCpp = (LogicalType*)(type._data_type);
    ASSERT_EQ(typeCpp->getLogicalTypeID(), LogicalTypeID::STRING);
    rag3db_data_type_destroy(&type);
    ASSERT_EQ(rag3db_query_result_get_column_data_type(&result, 1, &type), Rag3dbSuccess);
    typeCpp = (LogicalType*)(type._data_type);
    ASSERT_EQ(typeCpp->getLogicalTypeID(), LogicalTypeID::INT64);
    rag3db_data_type_destroy(&type);
    ASSERT_EQ(rag3db_query_result_get_column_data_type(&result, 2, &type), Rag3dbSuccess);
    typeCpp = (LogicalType*)(type._data_type);
    ASSERT_EQ(typeCpp->getLogicalTypeID(), LogicalTypeID::FLOAT);
    rag3db_data_type_destroy(&type);
    ASSERT_EQ(rag3db_query_result_get_column_data_type(&result, 222, &type), Rag3dbError);
    rag3db_query_result_destroy(&result);
}

// TODO(Guodong): Fix this test by adding support of STRUCT in arrow table export.
// TEST_F(CApiQueryResultTest, GetArrowSchema) {
//    auto connection = getConnection();
//    auto result = rag3db_connection_query(
//        connection, "MATCH (p:person)-[k:knows]-(q:person) RETURN p.fName, k, q.fName");
//    ASSERT_TRUE(rag3db_query_result_is_success(result));
//    auto schema = rag3db_query_result_get_arrow_schema(result);
//    ASSERT_STREQ(schema.name, "rag3db_query_result");
//    ASSERT_EQ(schema.n_children, 3);
//    ASSERT_STREQ(schema.children[0]->name, "p.fName");
//    ASSERT_STREQ(schema.children[1]->name, "k");
//    ASSERT_STREQ(schema.children[2]->name, "q.fName");
//
//    schema.release(&schema);
//    rag3db_query_result_destroy(result);
//}

TEST_F(CApiQueryResultTest, GetQuerySummary) {
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, "MATCH (a:person) RETURN a.fName, a.age, a.height",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    rag3db_query_summary summary;
    state = rag3db_query_result_get_query_summary(&result, &summary);
    ASSERT_EQ(state, Rag3dbSuccess);
    auto compilingTime = rag3db_query_summary_get_compiling_time(&summary);
    ASSERT_GT(compilingTime, 0);
    auto executionTime = rag3db_query_summary_get_execution_time(&summary);
    ASSERT_GT(executionTime, 0);
    rag3db_query_summary_destroy(&summary);
    rag3db_query_result_destroy(&result);
}

TEST_F(CApiQueryResultTest, GetNext) {
    rag3db_query_result result;
    rag3db_flat_tuple row;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        "MATCH (a:person) RETURN a.fName, a.age ORDER BY a.fName", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));

    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &row);
    ASSERT_EQ(state, Rag3dbSuccess);
    auto flatTupleCpp = (FlatTuple*)(row._flat_tuple);
    ASSERT_EQ(flatTupleCpp->getValue(0)->getValue<std::string>(), "Alice");
    ASSERT_EQ(flatTupleCpp->getValue(1)->getValue<int64_t>(), 35);

    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &row);
    ASSERT_EQ(state, Rag3dbSuccess);
    flatTupleCpp = (FlatTuple*)(row._flat_tuple);
    ASSERT_EQ(flatTupleCpp->getValue(0)->getValue<std::string>(), "Bob");
    ASSERT_EQ(flatTupleCpp->getValue(1)->getValue<int64_t>(), 30);
    rag3db_flat_tuple_destroy(&row);

    while (rag3db_query_result_has_next(&result)) {
        rag3db_query_result_get_next(&result, &row);
    }
    ASSERT_FALSE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &row);
    ASSERT_EQ(state, Rag3dbError);
    rag3db_query_result_destroy(&result);
}

TEST_F(CApiQueryResultTest, ResetIterator) {
    rag3db_query_result result;
    rag3db_flat_tuple row;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        "MATCH (a:person) RETURN a.fName, a.age ORDER BY a.fName", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));

    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &row);
    ASSERT_EQ(state, Rag3dbSuccess);
    auto flatTupleCpp = (FlatTuple*)(row._flat_tuple);
    ASSERT_EQ(flatTupleCpp->getValue(0)->getValue<std::string>(), "Alice");
    ASSERT_EQ(flatTupleCpp->getValue(1)->getValue<int64_t>(), 35);

    rag3db_query_result_reset_iterator(&result);

    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &row);
    ASSERT_EQ(state, Rag3dbSuccess);
    flatTupleCpp = (FlatTuple*)(row._flat_tuple);
    ASSERT_EQ(flatTupleCpp->getValue(0)->getValue<std::string>(), "Alice");
    ASSERT_EQ(flatTupleCpp->getValue(1)->getValue<int64_t>(), 35);
    rag3db_flat_tuple_destroy(&row);

    rag3db_query_result_destroy(&result);
}

TEST_F(CApiQueryResultTest, MultipleQuery) {
    rag3db_query_result result;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, "return 1; return 2; return 3;", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));

    char* str = rag3db_query_result_to_string(&result);
    ASSERT_EQ(std::string(str), "1\n1\n");
    rag3db_destroy_string(str);

    ASSERT_TRUE(rag3db_query_result_has_next_query_result(&result));
    rag3db_query_result next_query_result;
    ASSERT_EQ(rag3db_query_result_get_next_query_result(&result, &next_query_result), Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&next_query_result));
    str = rag3db_query_result_to_string(&next_query_result);
    ASSERT_EQ(std::string(str), "2\n2\n");
    rag3db_destroy_string(str);
    rag3db_query_result_destroy(&next_query_result);

    ASSERT_EQ(rag3db_query_result_get_next_query_result(&result, &next_query_result), Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&next_query_result));
    str = rag3db_query_result_to_string(&next_query_result);
    ASSERT_EQ(std::string(str), "3\n3\n");
    rag3db_destroy_string(str);

    ASSERT_FALSE(rag3db_query_result_has_next_query_result(&result));
    ASSERT_EQ(rag3db_query_result_get_next_query_result(&result, &next_query_result), Rag3dbError);
    rag3db_query_result_destroy(&next_query_result);

    rag3db_query_result_destroy(&result);
}
