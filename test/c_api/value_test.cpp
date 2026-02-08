#include "c_api_test/c_api_test.h"

using namespace rag3db::main;
using namespace rag3db::common;
using namespace rag3db::testing;

class CApiValueTest : public CApiTest {
public:
    std::string getInputDir() override {
        return TestHelper::appendRag3dbRootPath("dataset/tinysnb/");
    }
};

TEST(CApiValueTestEmptyDB, CreateNull) {
    rag3db_value* value = rag3db_value_create_null();
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::ANY);
    ASSERT_EQ(cppValue->isNull(), true);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateNullWithDatatype) {
    rag3db_logical_type type;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_INT64, nullptr, 0, &type);
    rag3db_value* value = rag3db_value_create_null_with_data_type(&type);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    rag3db_data_type_destroy(&type);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::INT64);
    ASSERT_EQ(cppValue->isNull(), true);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, IsNull) {
    rag3db_value* value = rag3db_value_create_int64(123);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(value));
    rag3db_value_destroy(value);
    value = rag3db_value_create_null();
    ASSERT_TRUE(rag3db_value_is_null(value));
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, SetNull) {
    rag3db_value* value = rag3db_value_create_int64(123);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(value));
    rag3db_value_set_null(value, true);
    ASSERT_TRUE(rag3db_value_is_null(value));
    rag3db_value_set_null(value, false);
    ASSERT_FALSE(rag3db_value_is_null(value));
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateDefault) {
    rag3db_logical_type type;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_INT64, nullptr, 0, &type);
    rag3db_value* value = rag3db_value_create_default(&type);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    rag3db_data_type_destroy(&type);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_FALSE(rag3db_value_is_null(value));
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::INT64);
    ASSERT_EQ(cppValue->getValue<int64_t>(), 0);
    rag3db_value_destroy(value);

    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_STRING, nullptr, 0, &type);
    value = rag3db_value_create_default(&type);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    rag3db_data_type_destroy(&type);
    cppValue = static_cast<Value*>(value->_value);
    ASSERT_FALSE(rag3db_value_is_null(value));
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::STRING);
    ASSERT_EQ(cppValue->getValue<std::string>(), "");
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateBool) {
    rag3db_value* value = rag3db_value_create_bool(true);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::BOOL);
    ASSERT_EQ(cppValue->getValue<bool>(), true);
    rag3db_value_destroy(value);

    value = rag3db_value_create_bool(false);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::BOOL);
    ASSERT_EQ(cppValue->getValue<bool>(), false);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateInt8) {
    rag3db_value* value = rag3db_value_create_int8(12);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::INT8);
    ASSERT_EQ(cppValue->getValue<int8_t>(), 12);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateInt16) {
    rag3db_value* value = rag3db_value_create_int16(123);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::INT16);
    ASSERT_EQ(cppValue->getValue<int16_t>(), 123);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateInt32) {
    rag3db_value* value = rag3db_value_create_int32(123);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::INT32);
    ASSERT_EQ(cppValue->getValue<int32_t>(), 123);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateInt64) {
    rag3db_value* value = rag3db_value_create_int64(123);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::INT64);
    ASSERT_EQ(cppValue->getValue<int64_t>(), 123);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateUInt8) {
    rag3db_value* value = rag3db_value_create_uint8(12);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::UINT8);
    ASSERT_EQ(cppValue->getValue<uint8_t>(), 12);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateUInt16) {
    rag3db_value* value = rag3db_value_create_uint16(123);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::UINT16);
    ASSERT_EQ(cppValue->getValue<uint16_t>(), 123);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateUInt32) {
    rag3db_value* value = rag3db_value_create_uint32(123);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::UINT32);
    ASSERT_EQ(cppValue->getValue<uint32_t>(), 123);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateUInt64) {
    rag3db_value* value = rag3db_value_create_uint64(123);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::UINT64);
    ASSERT_EQ(cppValue->getValue<uint64_t>(), 123);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateINT128) {
    rag3db_value* value = rag3db_value_create_int128(rag3db_int128_t{211111111, 100000000});
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::INT128);
    auto cppTimeStamp = cppValue->getValue<int128_t>();
    ASSERT_EQ(cppTimeStamp.high, 100000000);
    ASSERT_EQ(cppTimeStamp.low, 211111111);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateFloat) {
    rag3db_value* value = rag3db_value_create_float(123.456);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::FLOAT);
    ASSERT_FLOAT_EQ(cppValue->getValue<float>(), 123.456);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateDouble) {
    rag3db_value* value = rag3db_value_create_double(123.456);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::DOUBLE);
    ASSERT_DOUBLE_EQ(cppValue->getValue<double>(), 123.456);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateInternalID) {
    auto internalID = rag3db_internal_id_t{1, 123};
    rag3db_value* value = rag3db_value_create_internal_id(internalID);
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::INTERNAL_ID);
    auto internalIDCpp = cppValue->getValue<internalID_t>();
    ASSERT_EQ(internalIDCpp.tableID, 1);
    ASSERT_EQ(internalIDCpp.offset, 123);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateDate) {
    rag3db_value* value = rag3db_value_create_date(rag3db_date_t{123});
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::DATE);
    auto cppDate = cppValue->getValue<date_t>();
    ASSERT_EQ(cppDate.days, 123);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateTimeStamp) {
    rag3db_value* value = rag3db_value_create_timestamp(rag3db_timestamp_t{123});
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::TIMESTAMP);
    auto cppTimeStamp = cppValue->getValue<timestamp_t>();
    ASSERT_EQ(cppTimeStamp.value, 123);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateTimeStampNonStandard) {
    rag3db_value* value_ns = rag3db_value_create_timestamp_ns(rag3db_timestamp_ns_t{12345});
    rag3db_value* value_ms = rag3db_value_create_timestamp_ms(rag3db_timestamp_ms_t{123456});
    rag3db_value* value_sec = rag3db_value_create_timestamp_sec(rag3db_timestamp_sec_t{1234567});
    rag3db_value* value_tz = rag3db_value_create_timestamp_tz(rag3db_timestamp_tz_t{12345678});

    ASSERT_FALSE(value_ns->_is_owned_by_cpp);
    ASSERT_FALSE(value_ms->_is_owned_by_cpp);
    ASSERT_FALSE(value_sec->_is_owned_by_cpp);
    ASSERT_FALSE(value_tz->_is_owned_by_cpp);
    auto cppValue_ns = static_cast<Value*>(value_ns->_value);
    auto cppValue_ms = static_cast<Value*>(value_ms->_value);
    auto cppValue_sec = static_cast<Value*>(value_sec->_value);
    auto cppValue_tz = static_cast<Value*>(value_tz->_value);
    ASSERT_EQ(cppValue_ns->getDataType().getLogicalTypeID(), LogicalTypeID::TIMESTAMP_NS);
    ASSERT_EQ(cppValue_ms->getDataType().getLogicalTypeID(), LogicalTypeID::TIMESTAMP_MS);
    ASSERT_EQ(cppValue_sec->getDataType().getLogicalTypeID(), LogicalTypeID::TIMESTAMP_SEC);
    ASSERT_EQ(cppValue_tz->getDataType().getLogicalTypeID(), LogicalTypeID::TIMESTAMP_TZ);

    auto cppTimeStamp_ns = cppValue_ns->getValue<timestamp_ns_t>();
    auto cppTimeStamp_ms = cppValue_ms->getValue<timestamp_ms_t>();
    auto cppTimeStamp_sec = cppValue_sec->getValue<timestamp_sec_t>();
    auto cppTimeStamp_tz = cppValue_tz->getValue<timestamp_tz_t>();
    ASSERT_EQ(cppTimeStamp_ns.value, 12345);
    ASSERT_EQ(cppTimeStamp_ms.value, 123456);
    ASSERT_EQ(cppTimeStamp_sec.value, 1234567);
    ASSERT_EQ(cppTimeStamp_tz.value, 12345678);
    rag3db_value_destroy(value_ns);
    rag3db_value_destroy(value_ms);
    rag3db_value_destroy(value_sec);
    rag3db_value_destroy(value_tz);
}

TEST(CApiValueTestEmptyDB, CreateInterval) {
    rag3db_value* value = rag3db_value_create_interval(rag3db_interval_t{12, 3, 300});
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::INTERVAL);
    auto cppTimeStamp = cppValue->getValue<interval_t>();
    ASSERT_EQ(cppTimeStamp.months, 12);
    ASSERT_EQ(cppTimeStamp.days, 3);
    ASSERT_EQ(cppTimeStamp.micros, 300);
    rag3db_value_destroy(value);
}

TEST(CApiValueTestEmptyDB, CreateString) {
    rag3db_value* value = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::STRING);
    ASSERT_EQ(cppValue->getValue<std::string>(), "abcdefg");
    rag3db_value_destroy(value);
}

