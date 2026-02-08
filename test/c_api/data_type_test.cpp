#include "c_api/rag3db.h"
#include "common/types/types.h"
#include "gtest/gtest.h"

using namespace rag3db::common;

TEST(CApiDataTypeTest, Create) {
    rag3db_logical_type dataType;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_INT64, nullptr, 0, &dataType);
    ASSERT_NE(dataType._data_type, nullptr);
    auto dataTypeCpp = (LogicalType*)dataType._data_type;
    ASSERT_EQ(dataTypeCpp->getLogicalTypeID(), LogicalTypeID::INT64);

    rag3db_logical_type dataType2;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_LIST, &dataType, 0, &dataType2);
    ASSERT_NE(dataType2._data_type, nullptr);
    auto dataTypeCpp2 = (LogicalType*)dataType2._data_type;
    ASSERT_EQ(dataTypeCpp2->getLogicalTypeID(), LogicalTypeID::LIST);
    // ASSERT_EQ(dataTypeCpp2->getChildType()->getLogicalTypeID(), LogicalTypeID::INT64);

    rag3db_logical_type dataType3;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_ARRAY, &dataType, 100, &dataType3);
    ASSERT_NE(dataType3._data_type, nullptr);
    auto dataTypeCpp3 = (LogicalType*)dataType3._data_type;
    ASSERT_EQ(dataTypeCpp3->getLogicalTypeID(), LogicalTypeID::ARRAY);
    // ASSERT_EQ(dataTypeCpp3->getChildType()->getLogicalTypeID(), LogicalTypeID::INT64);
    ASSERT_EQ(ArrayType::getNumElements(*dataTypeCpp3), 100);

    // Since child type is copied, we should be able to destroy the original type without an error.
    rag3db_data_type_destroy(&dataType);
    rag3db_data_type_destroy(&dataType2);
    rag3db_data_type_destroy(&dataType3);
}

TEST(CApiDataTypeTest, Clone) {
    rag3db_logical_type dataType;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_INT64, nullptr, 0, &dataType);
    ASSERT_NE(dataType._data_type, nullptr);
    rag3db_logical_type dataTypeClone;
    rag3db_data_type_clone(&dataType, &dataTypeClone);
    ASSERT_NE(dataTypeClone._data_type, nullptr);
    auto dataTypeCpp = (LogicalType*)dataType._data_type;
    auto dataTypeCloneCpp = (LogicalType*)dataTypeClone._data_type;
    ASSERT_TRUE(*dataTypeCpp == *dataTypeCloneCpp);

    rag3db_logical_type dataType2;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_LIST, &dataType, 0, &dataType2);
    ASSERT_NE(dataType2._data_type, nullptr);
    rag3db_logical_type dataTypeClone2;
    rag3db_data_type_clone(&dataType2, &dataTypeClone2);
    ASSERT_NE(dataTypeClone2._data_type, nullptr);
    auto dataTypeCpp2 = (LogicalType*)dataType2._data_type;
    auto dataTypeCloneCpp2 = (LogicalType*)dataTypeClone2._data_type;
    ASSERT_TRUE(*dataTypeCpp2 == *dataTypeCloneCpp2);

    rag3db_logical_type dataType3;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_ARRAY, &dataType, 100, &dataType3);
    ASSERT_NE(dataType3._data_type, nullptr);
    rag3db_logical_type dataTypeClone3;
    rag3db_data_type_clone(&dataType3, &dataTypeClone3);
    ASSERT_NE(dataTypeClone3._data_type, nullptr);
    auto dataTypeCpp3 = (LogicalType*)dataType3._data_type;
    auto dataTypeCloneCpp3 = (LogicalType*)dataTypeClone3._data_type;
    ASSERT_TRUE(*dataTypeCpp3 == *dataTypeCloneCpp3);

    rag3db_data_type_destroy(&dataType);
    rag3db_data_type_destroy(&dataType2);
    rag3db_data_type_destroy(&dataType3);
    rag3db_data_type_destroy(&dataTypeClone);
    rag3db_data_type_destroy(&dataTypeClone2);
    rag3db_data_type_destroy(&dataTypeClone3);
}

