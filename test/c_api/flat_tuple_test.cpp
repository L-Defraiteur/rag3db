#include "c_api_test/c_api_test.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::testing;

class CApiFlatTupleTest : public CApiTest {
public:
    std::string getInputDir() override {
        return TestHelper::appendRag3dbRootPath("dataset/tinysnb/");
    }
};

TEST_F(CApiFlatTupleTest, GetValue) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        "MATCH (a:person) RETURN a.fName, a.age, a.height ORDER BY a.fName LIMIT 1", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_NE(value._value, nullptr);
    auto valueCpp = static_cast<Value*>(value._value);
    ASSERT_NE(valueCpp, nullptr);
    ASSERT_EQ(valueCpp->getDataType().getLogicalTypeID(), LogicalTypeID::STRING);
    ASSERT_EQ(valueCpp->getValue<std::string>(), "Alice");
    rag3db_value_destroy(&value);
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 1, &value), Rag3dbSuccess);
    ASSERT_NE(value._value, nullptr);
    valueCpp = static_cast<Value*>(value._value);
    ASSERT_NE(valueCpp, nullptr);
    ASSERT_EQ(valueCpp->getDataType().getLogicalTypeID(), LogicalTypeID::INT64);
    ASSERT_EQ(valueCpp->getValue<int64_t>(), 35);
    rag3db_value_destroy(&value);
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 2, &value), Rag3dbSuccess);
    ASSERT_NE(value._value, nullptr);
    valueCpp = static_cast<Value*>(value._value);
    ASSERT_NE(valueCpp, nullptr);
    ASSERT_EQ(valueCpp->getDataType().getLogicalTypeID(), LogicalTypeID::FLOAT);
    ASSERT_FLOAT_EQ(valueCpp->getValue<float>(), 1.731);
    rag3db_value_destroy(&value);
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 222, &value), Rag3dbError);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
}

TEST_F(CApiFlatTupleTest, ToString) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        "MATCH (a:person) RETURN a.fName, a.age, a.height ORDER BY a.fName LIMIT 1", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    auto columnWidths = (uint32_t*)malloc(3 * sizeof(uint32_t));
    columnWidths[0] = 10;
    columnWidths[1] = 5;
    columnWidths[2] = 10;
    char* str = rag3db_flat_tuple_to_string(&flatTuple);
    ASSERT_EQ(std::string(str), "Alice|35|1.731000\n");
    rag3db_destroy_string(str);
    free(columnWidths);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
}