TEST_F(CApiValueTest, CreateList) {
    auto connection = getConnection();
    rag3db_value* value1 = rag3db_value_create_int64(123);
    rag3db_value* value2 = rag3db_value_create_int64(456);
    rag3db_value* value3 = rag3db_value_create_int64(789);
    rag3db_value* value4 = rag3db_value_create_int64(101112);
    rag3db_value* value5 = rag3db_value_create_int64(131415);
    rag3db_value* elements[] = {value1, value2, value3, value4, value5};
    rag3db_value* value = nullptr;
    rag3db_state state = rag3db_value_create_list(5, elements, &value);
    ASSERT_EQ(state, Rag3dbSuccess);
    // Destroy the original values, the list should still be valid
    for (int i = 0; i < 5; ++i) {
        rag3db_value_destroy(elements[i]);
    }
    ASSERT_FALSE(value->_is_owned_by_cpp);
    rag3db_prepared_statement stmt;
    state = rag3db_connection_prepare(connection, (char*)"RETURN $1", &stmt);
    ASSERT_EQ(state, Rag3dbSuccess);
    state = rag3db_prepared_statement_bind_value(&stmt, "1", value);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_query_result result;
    state = rag3db_connection_execute(connection, &stmt, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    rag3db_flat_tuple flatTuple;
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value outValue;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &outValue), Rag3dbSuccess);
    ASSERT_TRUE(outValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&outValue));
    uint64_t size;
    ASSERT_EQ(rag3db_value_get_list_size(&outValue, &size), Rag3dbSuccess);
    ASSERT_EQ(size, 5);
    rag3db_value listElement;
    ASSERT_EQ(rag3db_value_get_list_element(&outValue, 0, &listElement), Rag3dbSuccess);
    ASSERT_TRUE(listElement._is_owned_by_cpp);
    int64_t int64Result;
    ASSERT_EQ(rag3db_value_get_int64(&listElement, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 123);
    rag3db_value_destroy(&listElement);
    ASSERT_EQ(rag3db_value_get_list_element(&outValue, 1, &listElement), Rag3dbSuccess);
    ASSERT_TRUE(listElement._is_owned_by_cpp);
    ASSERT_EQ(rag3db_value_get_int64(&listElement, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 456);
    rag3db_value_destroy(&listElement);
    ASSERT_EQ(rag3db_value_get_list_element(&outValue, 2, &listElement), Rag3dbSuccess);
    ASSERT_TRUE(listElement._is_owned_by_cpp);
    ASSERT_EQ(rag3db_value_get_int64(&listElement, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 789);
    rag3db_value_destroy(&listElement);
    ASSERT_EQ(rag3db_value_get_list_element(&outValue, 3, &listElement), Rag3dbSuccess);
    ASSERT_TRUE(listElement._is_owned_by_cpp);
    ASSERT_EQ(rag3db_value_get_int64(&listElement, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 101112);
    rag3db_value_destroy(&listElement);
    ASSERT_EQ(rag3db_value_get_list_element(&outValue, 4, &listElement), Rag3dbSuccess);
    ASSERT_TRUE(listElement._is_owned_by_cpp);
    ASSERT_EQ(rag3db_value_get_int64(&listElement, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 131415);
    rag3db_value_destroy(&listElement);
    rag3db_value_destroy(&outValue);
    rag3db_value_destroy(value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&stmt);
}

TEST(CApiValueTestEmptyDB, CreateListDifferentTypes) {
    rag3db_value* value1 = rag3db_value_create_int64(123);
    rag3db_value* value2 = rag3db_value_create_string((char*)"abcdefg");
    rag3db_value* elements[] = {value1, value2};
    rag3db_value* value = nullptr;
    rag3db_state state = rag3db_value_create_list(2, elements, &value);
    ASSERT_EQ(state, Rag3dbError);
    rag3db_value_destroy(value1);
    rag3db_value_destroy(value2);
}

TEST(CApiValueTestEmptyDB, CreateListEmpty) {
    rag3db_value* elements[] = {nullptr}; // Must be non-empty
    rag3db_value* value = nullptr;
    rag3db_state state = rag3db_value_create_list(0, elements, &value);
    ASSERT_EQ(state, Rag3dbError);
}

TEST_F(CApiValueTest, CreateListNested) {
    auto connection = getConnection();
    rag3db_value* value1 = rag3db_value_create_int64(123);
    rag3db_value* value2 = rag3db_value_create_int64(456);
    rag3db_value* value3 = rag3db_value_create_int64(789);
    rag3db_value* value4 = rag3db_value_create_int64(101112);
    rag3db_value* value5 = rag3db_value_create_int64(131415);
    rag3db_value* elements1[] = {value1, value2, value3};
    rag3db_value* elements2[] = {value4, value5};
    rag3db_value* list1 = nullptr;
    rag3db_value* list2 = nullptr;
    rag3db_value_create_list(3, elements1, &list1);
    ASSERT_FALSE(list1->_is_owned_by_cpp);
    rag3db_value_create_list(2, elements2, &list2);
    ASSERT_FALSE(list2->_is_owned_by_cpp);
    rag3db_value* elements[] = {list1, list2};
    rag3db_value* nestedList = nullptr;
    rag3db_state state = rag3db_value_create_list(2, elements, &nestedList);
    ASSERT_EQ(state, Rag3dbSuccess);
    // Destroy the original values, the list should still be valid
    for (int i = 0; i < 3; ++i) {
        rag3db_value_destroy(elements1[i]);
    }
    for (int i = 0; i < 2; ++i) {
        rag3db_value_destroy(elements2[i]);
    }
    rag3db_value_destroy(list1);
    rag3db_value_destroy(list2);
    ASSERT_FALSE(nestedList->_is_owned_by_cpp);
    rag3db_prepared_statement stmt;
    state = rag3db_connection_prepare(connection, (char*)"RETURN $1", &stmt);
    ASSERT_EQ(state, Rag3dbSuccess);
    state = rag3db_prepared_statement_bind_value(&stmt, "1", nestedList);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_query_result result;
    state = rag3db_connection_execute(connection, &stmt, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    rag3db_flat_tuple flatTuple;
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value outValue;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &outValue), Rag3dbSuccess);
    ASSERT_TRUE(outValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&outValue));
    uint64_t size;
    ASSERT_EQ(rag3db_value_get_list_size(&outValue, &size), Rag3dbSuccess);
    ASSERT_EQ(size, 2);
    rag3db_value listElement;
    ASSERT_EQ(rag3db_value_get_list_element(&outValue, 0, &listElement), Rag3dbSuccess);
    ASSERT_TRUE(listElement._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&listElement));
    ASSERT_EQ(rag3db_value_get_list_size(&listElement, &size), Rag3dbSuccess);
    ASSERT_EQ(size, 3);
    rag3db_value innerListElement;
    ASSERT_EQ(rag3db_value_get_list_element(&listElement, 0, &innerListElement), Rag3dbSuccess);
    ASSERT_TRUE(innerListElement._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&innerListElement));
    int64_t int64Result;
    ASSERT_EQ(rag3db_value_get_int64(&innerListElement, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 123);
    rag3db_value_destroy(&innerListElement);
    ASSERT_EQ(rag3db_value_get_list_element(&listElement, 1, &innerListElement), Rag3dbSuccess);
    ASSERT_TRUE(innerListElement._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&innerListElement));
    ASSERT_EQ(rag3db_value_get_int64(&innerListElement, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 456);
    rag3db_value_destroy(&innerListElement);
    ASSERT_EQ(rag3db_value_get_list_element(&listElement, 2, &innerListElement), Rag3dbSuccess);
    ASSERT_TRUE(innerListElement._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&innerListElement));
    ASSERT_EQ(rag3db_value_get_int64(&innerListElement, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 789);
    rag3db_value_destroy(&innerListElement);
    rag3db_value_destroy(&listElement);
    ASSERT_EQ(rag3db_value_get_list_element(&outValue, 1, &listElement), Rag3dbSuccess);
    ASSERT_TRUE(listElement._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&listElement));
    ASSERT_EQ(rag3db_value_get_list_size(&listElement, &size), Rag3dbSuccess);
    ASSERT_EQ(size, 2);
    rag3db_value_destroy(&listElement);
    rag3db_value_destroy(&outValue);
    rag3db_value_destroy(nestedList);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&stmt);
}