TEST(CApiDataTypeTest, Equals) {
    rag3db_logical_type dataType;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_INT64, nullptr, 0, &dataType);
    ASSERT_NE(dataType._data_type, nullptr);
    rag3db_logical_type dataTypeClone;
    rag3db_data_type_clone(&dataType, &dataTypeClone);
    ASSERT_NE(dataTypeClone._data_type, nullptr);
    ASSERT_TRUE(rag3db_data_type_equals(&dataType, &dataTypeClone));

    rag3db_logical_type dataType2;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_LIST, &dataType, 0, &dataType2);
    ASSERT_NE(dataType2._data_type, nullptr);
    rag3db_logical_type dataTypeClone2;
    rag3db_data_type_clone(&dataType2, &dataTypeClone2);
    ASSERT_NE(dataTypeClone2._data_type, nullptr);
    ASSERT_TRUE(rag3db_data_type_equals(&dataType2, &dataTypeClone2));

    rag3db_logical_type dataType3;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_ARRAY, &dataType, 100, &dataType3);
    ASSERT_NE(dataType3._data_type, nullptr);
    rag3db_logical_type dataTypeClone3;
    rag3db_data_type_clone(&dataType3, &dataTypeClone3);
    ASSERT_NE(dataTypeClone3._data_type, nullptr);
    ASSERT_TRUE(rag3db_data_type_equals(&dataType3, &dataTypeClone3));

    ASSERT_FALSE(rag3db_data_type_equals(&dataType, &dataType2));
    ASSERT_FALSE(rag3db_data_type_equals(&dataType, &dataType3));
    ASSERT_FALSE(rag3db_data_type_equals(&dataType2, &dataType3));

    rag3db_data_type_destroy(&dataType);
    rag3db_data_type_destroy(&dataType2);
    rag3db_data_type_destroy(&dataType3);
    rag3db_data_type_destroy(&dataTypeClone);
    rag3db_data_type_destroy(&dataTypeClone2);
    rag3db_data_type_destroy(&dataTypeClone3);
}

TEST(CApiDataTypeTest, GetID) {
    rag3db_logical_type dataType;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_INT64, nullptr, 0, &dataType);
    ASSERT_NE(dataType._data_type, nullptr);
    ASSERT_EQ(rag3db_data_type_get_id(&dataType), rag3db_data_type_id::RAG3DB_INT64);

    rag3db_logical_type dataType2;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_LIST, &dataType, 0, &dataType2);
    ASSERT_NE(dataType2._data_type, nullptr);
    ASSERT_EQ(rag3db_data_type_get_id(&dataType2), rag3db_data_type_id::RAG3DB_LIST);

    rag3db_logical_type dataType3;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_ARRAY, &dataType, 100, &dataType3);
    ASSERT_NE(dataType3._data_type, nullptr);
    ASSERT_EQ(rag3db_data_type_get_id(&dataType3), rag3db_data_type_id::RAG3DB_ARRAY);

    rag3db_data_type_destroy(&dataType);
    rag3db_data_type_destroy(&dataType2);
    rag3db_data_type_destroy(&dataType3);
}

// TODO(Chang): The getChildType interface has been removed from the C++ DataType class.
// Consider adding the StructType/ListType helper to C binding.
// TEST(CApiDataTypeTest, GetChildType) {
//    auto dataType = rag3db_data_type_create(rag3db_data_type_id::RAG3DB_INT64, nullptr, 0);
//    ASSERT_NE(dataType, nullptr);
//    ASSERT_EQ(rag3db_data_type_get_child_type(dataType), nullptr);
//
//    auto dataType2 = rag3db_data_type_create(rag3db_data_type_id::RAG3DB_LIST, dataType, 0);
//    ASSERT_NE(dataType2, nullptr);
//    auto childType2 = rag3db_data_type_get_child_type(dataType2);
//    ASSERT_NE(childType2, nullptr);
//    ASSERT_EQ(rag3db_data_type_get_id(childType2), rag3db_data_type_id::RAG3DB_INT64);
//    rag3db_data_type_destroy(childType2);
//    rag3db_data_type_destroy(dataType2);
//
//    auto dataType3 = rag3db_data_type_create(rag3db_data_type_id::RAG3DB_ARRAY, dataType, 100);
//    ASSERT_NE(dataType3, nullptr);
//    auto childType3 = rag3db_data_type_get_child_type(dataType3);
//    rag3db_data_type_destroy(dataType3);
//    // Destroying dataType3 should not destroy childType3.
//    ASSERT_NE(childType3, nullptr);
//    ASSERT_EQ(rag3db_data_type_get_id(childType3), rag3db_data_type_id::RAG3DB_INT64);
//    rag3db_data_type_destroy(childType3);
//
//    rag3db_data_type_destroy(dataType);
//}

TEST(CApiDataTypeTest, GetFixedNumElementsInList) {
    rag3db_logical_type dataType;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_INT64, nullptr, 0, &dataType);
    ASSERT_NE(dataType._data_type, nullptr);
    uint64_t numElements;
    ASSERT_EQ(rag3db_data_type_get_num_elements_in_array(&dataType, &numElements), Rag3dbError);

    rag3db_logical_type dataType2;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_LIST, &dataType, 0, &dataType2);
    ASSERT_NE(dataType2._data_type, nullptr);
    ASSERT_EQ(rag3db_data_type_get_num_elements_in_array(&dataType2, &numElements), Rag3dbError);

    rag3db_logical_type dataType3;
    rag3db_data_type_create(rag3db_data_type_id::RAG3DB_ARRAY, &dataType, 100, &dataType3);
    ASSERT_NE(dataType3._data_type, nullptr);
    ASSERT_EQ(rag3db_data_type_get_num_elements_in_array(&dataType3, &numElements), Rag3dbSuccess);
    ASSERT_EQ(numElements, 100);

    rag3db_data_type_destroy(&dataType);
    rag3db_data_type_destroy(&dataType2);
    rag3db_data_type_destroy(&dataType3);
}