TEST_F(CApiValueTest, CreateStruct) {
    auto connection = getConnection();
    rag3db_value* value1 = rag3db_value_create_int16(32);
    rag3db_value* value2 = rag3db_value_create_string((char*)"Wong");
    rag3db_value* value3 = rag3db_value_create_string((char*)"Kelley");
    rag3db_value* value4 = rag3db_value_create_int64(123456);
    rag3db_value* value5 = rag3db_value_create_string((char*)"CEO");
    rag3db_value* value6 = rag3db_value_create_bool(true);
    rag3db_value* employmentElements[] = {value5, value6};
    const char* employmentFieldNames[] = {(char*)"title", (char*)"is_current"};
    rag3db_value* employment = nullptr;
    rag3db_state state =
        rag3db_value_create_struct(2, employmentFieldNames, employmentElements, &employment);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_FALSE(employment->_is_owned_by_cpp);
    rag3db_value_destroy(value5);
    rag3db_value_destroy(value6);
    rag3db_value* personElements[] = {value1, value2, value3, value4, employment};
    const char* personFieldNames[] = {(char*)"age", (char*)"first_name", (char*)"last_name",
        (char*)"id", (char*)"employment"};
    rag3db_value* person = nullptr;
    state = rag3db_value_create_struct(5, personFieldNames, personElements, &person);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value_destroy(value1);
    rag3db_value_destroy(value2);
    rag3db_value_destroy(value3);
    rag3db_value_destroy(value4);
    rag3db_value_destroy(employment);
    ASSERT_FALSE(person->_is_owned_by_cpp);
    rag3db_prepared_statement stmt;
    state = rag3db_connection_prepare(connection, (char*)"RETURN $1", &stmt);
    ASSERT_EQ(state, Rag3dbSuccess);
    state = rag3db_prepared_statement_bind_value(&stmt, "1", person);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_query_result result;
    state = rag3db_connection_execute(connection, &stmt, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    rag3db_flat_tuple flatTuple;
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value outValue;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &outValue), Rag3dbSuccess);
    ASSERT_TRUE(outValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&outValue));
    uint64_t size;
    state = rag3db_value_get_struct_num_fields(&outValue, &size);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(size, 5);
    char* structFieldName;
    rag3db_value structFieldValue;
    state = rag3db_value_get_struct_field_name(&outValue, 0, &structFieldName);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_STREQ(structFieldName, "age");
    state = rag3db_value_get_struct_field_value(&outValue, 0, &structFieldValue);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(structFieldValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&structFieldValue));
    int16_t int16Result;
    state = rag3db_value_get_int16(&structFieldValue, &int16Result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(int16Result, 32);
    rag3db_value_destroy(&structFieldValue);
    rag3db_destroy_string(structFieldName);
    state = rag3db_value_get_struct_field_name(&outValue, 1, &structFieldName);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_STREQ(structFieldName, "first_name");
    state = rag3db_value_get_struct_field_value(&outValue, 1, &structFieldValue);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(structFieldValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&structFieldValue));
    char* stringResult;
    state = rag3db_value_get_string(&structFieldValue, &stringResult);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_STREQ(stringResult, "Wong");
    rag3db_value_destroy(&structFieldValue);
    rag3db_destroy_string(structFieldName);
    rag3db_destroy_string(stringResult);
    state = rag3db_value_get_struct_field_name(&outValue, 2, &structFieldName);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_STREQ(structFieldName, "last_name");
    state = rag3db_value_get_struct_field_value(&outValue, 2, &structFieldValue);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(structFieldValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&structFieldValue));
    state = rag3db_value_get_string(&structFieldValue, &stringResult);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_STREQ(stringResult, "Kelley");
    rag3db_value_destroy(&structFieldValue);
    rag3db_destroy_string(structFieldName);
    rag3db_destroy_string(stringResult);
    state = rag3db_value_get_struct_field_name(&outValue, 3, &structFieldName);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_STREQ(structFieldName, "id");
    state = rag3db_value_get_struct_field_value(&outValue, 3, &structFieldValue);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(structFieldValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&structFieldValue));
    int64_t int64Result;
    state = rag3db_value_get_int64(&structFieldValue, &int64Result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(int64Result, 123456);
    rag3db_value_destroy(&structFieldValue);
    rag3db_destroy_string(structFieldName);
    state = rag3db_value_get_struct_field_name(&outValue, 4, &structFieldName);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_STREQ(structFieldName, "employment");
    state = rag3db_value_get_struct_field_value(&outValue, 4, &structFieldValue);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(structFieldValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&structFieldValue));
    state = rag3db_value_get_struct_num_fields(&structFieldValue, &size);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(size, 2);
    char* employmentFieldName;
    rag3db_value employmentFieldValue;
    state = rag3db_value_get_struct_field_name(&structFieldValue, 0, &employmentFieldName);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_STREQ(employmentFieldName, "title");
    state = rag3db_value_get_struct_field_value(&structFieldValue, 0, &employmentFieldValue);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(employmentFieldValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&employmentFieldValue));
    state = rag3db_value_get_string(&employmentFieldValue, &stringResult);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_STREQ(stringResult, "CEO");
    rag3db_value_destroy(&employmentFieldValue);
    rag3db_destroy_string(employmentFieldName);
    rag3db_destroy_string(stringResult);
    state = rag3db_value_get_struct_field_name(&structFieldValue, 1, &employmentFieldName);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_STREQ(employmentFieldName, "is_current");
    state = rag3db_value_get_struct_field_value(&structFieldValue, 1, &employmentFieldValue);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(employmentFieldValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&employmentFieldValue));
    bool boolResult;
    state = rag3db_value_get_bool(&employmentFieldValue, &boolResult);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_EQ(boolResult, true);
    rag3db_value_destroy(&employmentFieldValue);
    rag3db_destroy_string(employmentFieldName);
    rag3db_value_destroy(&structFieldValue);
    rag3db_destroy_string(structFieldName);
    rag3db_value_destroy(&outValue);
    rag3db_value_destroy(person);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&stmt);
}

TEST(CApiValueTestEmptyDB, CreateStructEmpty) {
    const char* fieldNames[] = {(char*)"name"}; // Must be non-empty
    rag3db_value* values[] = {nullptr};           // Must be non-empty
    rag3db_value* value = nullptr;
    rag3db_state state = rag3db_value_create_struct(0, fieldNames, values, &value);
    ASSERT_EQ(state, Rag3dbError);
}

TEST_F(CApiValueTest, CreateMap) {
    auto connection = getConnection();
    rag3db_value* key1 = rag3db_value_create_int64(1);
    rag3db_value* value1 = rag3db_value_create_string((char*)"one");
    rag3db_value* key2 = rag3db_value_create_int64(2);
    rag3db_value* value2 = rag3db_value_create_string((char*)"two");
    rag3db_value* key3 = rag3db_value_create_int64(3);
    rag3db_value* value3 = rag3db_value_create_string((char*)"three");
    rag3db_value* keys[] = {key1, key2, key3};
    rag3db_value* values[] = {value1, value2, value3};
    rag3db_value* map = nullptr;
    rag3db_state state = rag3db_value_create_map(3, keys, values, &map);
    ASSERT_EQ(state, Rag3dbSuccess);
    // Destroy the original values, the map should still be valid
    for (int i = 0; i < 3; ++i) {
        rag3db_value_destroy(keys[i]);
        rag3db_value_destroy(values[i]);
    }
    ASSERT_FALSE(map->_is_owned_by_cpp);
    rag3db_prepared_statement stmt;
    state = rag3db_connection_prepare(connection, (char*)"RETURN $1", &stmt);
    ASSERT_EQ(state, Rag3dbSuccess);
    state = rag3db_prepared_statement_bind_value(&stmt, "1", map);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_query_result result;
    state = rag3db_connection_execute(connection, &stmt, &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    rag3db_flat_tuple flatTuple;
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value outValue;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &outValue), Rag3dbSuccess);
    ASSERT_TRUE(outValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&outValue));
    uint64_t size;
    ASSERT_EQ(rag3db_value_get_map_size(&outValue, &size), Rag3dbSuccess);
    ASSERT_EQ(size, 3);
    rag3db_value mapValue;
    ASSERT_EQ(rag3db_value_get_map_value(&outValue, 0, &mapValue), Rag3dbSuccess);
    ASSERT_TRUE(mapValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&mapValue));
    char* stringResult;
    ASSERT_EQ(rag3db_value_get_string(&mapValue, &stringResult), Rag3dbSuccess);
    ASSERT_STREQ(stringResult, "one");
    rag3db_value_destroy(&mapValue);
    rag3db_destroy_string(stringResult);
    ASSERT_EQ(rag3db_value_get_map_value(&outValue, 1, &mapValue), Rag3dbSuccess);
    ASSERT_TRUE(mapValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&mapValue));
    ASSERT_EQ(rag3db_value_get_string(&mapValue, &stringResult), Rag3dbSuccess);
    ASSERT_STREQ(stringResult, "two");
    rag3db_value_destroy(&mapValue);
    rag3db_destroy_string(stringResult);
    ASSERT_EQ(rag3db_value_get_map_value(&outValue, 2, &mapValue), Rag3dbSuccess);
    ASSERT_TRUE(mapValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&mapValue));
    ASSERT_EQ(rag3db_value_get_string(&mapValue, &stringResult), Rag3dbSuccess);
    ASSERT_STREQ(stringResult, "three");
    rag3db_value_destroy(&mapValue);
    rag3db_destroy_string(stringResult);
    ASSERT_EQ(rag3db_value_get_map_key(&outValue, 0, &mapValue), Rag3dbSuccess);
    ASSERT_TRUE(mapValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&mapValue));
    int64_t int64Result;
    ASSERT_EQ(rag3db_value_get_int64(&mapValue, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 1);
    rag3db_value_destroy(&mapValue);
    ASSERT_EQ(rag3db_value_get_map_key(&outValue, 1, &mapValue), Rag3dbSuccess);
    ASSERT_TRUE(mapValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&mapValue));
    ASSERT_EQ(rag3db_value_get_int64(&mapValue, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 2);
    rag3db_value_destroy(&mapValue);
    ASSERT_EQ(rag3db_value_get_map_key(&outValue, 2, &mapValue), Rag3dbSuccess);
    ASSERT_TRUE(mapValue._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&mapValue));
    ASSERT_EQ(rag3db_value_get_int64(&mapValue, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 3);
    rag3db_value_destroy(&mapValue);
    rag3db_value_destroy(&outValue);
    rag3db_value_destroy(map);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
    rag3db_prepared_statement_destroy(&stmt);
}

TEST(CApiValueTestEmptyDB, CreateMapEmpty) {
    rag3db_value* keys[] = {nullptr};   // Must be non-empty
    rag3db_value* values[] = {nullptr}; // Must be non-empty
    rag3db_value* map = nullptr;
    rag3db_state state = rag3db_value_create_map(0, keys, values, &map);
    ASSERT_EQ(state, Rag3dbError);
}

TEST(CApiValueTestEmptyDB, Clone) {
    rag3db_value* value = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_FALSE(value->_is_owned_by_cpp);
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::STRING);
    ASSERT_EQ(cppValue->getValue<std::string>(), "abcdefg");

    rag3db_value* clone = rag3db_value_clone(value);
    rag3db_value_destroy(value);

    ASSERT_FALSE(clone->_is_owned_by_cpp);
    auto cppClone = static_cast<Value*>(clone->_value);
    ASSERT_EQ(cppClone->getDataType().getLogicalTypeID(), LogicalTypeID::STRING);
    ASSERT_EQ(cppClone->getValue<std::string>(), "abcdefg");
    rag3db_value_destroy(clone);
}

TEST(CApiValueTestEmptyDB, Copy) {
    rag3db_value* value = rag3db_value_create_string((char*)"abc");

    rag3db_value* value2 = rag3db_value_create_string((char*)"abcdefg");
    rag3db_value_copy(value, value2);
    rag3db_value_destroy(value2);

    ASSERT_FALSE(rag3db_value_is_null(value));
    auto cppValue = static_cast<Value*>(value->_value);
    ASSERT_EQ(cppValue->getDataType().getLogicalTypeID(), LogicalTypeID::STRING);
    ASSERT_EQ(cppValue->getValue<std::string>(), "abcdefg");
    rag3db_value_destroy(value);
}

TEST_F(CApiValueTest, GetListSize) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) RETURN a.workedHours ORDER BY a.ID", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    uint64_t size;
    ASSERT_EQ(rag3db_value_get_list_size(&value, &size), Rag3dbSuccess);
    ASSERT_EQ(size, 2);

    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_list_size(badValue, &size), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetListElement) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) RETURN a.workedHours ORDER BY a.ID", &result);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    uint64_t size;
    ASSERT_EQ(rag3db_value_get_list_size(&value, &size), Rag3dbSuccess);
    ASSERT_EQ(size, 2);

    rag3db_value listElement;
    ASSERT_EQ(rag3db_value_get_list_element(&value, 0, &listElement), Rag3dbSuccess);
    ASSERT_TRUE(listElement._is_owned_by_cpp);
    int64_t int64Result;
    ASSERT_EQ(rag3db_value_get_int64(&listElement, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 10);

    ASSERT_EQ(rag3db_value_get_list_element(&value, 1, &listElement), Rag3dbSuccess);
    ASSERT_TRUE(listElement._is_owned_by_cpp);
    ASSERT_EQ(rag3db_value_get_int64(&listElement, &int64Result), Rag3dbSuccess);
    ASSERT_EQ(int64Result, 5);
    rag3db_value_destroy(&listElement);

    ASSERT_EQ(rag3db_value_get_list_element(&value, 222, &listElement), Rag3dbError);

    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
}

TEST_F(CApiValueTest, GetStructNumFields) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (m:movies) WHERE m.name=\"Roma\" RETURN m.description", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    rag3db_flat_tuple_get_value(&flatTuple, 0, &value);
    uint64_t numFields;
    ASSERT_EQ(rag3db_value_get_struct_num_fields(&value, &numFields), Rag3dbSuccess);
    ASSERT_EQ(numFields, 14);

    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_struct_num_fields(badValue, &numFields), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetStructFieldName) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (m:movies) WHERE m.name=\"Roma\" RETURN m.description", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    char* fieldName;
    ASSERT_EQ(rag3db_value_get_struct_field_name(&value, 0, &fieldName), Rag3dbSuccess);
    ASSERT_STREQ(fieldName, "rating");
    rag3db_destroy_string(fieldName);

    ASSERT_EQ(rag3db_value_get_struct_field_name(&value, 1, &fieldName), Rag3dbSuccess);
    ASSERT_STREQ(fieldName, "stars");
    rag3db_destroy_string(fieldName);

    ASSERT_EQ(rag3db_value_get_struct_field_name(&value, 2, &fieldName), Rag3dbSuccess);
    ASSERT_STREQ(fieldName, "views");
    rag3db_destroy_string(fieldName);

    ASSERT_EQ(rag3db_value_get_struct_field_name(&value, 3, &fieldName), Rag3dbSuccess);
    ASSERT_STREQ(fieldName, "release");
    rag3db_destroy_string(fieldName);

    ASSERT_EQ(rag3db_value_get_struct_field_name(&value, 4, &fieldName), Rag3dbSuccess);
    ASSERT_STREQ(fieldName, "release_ns");
    rag3db_destroy_string(fieldName);

    ASSERT_EQ(rag3db_value_get_struct_field_name(&value, 5, &fieldName), Rag3dbSuccess);
    ASSERT_STREQ(fieldName, "release_ms");
    rag3db_destroy_string(fieldName);

    ASSERT_EQ(rag3db_value_get_struct_field_name(&value, 6, &fieldName), Rag3dbSuccess);
    ASSERT_STREQ(fieldName, "release_sec");
    rag3db_destroy_string(fieldName);

    ASSERT_EQ(rag3db_value_get_struct_field_name(&value, 7, &fieldName), Rag3dbSuccess);
    ASSERT_STREQ(fieldName, "release_tz");
    rag3db_destroy_string(fieldName);

    ASSERT_EQ(rag3db_value_get_struct_field_name(&value, 8, &fieldName), Rag3dbSuccess);
    ASSERT_STREQ(fieldName, "film");
    rag3db_destroy_string(fieldName);

    ASSERT_EQ(rag3db_value_get_struct_field_name(&value, 222, &fieldName), Rag3dbError);

    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
}

TEST_F(CApiValueTest, GetStructFieldValue) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (m:movies) WHERE m.name=\"Roma\" RETURN m.description", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);

    rag3db_value fieldValue;
    ASSERT_EQ(rag3db_value_get_struct_field_value(&value, 0, &fieldValue), Rag3dbSuccess);
    rag3db_logical_type fieldType;
    rag3db_value_get_data_type(&fieldValue, &fieldType);
    ASSERT_EQ(rag3db_data_type_get_id(&fieldType), RAG3DB_DOUBLE);
    double doubleValue;
    ASSERT_EQ(rag3db_value_get_double(&fieldValue, &doubleValue), Rag3dbSuccess);
    ASSERT_DOUBLE_EQ(doubleValue, 1223);
    rag3db_data_type_destroy(&fieldType);

    ASSERT_EQ(rag3db_value_get_struct_field_value(&value, 1, &fieldValue), Rag3dbSuccess);
    rag3db_value_get_data_type(&fieldValue, &fieldType);
    rag3db_data_type_destroy(&fieldType);

    ASSERT_EQ(rag3db_value_get_struct_field_value(&value, 2, &fieldValue), Rag3dbSuccess);
    rag3db_value_get_data_type(&fieldValue, &fieldType);
    ASSERT_EQ(rag3db_data_type_get_id(&fieldType), RAG3DB_INT64);
    int64_t int64Value;
    ASSERT_EQ(rag3db_value_get_int64(&fieldValue, &int64Value), Rag3dbSuccess);
    rag3db_data_type_destroy(&fieldType);

    ASSERT_EQ(rag3db_value_get_struct_field_value(&value, 3, &fieldValue), Rag3dbSuccess);
    rag3db_value_get_data_type(&fieldValue, &fieldType);
    ASSERT_EQ(rag3db_data_type_get_id(&fieldType), RAG3DB_TIMESTAMP);
    rag3db_timestamp_t timestamp;
    ASSERT_EQ(rag3db_value_get_timestamp(&fieldValue, &timestamp), Rag3dbSuccess);
    ASSERT_EQ(timestamp.value, 1297442662000000);
    rag3db_data_type_destroy(&fieldType);

    ASSERT_EQ(rag3db_value_get_struct_field_value(&value, 4, &fieldValue), Rag3dbSuccess);
    rag3db_value_get_data_type(&fieldValue, &fieldType);
    ASSERT_EQ(rag3db_data_type_get_id(&fieldType), RAG3DB_TIMESTAMP_NS);
    rag3db_timestamp_ns_t timestamp_ns;
    ASSERT_EQ(rag3db_value_get_timestamp_ns(&fieldValue, &timestamp_ns), Rag3dbSuccess);
    ASSERT_EQ(timestamp_ns.value, 1297442662123456000);
    rag3db_data_type_destroy(&fieldType);

    ASSERT_EQ(rag3db_value_get_struct_field_value(&value, 5, &fieldValue), Rag3dbSuccess);
    rag3db_value_get_data_type(&fieldValue, &fieldType);
    ASSERT_EQ(rag3db_data_type_get_id(&fieldType), RAG3DB_TIMESTAMP_MS);
    rag3db_timestamp_ms_t timestamp_ms;
    ASSERT_EQ(rag3db_value_get_timestamp_ms(&fieldValue, &timestamp_ms), Rag3dbSuccess);
    ASSERT_EQ(timestamp_ms.value, 1297442662123);
    rag3db_data_type_destroy(&fieldType);

    ASSERT_EQ(rag3db_value_get_struct_field_value(&value, 6, &fieldValue), Rag3dbSuccess);
    rag3db_value_get_data_type(&fieldValue, &fieldType);
    ASSERT_EQ(rag3db_data_type_get_id(&fieldType), RAG3DB_TIMESTAMP_SEC);
    rag3db_timestamp_sec_t timestamp_sec;
    ASSERT_EQ(rag3db_value_get_timestamp_sec(&fieldValue, &timestamp_sec), Rag3dbSuccess);
    ASSERT_EQ(timestamp_sec.value, 1297442662);
    rag3db_data_type_destroy(&fieldType);

    ASSERT_EQ(rag3db_value_get_struct_field_value(&value, 7, &fieldValue), Rag3dbSuccess);
    rag3db_value_get_data_type(&fieldValue, &fieldType);
    ASSERT_EQ(rag3db_data_type_get_id(&fieldType), RAG3DB_TIMESTAMP_TZ);
    rag3db_timestamp_tz_t timestamp_tz;
    ASSERT_EQ(rag3db_value_get_timestamp_tz(&fieldValue, &timestamp_tz), Rag3dbSuccess);
    ASSERT_EQ(timestamp_tz.value, 1297442662123456);
    rag3db_data_type_destroy(&fieldType);

    ASSERT_EQ(rag3db_value_get_struct_field_value(&value, 8, &fieldValue), Rag3dbSuccess);
    rag3db_value_get_data_type(&fieldValue, &fieldType);
    ASSERT_EQ(rag3db_data_type_get_id(&fieldType), RAG3DB_DATE);
    rag3db_date_t date;
    ASSERT_EQ(rag3db_value_get_date(&fieldValue, &date), Rag3dbSuccess);
    ASSERT_EQ(date.days, 15758);
    rag3db_data_type_destroy(&fieldType);
    rag3db_value_destroy(&fieldValue);

    ASSERT_EQ(rag3db_value_get_struct_field_value(&value, 222, &fieldValue), Rag3dbError);

    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
}

TEST_F(CApiValueTest, getMapNumFields) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (m:movies) WHERE m.length = 2544 RETURN m.audience", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_FALSE(rag3db_query_result_has_next(&result));
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);

    uint64_t mapFields;
    ASSERT_EQ(rag3db_value_get_map_size(&value, &mapFields), Rag3dbSuccess);
    ASSERT_EQ(mapFields, 1);

    rag3db_query_result_destroy(&result);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
}

TEST_F(CApiValueTest, getMapKey) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (m:movies) WHERE m.length = 2544 RETURN m.audience", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_FALSE(rag3db_query_result_has_next(&result));
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);

    rag3db_value key;
    ASSERT_EQ(rag3db_value_get_map_key(&value, 0, &key), Rag3dbSuccess);
    rag3db_logical_type keyType;
    rag3db_value_get_data_type(&key, &keyType);
    ASSERT_EQ(rag3db_data_type_get_id(&keyType), RAG3DB_STRING);
    char* mapName;
    ASSERT_EQ(rag3db_value_get_string(&key, &mapName), Rag3dbSuccess);
    ASSERT_STREQ(mapName, "audience1");
    rag3db_destroy_string(mapName);
    rag3db_data_type_destroy(&keyType);
    rag3db_value_destroy(&key);

    ASSERT_EQ(rag3db_value_get_map_key(&value, 1, &key), Rag3dbError);
    rag3db_query_result_destroy(&result);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
}

TEST_F(CApiValueTest, getMapValue) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (m:movies) WHERE m.length = 2544 RETURN m.audience", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_FALSE(rag3db_query_result_has_next(&result));
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);

    rag3db_value mapValue;
    ASSERT_EQ(rag3db_value_get_map_value(&value, 0, &mapValue), Rag3dbSuccess);
    rag3db_logical_type mapType;
    rag3db_value_get_data_type(&mapValue, &mapType);
    ASSERT_EQ(rag3db_data_type_get_id(&mapType), RAG3DB_INT64);
    int64_t mapIntValue;
    ASSERT_EQ(rag3db_value_get_int64(&mapValue, &mapIntValue), Rag3dbSuccess);
    ASSERT_EQ(mapIntValue, 33);

    ASSERT_EQ(rag3db_value_get_map_value(&value, 1, &mapValue), Rag3dbError);

    rag3db_data_type_destroy(&mapType);
    rag3db_query_result_destroy(&result);
    rag3db_value_destroy(&mapValue);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
}

TEST_F(CApiValueTest, getDecimalAsString) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"UNWIND [1] AS A UNWIND [5.7, 8.3, 8.7, 13.7] AS B WITH cast(CAST(A AS DECIMAL) "
               "* "
               "CAST(B AS DECIMAL) AS DECIMAL(18, 1)) AS PROD RETURN COLLECT(PROD) AS RES",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);

    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);

    rag3db_logical_type dataType;
    rag3db_value_get_data_type(&value, &dataType);
    ASSERT_EQ(rag3db_data_type_get_id(&dataType), RAG3DB_LIST);
    uint64_t list_size;
    ASSERT_EQ(rag3db_value_get_list_size(&value, &list_size), Rag3dbSuccess);
    ASSERT_EQ(list_size, 4);
    rag3db_data_type_destroy(&dataType);

    rag3db_value decimal_entry;
    char* decimal_value;
    std::string decimal_string_value;
    ASSERT_EQ(rag3db_value_get_list_element(&value, 0, &decimal_entry), Rag3dbSuccess);
    rag3db_value_get_data_type(&decimal_entry, &dataType);
    ASSERT_EQ(rag3db_data_type_get_id(&dataType), RAG3DB_DECIMAL);
    ASSERT_EQ(rag3db_value_get_decimal_as_string(&decimal_entry, &decimal_value), Rag3dbSuccess);
    decimal_string_value = std::string(decimal_value);
    ASSERT_EQ(decimal_string_value, "5.7");
    rag3db_destroy_string(decimal_value);
    rag3db_data_type_destroy(&dataType);

    ASSERT_EQ(rag3db_value_get_list_element(&value, 1, &decimal_entry), Rag3dbSuccess);
    rag3db_value_get_data_type(&decimal_entry, &dataType);
    ASSERT_EQ(rag3db_data_type_get_id(&dataType), RAG3DB_DECIMAL);
    ASSERT_EQ(rag3db_value_get_decimal_as_string(&decimal_entry, &decimal_value), Rag3dbSuccess);
    decimal_string_value = std::string(decimal_value);
    ASSERT_EQ(decimal_string_value, "8.3");
    rag3db_destroy_string(decimal_value);
    rag3db_data_type_destroy(&dataType);

    ASSERT_EQ(rag3db_value_get_list_element(&value, 2, &decimal_entry), Rag3dbSuccess);
    rag3db_value_get_data_type(&decimal_entry, &dataType);
    ASSERT_EQ(rag3db_data_type_get_id(&dataType), RAG3DB_DECIMAL);
    ASSERT_EQ(rag3db_value_get_decimal_as_string(&decimal_entry, &decimal_value), Rag3dbSuccess);
    decimal_string_value = std::string(decimal_value);
    ASSERT_EQ(decimal_string_value, "8.7");
    rag3db_destroy_string(decimal_value);
    rag3db_data_type_destroy(&dataType);

    ASSERT_EQ(rag3db_value_get_list_element(&value, 3, &decimal_entry), Rag3dbSuccess);
    rag3db_value_get_data_type(&decimal_entry, &dataType);
    ASSERT_EQ(rag3db_data_type_get_id(&dataType), RAG3DB_DECIMAL);
    ASSERT_EQ(rag3db_value_get_decimal_as_string(&decimal_entry, &decimal_value), Rag3dbSuccess);
    decimal_string_value = std::string(decimal_value);
    ASSERT_EQ(decimal_string_value, "13.7");
    rag3db_destroy_string(decimal_value);
    rag3db_data_type_destroy(&dataType);

    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
    rag3db_value_destroy(&decimal_entry);
}

TEST_F(CApiValueTest, GetDataType) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) RETURN a.fName, a.isStudent, a.workedHours", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    rag3db_logical_type dataType;
    rag3db_value_get_data_type(&value, &dataType);
    ASSERT_EQ(rag3db_data_type_get_id(&dataType), RAG3DB_STRING);
    rag3db_data_type_destroy(&dataType);

    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 1, &value), Rag3dbSuccess);
    rag3db_value_get_data_type(&value, &dataType);
    ASSERT_EQ(rag3db_data_type_get_id(&dataType), RAG3DB_BOOL);
    rag3db_data_type_destroy(&dataType);

    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 2, &value), Rag3dbSuccess);
    rag3db_value_get_data_type(&value, &dataType);
    ASSERT_EQ(rag3db_data_type_get_id(&dataType), RAG3DB_LIST);
    rag3db_data_type_destroy(&dataType);
    rag3db_value_destroy(&value);

    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
}

TEST_F(CApiValueTest, GetBool) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) RETURN a.isStudent ORDER BY a.ID", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    bool boolValue;
    ASSERT_EQ(rag3db_value_get_bool(&value, &boolValue), Rag3dbSuccess);
    ASSERT_TRUE(boolValue);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_bool(badValue, &boolValue), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetInt8) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) -[r:studyAt]-> (b:organisation) RETURN r.level ORDER BY a.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    int8_t int8Value;
    ASSERT_EQ(rag3db_value_get_int8(&value, &int8Value), Rag3dbSuccess);
    ASSERT_EQ(int8Value, 5);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_int8(badValue, &int8Value), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetInt16) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) -[r:studyAt]-> (b:organisation) RETURN r.length ORDER BY a.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    int16_t int16Value;
    ASSERT_EQ(rag3db_value_get_int16(&value, &int16Value), Rag3dbSuccess);
    ASSERT_EQ(int16Value, 5);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_int16(badValue, &int16Value), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetInt32) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (m:movies) RETURN m.length ORDER BY m.name", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    int32_t int32Value;
    ASSERT_EQ(rag3db_value_get_int32(&value, &int32Value), Rag3dbSuccess);
    ASSERT_EQ(int32Value, 298);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_int32(badValue, &int32Value), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetInt64) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, (char*)"MATCH (a:person) RETURN a.ID ORDER BY a.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    int64_t int64Value;
    ASSERT_EQ(rag3db_value_get_int64(&value, &int64Value), Rag3dbSuccess);
    ASSERT_EQ(int64Value, 0);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_int64(badValue, &int64Value), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetUInt8) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) -[r:studyAt]-> (b:organisation) RETURN r.ulevel ORDER BY a.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    uint8_t uint8Value;
    ASSERT_EQ(rag3db_value_get_uint8(&value, &uint8Value), Rag3dbSuccess);
    ASSERT_EQ(uint8Value, 250);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_uint8(badValue, &uint8Value), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetUInt16) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) -[r:studyAt]-> (b:organisation) RETURN r.ulength ORDER BY "
               "a.ID",
        &result);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    uint16_t uint16Value;
    ASSERT_EQ(rag3db_value_get_uint16(&value, &uint16Value), Rag3dbSuccess);
    ASSERT_EQ(uint16Value, 33768);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_uint16(badValue, &uint16Value), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetUInt32) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) -[r:studyAt]-> (b:organisation) "
               "RETURN r.temperature ORDER BY a.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    uint32_t uint32Value;
    ASSERT_EQ(rag3db_value_get_uint32(&value, &uint32Value), Rag3dbSuccess);
    ASSERT_EQ(uint32Value, 32800);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_uint32(badValue, &uint32Value), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetUInt64) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) -[r:studyAt]-> (b:organisation) RETURN r.code ORDER BY a.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    uint64_t uint64Value;
    ASSERT_EQ(rag3db_value_get_uint64(&value, &uint64Value), Rag3dbSuccess);
    ASSERT_EQ(uint64Value, 9223372036854775808ull);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_uint64(badValue, &uint64Value), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetInt128) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) -[r:studyAt]-> (b:organisation) RETURN r.hugedata ORDER BY "
               "a.ID",
        &result);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    rag3db_int128_t int128;
    ASSERT_EQ(rag3db_value_get_int128(&value, &int128), Rag3dbSuccess);
    ASSERT_EQ(int128.high, 100000000);
    ASSERT_EQ(int128.low, 211111111);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_int128(badValue, &int128), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, StringToInt128Test) {
    char input[] = "1844674407370955161811111111";
    rag3db_int128_t int128_val;
    ASSERT_EQ(rag3db_int128_t_from_string(input, &int128_val), Rag3dbSuccess);
    ASSERT_EQ(int128_val.high, 100000000);
    ASSERT_EQ(int128_val.low, 211111111);

    char badInput[] = "this is not a int128";
    rag3db_int128_t int128_val2;
    ASSERT_EQ(rag3db_int128_t_from_string(badInput, &int128_val2), Rag3dbError);
}

TEST_F(CApiValueTest, Int128ToStringTest) {
    auto int128_val = rag3db_int128_t{211111111, 100000000};
    char* str;
    ASSERT_EQ(rag3db_int128_t_to_string(int128_val, &str), Rag3dbSuccess);
    ASSERT_STREQ(str, "1844674407370955161811111111");
    rag3db_destroy_string(str);
}

TEST_F(CApiValueTest, GetFloat) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) RETURN a.height ORDER BY a.ID", &result);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    float floatValue;
    ASSERT_EQ(rag3db_value_get_float(&value, &floatValue), Rag3dbSuccess);
    ASSERT_FLOAT_EQ(floatValue, 1.731);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_float(badValue, &floatValue), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetDouble) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) RETURN a.eyeSight ORDER BY a.ID", &result);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    double doubleValue;
    ASSERT_EQ(rag3db_value_get_double(&value, &doubleValue), Rag3dbSuccess);
    ASSERT_DOUBLE_EQ(doubleValue, 5.0);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_double(badValue, &doubleValue), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetInternalID) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, (char*)"MATCH (a:person) RETURN a ORDER BY a.ID",
        &result);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    rag3db_value nodeIDVal;
    ASSERT_EQ(rag3db_value_get_struct_field_value(&value, 0 /* internal ID field idx */, &nodeIDVal),
        Rag3dbSuccess);
    rag3db_internal_id_t internalID;
    ASSERT_EQ(rag3db_value_get_internal_id(&nodeIDVal, &internalID), Rag3dbSuccess);
    ASSERT_EQ(internalID.table_id, 0);
    ASSERT_EQ(internalID.offset, 0);
    rag3db_value_destroy(&nodeIDVal);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_internal_id(badValue, &internalID), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetRelVal) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) -[r:knows]-> (b:person) RETURN r ORDER BY a.ID, b.ID", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value rel;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &rel), Rag3dbSuccess);
    ASSERT_TRUE(rel._is_owned_by_cpp);
    rag3db_value relIdVal;
    ASSERT_EQ(rag3db_rel_val_get_id_val(&rel, &relIdVal), Rag3dbSuccess);
    rag3db_internal_id_t relInternalID;
    ASSERT_EQ(rag3db_value_get_internal_id(&relIdVal, &relInternalID), Rag3dbSuccess);
    ASSERT_EQ(relInternalID.table_id, 3);
    ASSERT_EQ(relInternalID.offset, 0);
    rag3db_value relSrcIDVal;
    ASSERT_EQ(rag3db_rel_val_get_src_id_val(&rel, &relSrcIDVal), Rag3dbSuccess);
    rag3db_internal_id_t relSrcID;
    ASSERT_EQ(rag3db_value_get_internal_id(&relSrcIDVal, &relSrcID), Rag3dbSuccess);
    ASSERT_EQ(relSrcID.table_id, 0);
    ASSERT_EQ(relSrcID.offset, 0);
    rag3db_value relDstIDVal;
    ASSERT_EQ(rag3db_rel_val_get_dst_id_val(&rel, &relDstIDVal), Rag3dbSuccess);
    rag3db_internal_id_t relDstID;
    ASSERT_EQ(rag3db_value_get_internal_id(&relDstIDVal, &relDstID), Rag3dbSuccess);
    ASSERT_EQ(relDstID.table_id, 0);
    ASSERT_EQ(relDstID.offset, 1);
    rag3db_value relLabel;
    ASSERT_EQ(rag3db_rel_val_get_label_val(&rel, &relLabel), Rag3dbSuccess);
    char* relLabelStr;
    ASSERT_EQ(rag3db_value_get_string(&relLabel, &relLabelStr), Rag3dbSuccess);
    ASSERT_STREQ(relLabelStr, "knows");
    uint64_t propertiesSize;
    ASSERT_EQ(rag3db_rel_val_get_property_size(&rel, &propertiesSize), Rag3dbSuccess);
    ASSERT_EQ(propertiesSize, 7);
    rag3db_destroy_string(relLabelStr);
    rag3db_value_destroy(&relLabel);
    rag3db_value_destroy(&relIdVal);
    rag3db_value_destroy(&relSrcIDVal);
    rag3db_value_destroy(&relDstIDVal);
    rag3db_value_destroy(&rel);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_rel_val_get_src_id_val(badValue, &relSrcIDVal), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetDate) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) RETURN a.birthdate ORDER BY a.ID", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    rag3db_date_t date;
    ASSERT_EQ(rag3db_value_get_date(&value, &date), Rag3dbSuccess);
    ASSERT_EQ(date.days, -25567);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_date(badValue, &date), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetTimestamp) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) RETURN a.registerTime ORDER BY a.ID", &result);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    rag3db_timestamp_t timestamp;
    ASSERT_EQ(rag3db_value_get_timestamp(&value, &timestamp), Rag3dbSuccess);
    ASSERT_EQ(timestamp.value, 1313839530000000);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_timestamp(badValue, &timestamp), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetInterval) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) RETURN a.lastJobDuration ORDER BY a.ID", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    rag3db_interval_t interval;
    ASSERT_EQ(rag3db_value_get_interval(&value, &interval), Rag3dbSuccess);
    ASSERT_EQ(interval.months, 36);
    ASSERT_EQ(interval.days, 2);
    ASSERT_EQ(interval.micros, 46920000000);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_interval(badValue, &interval), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetString) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) RETURN a.fName ORDER BY a.ID", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    char* str;
    ASSERT_EQ(rag3db_value_get_string(&value, &str), Rag3dbSuccess);
    ASSERT_STREQ(str, "Alice");
    rag3db_destroy_string(str);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_int32(123);
    ASSERT_EQ(rag3db_value_get_string(badValue, &str), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetBlob) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state =
        rag3db_connection_query(connection, (char*)R"(RETURN BLOB('\xAA\xBB\xCD\x1A');)", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    uint8_t* blob;
    ASSERT_EQ(rag3db_value_get_blob(&value, &blob), Rag3dbSuccess);
    ASSERT_EQ(blob[0], 0xAA);
    ASSERT_EQ(blob[1], 0xBB);
    ASSERT_EQ(blob[2], 0xCD);
    ASSERT_EQ(blob[3], 0x1A);
    ASSERT_EQ(blob[4], 0x00);
    rag3db_destroy_blob(blob);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_blob(badValue, &blob), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetUUID) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)R"(RETURN UUID("A0EEBC99-9C0B-4EF8-BB6D-6BB9BD380A11");)", &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value value;
    rag3db_flat_tuple_get_value(&flatTuple, 0, &value);
    ASSERT_TRUE(value._is_owned_by_cpp);
    ASSERT_FALSE(rag3db_value_is_null(&value));
    char* str;
    ASSERT_EQ(rag3db_value_get_uuid(&value, &str), Rag3dbSuccess);
    ASSERT_STREQ(str, "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11");
    rag3db_destroy_string(str);
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_value_get_uuid(badValue, &str), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, ToSting) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) RETURN a.fName, a.isStudent, a.workedHours ORDER BY "
               "a.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));

    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);

    rag3db_value value;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &value), Rag3dbSuccess);
    char* str = rag3db_value_to_string(&value);
    ASSERT_STREQ(str, "Alice");
    rag3db_destroy_string(str);

    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 1, &value), Rag3dbSuccess);
    str = rag3db_value_to_string(&value);
    ASSERT_STREQ(str, "True");
    rag3db_destroy_string(str);

    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 2, &value), Rag3dbSuccess);
    str = rag3db_value_to_string(&value);
    ASSERT_STREQ(str, "[10,5]");
    rag3db_destroy_string(str);
    rag3db_value_destroy(&value);

    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
}

TEST_F(CApiValueTest, NodeValGetLabelVal) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, (char*)"MATCH (a:person) RETURN a ORDER BY a.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));

    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value nodeVal;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &nodeVal), Rag3dbSuccess);
    rag3db_value labelVal;
    ASSERT_EQ(rag3db_node_val_get_label_val(&nodeVal, &labelVal), Rag3dbSuccess);
    char* labelStr;
    ASSERT_EQ(rag3db_value_get_string(&labelVal, &labelStr), Rag3dbSuccess);
    ASSERT_STREQ(labelStr, "person");
    rag3db_destroy_string(labelStr);
    rag3db_value_destroy(&labelVal);
    rag3db_value_destroy(&nodeVal);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_node_val_get_label_val(badValue, &labelVal), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, NodeValGetID) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, (char*)"MATCH (a:person) RETURN a ORDER BY a.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));

    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value nodeVal;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &nodeVal), Rag3dbSuccess);
    rag3db_value nodeIDVal;
    ASSERT_EQ(rag3db_node_val_get_id_val(&nodeVal, &nodeIDVal), Rag3dbSuccess);
    ASSERT_NE(nodeIDVal._value, nullptr);
    rag3db_internal_id_t internalID;
    ASSERT_EQ(rag3db_value_get_internal_id(&nodeIDVal, &internalID), Rag3dbSuccess);
    ASSERT_EQ(internalID.table_id, 0);
    ASSERT_EQ(internalID.offset, 0);
    rag3db_value_destroy(&nodeIDVal);
    rag3db_value_destroy(&nodeVal);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_node_val_get_id_val(badValue, &nodeIDVal), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, NodeValGetLabelName) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, (char*)"MATCH (a:person) RETURN a ORDER BY a.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));

    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value nodeVal;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &nodeVal), Rag3dbSuccess);
    rag3db_value labelVal;
    ASSERT_EQ(rag3db_node_val_get_label_val(&nodeVal, &labelVal), Rag3dbSuccess);
    char* labelStr;
    ASSERT_EQ(rag3db_value_get_string(&labelVal, &labelStr), Rag3dbSuccess);
    ASSERT_STREQ(labelStr, "person");
    rag3db_destroy_string(labelStr);
    rag3db_value_destroy(&labelVal);
    rag3db_value_destroy(&nodeVal);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_node_val_get_label_val(badValue, &labelVal), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, NodeValGetProperty) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection, (char*)"MATCH (a:person) RETURN a ORDER BY a.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value node;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &node), Rag3dbSuccess);
    char* propertyName;
    ASSERT_EQ(rag3db_node_val_get_property_name_at(&node, 0, &propertyName), Rag3dbSuccess);
    ASSERT_STREQ(propertyName, "ID");
    rag3db_destroy_string(propertyName);
    ASSERT_EQ(rag3db_node_val_get_property_name_at(&node, 1, &propertyName), Rag3dbSuccess);
    ASSERT_STREQ(propertyName, "fName");
    rag3db_destroy_string(propertyName);
    ASSERT_EQ(rag3db_node_val_get_property_name_at(&node, 2, &propertyName), Rag3dbSuccess);
    ASSERT_STREQ(propertyName, "gender");
    rag3db_destroy_string(propertyName);
    ASSERT_EQ(rag3db_node_val_get_property_name_at(&node, 3, &propertyName), Rag3dbSuccess);
    ASSERT_STREQ(propertyName, "isStudent");
    rag3db_destroy_string(propertyName);

    rag3db_value propertyValue;
    ASSERT_EQ(rag3db_node_val_get_property_value_at(&node, 0, &propertyValue), Rag3dbSuccess);
    int64_t propertyValueID;
    ASSERT_EQ(rag3db_value_get_int64(&propertyValue, &propertyValueID), Rag3dbSuccess);
    ASSERT_EQ(propertyValueID, 0);
    ASSERT_EQ(rag3db_node_val_get_property_value_at(&node, 1, &propertyValue), Rag3dbSuccess);
    char* propertyValuefName;
    ASSERT_EQ(rag3db_value_get_string(&propertyValue, &propertyValuefName), Rag3dbSuccess);
    ASSERT_STREQ(propertyValuefName, "Alice");
    rag3db_destroy_string(propertyValuefName);
    ASSERT_EQ(rag3db_node_val_get_property_value_at(&node, 2, &propertyValue), Rag3dbSuccess);
    int64_t propertyValueGender;
    ASSERT_EQ(rag3db_value_get_int64(&propertyValue, &propertyValueGender), Rag3dbSuccess);
    ASSERT_EQ(propertyValueGender, 1);
    ASSERT_EQ(rag3db_node_val_get_property_value_at(&node, 3, &propertyValue), Rag3dbSuccess);
    bool propertyValueIsStudent;
    ASSERT_EQ(rag3db_value_get_bool(&propertyValue, &propertyValueIsStudent), Rag3dbSuccess);
    ASSERT_EQ(propertyValueIsStudent, true);
    rag3db_value_destroy(&propertyValue);

    rag3db_value_destroy(&node);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_node_val_get_property_name_at(badValue, 0, &propertyName), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, NodeValToString) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (b:organisation) RETURN b ORDER BY b.ID", &result);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value node;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &node), Rag3dbSuccess);
    ASSERT_TRUE(node._is_owned_by_cpp);

    char* str = rag3db_value_to_string(&node);
    ASSERT_STREQ(str,
        "{_ID: 1:0, _LABEL: organisation, ID: 1, name: ABFsUni, orgCode: 325, mark: 3.700000, "
        "score: -2, history: 10 years 5 months 13 hours 24 us, licenseValidInterval: 3 years "
        "5 days, rating: 1.000000, state: {revenue: 138, location: ['toronto','montr,eal'], "
        "stock: {price: [96,56], volume: 1000}}, info: 3.120000}");
    rag3db_destroy_string(str);

    rag3db_value_destroy(&node);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);
}

TEST_F(CApiValueTest, RelValGetProperty) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) -[e:workAt]-> (b:organisation) RETURN e ORDER BY a.ID, b.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value rel;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &rel), Rag3dbSuccess);
    ASSERT_TRUE(rel._is_owned_by_cpp);
    uint64_t propertiesSize;
    ASSERT_EQ(rag3db_rel_val_get_property_size(&rel, &propertiesSize), Rag3dbSuccess);
    ASSERT_EQ(propertiesSize, 3);

    char* propertyName;
    ASSERT_EQ(rag3db_rel_val_get_property_name_at(&rel, 0, &propertyName), Rag3dbSuccess);
    ASSERT_STREQ(propertyName, "year");
    rag3db_destroy_string(propertyName);

    ASSERT_EQ(rag3db_rel_val_get_property_name_at(&rel, 1, &propertyName), Rag3dbSuccess);
    ASSERT_STREQ(propertyName, "grading");
    rag3db_destroy_string(propertyName);
    ASSERT_EQ(rag3db_rel_val_get_property_name_at(&rel, 2, &propertyName), Rag3dbSuccess);
    ASSERT_STREQ(propertyName, "rating");
    rag3db_destroy_string(propertyName);

    rag3db_value propertyValue;
    ASSERT_EQ(rag3db_rel_val_get_property_value_at(&rel, 0, &propertyValue), Rag3dbSuccess);
    int64_t propertyValueYear;
    ASSERT_EQ(rag3db_value_get_int64(&propertyValue, &propertyValueYear), Rag3dbSuccess);
    ASSERT_EQ(propertyValueYear, 2015);

    ASSERT_EQ(rag3db_rel_val_get_property_value_at(&rel, 1, &propertyValue), Rag3dbSuccess);
    rag3db_value listValue;
    ASSERT_EQ(rag3db_value_get_list_element(&propertyValue, 0, &listValue), Rag3dbSuccess);
    double listValueGrading;
    ASSERT_EQ(rag3db_value_get_double(&listValue, &listValueGrading), Rag3dbSuccess);
    ASSERT_DOUBLE_EQ(listValueGrading, 3.8);
    ASSERT_EQ(rag3db_value_get_list_element(&propertyValue, 1, &listValue), Rag3dbSuccess);
    ASSERT_EQ(rag3db_value_get_double(&listValue, &listValueGrading), Rag3dbSuccess);
    ASSERT_DOUBLE_EQ(listValueGrading, 2.5);
    rag3db_value_destroy(&listValue);

    ASSERT_EQ(rag3db_rel_val_get_property_value_at(&rel, 2, &propertyValue), Rag3dbSuccess);
    float propertyValueRating;
    ASSERT_EQ(rag3db_value_get_float(&propertyValue, &propertyValueRating), Rag3dbSuccess);
    ASSERT_FLOAT_EQ(propertyValueRating, 8.2);
    rag3db_value_destroy(&propertyValue);

    rag3db_value_destroy(&rel);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_rel_val_get_property_name_at(badValue, 0, &propertyName), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, RelValToString) {
    rag3db_query_result result;
    rag3db_flat_tuple flatTuple;
    rag3db_state state;
    auto connection = getConnection();
    state = rag3db_connection_query(connection,
        (char*)"MATCH (a:person) -[e:workAt]-> (b:organisation) RETURN e ORDER BY a.ID, b.ID",
        &result);
    ASSERT_EQ(state, Rag3dbSuccess);
    ASSERT_TRUE(rag3db_query_result_is_success(&result));
    ASSERT_TRUE(rag3db_query_result_has_next(&result));
    state = rag3db_query_result_get_next(&result, &flatTuple);
    ASSERT_EQ(state, Rag3dbSuccess);
    rag3db_value rel;
    ASSERT_EQ(rag3db_flat_tuple_get_value(&flatTuple, 0, &rel), Rag3dbSuccess);
    ASSERT_TRUE(rel._is_owned_by_cpp);
    char* str;
    ASSERT_EQ(rag3db_rel_val_to_string(&rel, &str), Rag3dbSuccess);
    ASSERT_STREQ(str, "(0:2)-{_LABEL: workAt, _ID: 7:0, year: 2015, grading: [3.800000,2.500000], "
                      "rating: 8.200000}->(1:1)");
    rag3db_destroy_string(str);
    rag3db_value_destroy(&rel);
    rag3db_flat_tuple_destroy(&flatTuple);
    rag3db_query_result_destroy(&result);

    rag3db_value* badValue = rag3db_value_create_string((char*)"abcdefg");
    ASSERT_EQ(rag3db_rel_val_to_string(badValue, &str), Rag3dbError);
    rag3db_value_destroy(badValue);
}

TEST_F(CApiValueTest, GetTmFromNonStandardTimestamp) {
    rag3db_timestamp_ns_t timestamp_ns = rag3db_timestamp_ns_t{17515323532900000};
    rag3db_timestamp_ms_t timestamp_ms = rag3db_timestamp_ms_t{1012323435341};
    rag3db_timestamp_sec_t timestamp_sec = rag3db_timestamp_sec_t{1432135648};
    rag3db_timestamp_tz_t timestamp_tz = rag3db_timestamp_tz_t{771513532900000};
    struct tm tm;
    ASSERT_EQ(rag3db_timestamp_ns_to_tm(timestamp_ns, &tm), Rag3dbSuccess);
    ASSERT_EQ(tm.tm_year, 70);
    ASSERT_EQ(tm.tm_mon, 6);
    ASSERT_EQ(tm.tm_mday, 22);
    ASSERT_EQ(tm.tm_hour, 17);
    ASSERT_EQ(tm.tm_min, 22);
    ASSERT_EQ(tm.tm_sec, 3);
    ASSERT_EQ(rag3db_timestamp_ms_to_tm(timestamp_ms, &tm), Rag3dbSuccess);
    ASSERT_EQ(tm.tm_year, 102);
    ASSERT_EQ(tm.tm_mon, 0);
    ASSERT_EQ(tm.tm_mday, 29);
    ASSERT_EQ(tm.tm_hour, 16);
    ASSERT_EQ(tm.tm_min, 57);
    ASSERT_EQ(tm.tm_sec, 15);
    ASSERT_EQ(rag3db_timestamp_sec_to_tm(timestamp_sec, &tm), Rag3dbSuccess);
    ASSERT_EQ(tm.tm_year, 115);
    ASSERT_EQ(tm.tm_mon, 4);
    ASSERT_EQ(tm.tm_mday, 20);
    ASSERT_EQ(tm.tm_hour, 15);
    ASSERT_EQ(tm.tm_min, 27);
    ASSERT_EQ(tm.tm_sec, 28);
    ASSERT_EQ(rag3db_timestamp_tz_to_tm(timestamp_tz, &tm), Rag3dbSuccess);
    ASSERT_EQ(tm.tm_year, 94);
    ASSERT_EQ(tm.tm_mon, 5);
    ASSERT_EQ(tm.tm_mday, 13);
    ASSERT_EQ(tm.tm_hour, 13);
    ASSERT_EQ(tm.tm_min, 18);
    ASSERT_EQ(tm.tm_sec, 52);
}

TEST_F(CApiValueTest, GetTmFromTimestamp) {
    rag3db_timestamp_t timestamp = rag3db_timestamp_t{171513532900000};
    struct tm tm;
    ASSERT_EQ(rag3db_timestamp_to_tm(timestamp, &tm), Rag3dbSuccess);
    ASSERT_EQ(tm.tm_year, 75);
    ASSERT_EQ(tm.tm_mon, 5);
    ASSERT_EQ(tm.tm_mday, 9);
    ASSERT_EQ(tm.tm_hour, 2);
    ASSERT_EQ(tm.tm_min, 38);
    ASSERT_EQ(tm.tm_sec, 52);
}

TEST_F(CApiValueTest, GetTmFromDate) {
    rag3db_date_t date = rag3db_date_t{-255};
    struct tm tm;
    ASSERT_EQ(rag3db_date_to_tm(date, &tm), Rag3dbSuccess);
    ASSERT_EQ(tm.tm_year, 69);
    ASSERT_EQ(tm.tm_mon, 3);
    ASSERT_EQ(tm.tm_mday, 21);
    ASSERT_EQ(tm.tm_hour, 0);
    ASSERT_EQ(tm.tm_min, 0);
    ASSERT_EQ(tm.tm_sec, 0);
}

TEST_F(CApiValueTest, GetTimestampFromTm) {
    struct tm tm;
    tm.tm_year = 75;
    tm.tm_mon = 5;
    tm.tm_mday = 9;
    tm.tm_hour = 2;
    tm.tm_min = 38;
    tm.tm_sec = 52;
    rag3db_timestamp_t timestamp;
    ASSERT_EQ(rag3db_timestamp_from_tm(tm, &timestamp), Rag3dbSuccess);
    ASSERT_EQ(timestamp.value, 171513532000000);
}

TEST_F(CApiValueTest, GetNonStandardTimestampFromTm) {
    struct tm tm;
    tm.tm_year = 70;
    tm.tm_mon = 6;
    tm.tm_mday = 22;
    tm.tm_hour = 17;
    tm.tm_min = 22;
    tm.tm_sec = 3;
    rag3db_timestamp_ns_t timestamp_ns;
    ASSERT_EQ(rag3db_timestamp_ns_from_tm(tm, &timestamp_ns), Rag3dbSuccess);
    ASSERT_EQ(timestamp_ns.value, 17515323000000000);
    tm.tm_year = 102;
    tm.tm_mon = 0;
    tm.tm_mday = 29;
    tm.tm_hour = 16;
    tm.tm_min = 57;
    tm.tm_sec = 15;
    rag3db_timestamp_ms_t timestamp_ms;
    ASSERT_EQ(rag3db_timestamp_ms_from_tm(tm, &timestamp_ms), Rag3dbSuccess);
    ASSERT_EQ(timestamp_ms.value, 1012323435000);
    tm.tm_year = 115;
    tm.tm_mon = 4;
    tm.tm_mday = 20;
    tm.tm_hour = 15;
    tm.tm_min = 27;
    tm.tm_sec = 28;
    rag3db_timestamp_sec_t timestamp_sec;
    ASSERT_EQ(rag3db_timestamp_sec_from_tm(tm, &timestamp_sec), Rag3dbSuccess);
    ASSERT_EQ(timestamp_sec.value, 1432135648);
    tm.tm_year = 94;
    tm.tm_mon = 5;
    tm.tm_mday = 13;
    tm.tm_hour = 13;
    tm.tm_min = 18;
    tm.tm_sec = 52;
    rag3db_timestamp_tz_t timestamp_tz;
    ASSERT_EQ(rag3db_timestamp_tz_from_tm(tm, &timestamp_tz), Rag3dbSuccess);
    ASSERT_EQ(timestamp_tz.value, 771513532000000);
}

TEST_F(CApiValueTest, GetDateFromTm) {
    struct tm tm;
    tm.tm_year = 69;
    tm.tm_mon = 3;
    tm.tm_mday = 21;
    tm.tm_hour = 0;
    tm.tm_min = 0;
    tm.tm_sec = 0;
    rag3db_date_t date;
    ASSERT_EQ(rag3db_date_from_tm(tm, &date), Rag3dbSuccess);
    ASSERT_EQ(date.days, -255);
}

TEST_F(CApiValueTest, GetDateFromString) {
    char input[] = "1969-04-21";
    rag3db_date_t date;
    ASSERT_EQ(rag3db_date_from_string(input, &date), Rag3dbSuccess);
    ASSERT_EQ(date.days, -255);

    char badInput[] = "this is not a date";
    ASSERT_EQ(rag3db_date_from_string(badInput, &date), Rag3dbError);
}

TEST_F(CApiValueTest, GetStringFromDate) {
    rag3db_date_t date = rag3db_date_t{-255};
    char* str;
    ASSERT_EQ(rag3db_date_to_string(date, &str), Rag3dbSuccess);
    ASSERT_STREQ(str, "1969-04-21");
    rag3db_destroy_string(str);
}

TEST_F(CApiValueTest, GetDifftimeFromInterval) {
    rag3db_interval_t interval = rag3db_interval_t{36, 2, 46920000000};
    double difftime;
    rag3db_interval_to_difftime(interval, &difftime);
    ASSERT_DOUBLE_EQ(difftime, 93531720);
}

TEST_F(CApiValueTest, GetIntervalFromDifftime) {
    double difftime = 211110160.479;
    rag3db_interval_t interval;
    rag3db_interval_from_difftime(difftime, &interval);
    ASSERT_EQ(interval.months, 81);
    ASSERT_EQ(interval.days, 13);
    ASSERT_EQ(interval.micros, 34960479000);
}
