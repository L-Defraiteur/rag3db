#pragma once

#include <cstdint>
#include <memory>

#include "rust/cxx.h"
#ifdef RAG3DB_BUNDLED
#include "common/type_utils.h"
#include "common/types/int128_t.h"
#include "common/types/types.h"
#include "common/types/value/nested.h"
#include "common/types/value/node.h"
#include "common/types/value/recursive_rel.h"
#include "common/types/value/rel.h"
#include "common/types/value/value.h"
#include "main/rag3db.h"
#include "storage/storage_version_info.h"
#else
#include <rag3db.hpp>
#endif

namespace rag3db_rs {

struct TypeListBuilder {
    std::vector<rag3db::common::LogicalType> types;

    void insert(std::unique_ptr<rag3db::common::LogicalType> type) {
        types.push_back(std::move(*type));
    }
};

std::unique_ptr<TypeListBuilder> create_type_list();

struct QueryParams {
    std::unordered_map<std::string, std::unique_ptr<rag3db::common::Value>> inputParams;

    void insert(const rust::Str key, std::unique_ptr<rag3db::common::Value> value) {
        inputParams.insert(std::make_pair(key, std::move(value)));
    }
};

std::unique_ptr<QueryParams> new_params();

std::unique_ptr<rag3db::common::LogicalType> create_logical_type(rag3db::common::LogicalTypeID id);
std::unique_ptr<rag3db::common::LogicalType> create_logical_type_list(
    std::unique_ptr<rag3db::common::LogicalType> childType);
std::unique_ptr<rag3db::common::LogicalType> create_logical_type_array(
    std::unique_ptr<rag3db::common::LogicalType> childType, uint64_t numElements);

inline std::unique_ptr<rag3db::common::LogicalType> create_logical_type_struct(
    const rust::Vec<rust::String>& fieldNames, std::unique_ptr<TypeListBuilder> fieldTypes) {
    std::vector<rag3db::common::StructField> fields;
    for (auto i = 0u; i < fieldNames.size(); i++) {
        fields.emplace_back(std::string(fieldNames[i]), std::move(fieldTypes->types[i]));
    }
    return std::make_unique<rag3db::common::LogicalType>(
        rag3db::common::LogicalType::STRUCT(std::move(fields)));
}
inline std::unique_ptr<rag3db::common::LogicalType> create_logical_type_union(
    const rust::Vec<rust::String>& fieldNames, std::unique_ptr<TypeListBuilder> fieldTypes) {
    std::vector<rag3db::common::StructField> fields;
    for (auto i = 0u; i < fieldNames.size(); i++) {
        fields.emplace_back(std::string(fieldNames[i]), std::move(fieldTypes->types[i]));
    }
    return std::make_unique<rag3db::common::LogicalType>(
        rag3db::common::LogicalType::UNION(std::move(fields)));
}
std::unique_ptr<rag3db::common::LogicalType> create_logical_type_map(
    std::unique_ptr<rag3db::common::LogicalType> keyType,
    std::unique_ptr<rag3db::common::LogicalType> valueType);

inline std::unique_ptr<rag3db::common::LogicalType> create_logical_type_decimal(uint32_t precision,
    uint32_t scale) {
    return std::make_unique<rag3db::common::LogicalType>(
        rag3db::common::LogicalType::DECIMAL(precision, scale));
}

std::unique_ptr<rag3db::common::LogicalType> logical_type_get_list_child_type(
    const rag3db::common::LogicalType& logicalType);
std::unique_ptr<rag3db::common::LogicalType> logical_type_get_array_child_type(
    const rag3db::common::LogicalType& logicalType);
uint64_t logical_type_get_array_num_elements(const rag3db::common::LogicalType& logicalType);

rust::Vec<rust::String> logical_type_get_struct_field_names(const rag3db::common::LogicalType& value);
std::unique_ptr<std::vector<rag3db::common::LogicalType>> logical_type_get_struct_field_types(
    const rag3db::common::LogicalType& value);

inline uint32_t logical_type_get_decimal_precision(const rag3db::common::LogicalType& logicalType) {
    return rag3db::common::DecimalType::getPrecision(logicalType);
}
inline uint32_t logical_type_get_decimal_scale(const rag3db::common::LogicalType& logicalType) {
    return rag3db::common::DecimalType::getScale(logicalType);
}

/* Database */
std::unique_ptr<rag3db::main::Database> new_database(std::string_view databasePath,
    uint64_t bufferPoolSize, uint64_t maxNumThreads, bool enableCompression, bool readOnly,
    uint64_t maxDBSize, bool autoCheckpoint, int64_t checkpointThreshold,
    bool throwOnWalReplayFailure, bool enableChecksums);

void database_set_logging_level(rag3db::main::Database& database, const std::string& level);

/* Connection */
std::unique_ptr<rag3db::main::Connection> database_connect(rag3db::main::Database& database);
std::unique_ptr<rag3db::main::QueryResult> connection_execute(rag3db::main::Connection& connection,
    rag3db::main::PreparedStatement& query, std::unique_ptr<QueryParams> params);
inline std::unique_ptr<rag3db::main::QueryResult> connection_query(rag3db::main::Connection& connection,
    std::string_view query) {
    return connection.query(query);
}

/* PreparedStatement */
rust::String prepared_statement_error_message(const rag3db::main::PreparedStatement& statement);

/* QueryResult */
rust::String query_result_to_string(const rag3db::main::QueryResult& result);
rust::String query_result_get_error_message(const rag3db::main::QueryResult& result);

double query_result_get_compiling_time(const rag3db::main::QueryResult& result);
double query_result_get_execution_time(const rag3db::main::QueryResult& result);

std::unique_ptr<std::vector<rag3db::common::LogicalType>> query_result_column_data_types(
    const rag3db::main::QueryResult& query_result);
rust::Vec<rust::String> query_result_column_names(const rag3db::main::QueryResult& query_result);

/* NodeVal/RelVal */
rust::String node_value_get_label_name(const rag3db::common::Value& val);
rust::String rel_value_get_label_name(const rag3db::common::Value& val);

size_t node_value_get_num_properties(const rag3db::common::Value& value);
size_t rel_value_get_num_properties(const rag3db::common::Value& value);

rust::String node_value_get_property_name(const rag3db::common::Value& value, size_t index);
rust::String rel_value_get_property_name(const rag3db::common::Value& value, size_t index);

const rag3db::common::Value& node_value_get_property_value(const rag3db::common::Value& value,
    size_t index);
const rag3db::common::Value& rel_value_get_property_value(const rag3db::common::Value& value,
    size_t index);

/* NodeVal */
const rag3db::common::Value& node_value_get_node_id(const rag3db::common::Value& val);

/* RelVal */
const rag3db::common::Value& rel_value_get_src_id(const rag3db::common::Value& val);
std::array<uint64_t, 2> rel_value_get_dst_id(const rag3db::common::Value& val);

/* RecursiveRel */
const rag3db::common::Value& recursive_rel_get_nodes(const rag3db::common::Value& val);
const rag3db::common::Value& recursive_rel_get_rels(const rag3db::common::Value& val);

/* FlatTuple */
const rag3db::common::Value& flat_tuple_get_value(const rag3db::processor::FlatTuple& flatTuple,
    uint32_t index);

/* Value */
const std::string& value_get_string(const rag3db::common::Value& value);

template<typename T>
std::unique_ptr<T> value_get_unique(const rag3db::common::Value& value) {
    return std::make_unique<T>(value.getValue<T>());
}

int64_t value_get_interval_secs(const rag3db::common::Value& value);
int32_t value_get_interval_micros(const rag3db::common::Value& value);
int32_t value_get_date_days(const rag3db::common::Value& value);
int64_t value_get_timestamp_ns(const rag3db::common::Value& value);
int64_t value_get_timestamp_ms(const rag3db::common::Value& value);
int64_t value_get_timestamp_sec(const rag3db::common::Value& value);
int64_t value_get_timestamp_micros(const rag3db::common::Value& value);
int64_t value_get_timestamp_tz(const rag3db::common::Value& value);
std::array<uint64_t, 2> value_get_int128_t(const rag3db::common::Value& value);
std::array<uint64_t, 2> value_get_internal_id(const rag3db::common::Value& value);
uint32_t value_get_children_size(const rag3db::common::Value& value);
const rag3db::common::Value& value_get_child(const rag3db::common::Value& value, uint32_t index);
rag3db::common::LogicalTypeID value_get_data_type_id(const rag3db::common::Value& value);
const rag3db::common::LogicalType& value_get_data_type(const rag3db::common::Value& value);
inline rag3db::common::PhysicalTypeID value_get_physical_type(const rag3db::common::Value& value) {
    return value.getDataType().getPhysicalType();
}
rust::String value_to_string(const rag3db::common::Value& val);

std::unique_ptr<rag3db::common::Value> create_value_string(rag3db::common::LogicalTypeID typ,
    const rust::Slice<const unsigned char> value);
std::unique_ptr<rag3db::common::Value> create_value_timestamp(const int64_t timestamp);
std::unique_ptr<rag3db::common::Value> create_value_timestamp_tz(const int64_t timestamp);
std::unique_ptr<rag3db::common::Value> create_value_timestamp_ns(const int64_t timestamp);
std::unique_ptr<rag3db::common::Value> create_value_timestamp_ms(const int64_t timestamp);
std::unique_ptr<rag3db::common::Value> create_value_timestamp_sec(const int64_t timestamp);
inline std::unique_ptr<rag3db::common::Value> create_value_date(const int32_t date) {
    return std::make_unique<rag3db::common::Value>(rag3db::common::date_t(date));
}
std::unique_ptr<rag3db::common::Value> create_value_interval(const int32_t months, const int32_t days,
    const int64_t micros);
std::unique_ptr<rag3db::common::Value> create_value_null(
    std::unique_ptr<rag3db::common::LogicalType> typ);
std::unique_ptr<rag3db::common::Value> create_value_int128_t(int64_t high, uint64_t low);
std::unique_ptr<rag3db::common::Value> create_value_internal_id(uint64_t offset, uint64_t table);

inline std::unique_ptr<rag3db::common::Value> create_value_uuid_t(int64_t high, uint64_t low) {
    return std::make_unique<rag3db::common::Value>(
        rag3db::common::ku_uuid_t{rag3db::common::int128_t(low, high)});
}

template<typename T>
std::unique_ptr<rag3db::common::Value> create_value(const T value) {
    return std::make_unique<rag3db::common::Value>(value);
}
inline std::unique_ptr<rag3db::common::Value> create_value_decimal(int64_t high, uint64_t low,
    uint32_t scale, uint32_t precision) {
    auto value =
        std::make_unique<rag3db::common::Value>(rag3db::common::LogicalType::DECIMAL(precision, scale),
            std::vector<std::unique_ptr<rag3db::common::Value>>{});
    auto i128 = rag3db::common::int128_t(low, high);
    rag3db::common::TypeUtils::visit(
        value->getDataType().getPhysicalType(),
        [&](rag3db::common::int128_t) { value->val.int128Val = i128; },
        [&](int64_t) { value->val.int64Val = static_cast<int64_t>(i128); },
        [&](int32_t) { value->val.int32Val = static_cast<int32_t>(i128); },
        [&](int16_t) { value->val.int16Val = static_cast<int16_t>(i128); },
        [](auto) { KU_UNREACHABLE; });
    return value;
}

struct ValueListBuilder {
    std::vector<std::unique_ptr<rag3db::common::Value>> values;

    void insert(std::unique_ptr<rag3db::common::Value> value) { values.push_back(std::move(value)); }
};

std::unique_ptr<rag3db::common::Value> get_list_value(std::unique_ptr<rag3db::common::LogicalType> typ,
    std::unique_ptr<ValueListBuilder> value);
std::unique_ptr<ValueListBuilder> create_list();

inline std::string_view string_view_from_str(rust::Str s) {
    return {s.data(), s.size()};
}

inline rag3db::storage::storage_version_t get_storage_version() {
    return rag3db::storage::StorageVersionInfo::getStorageVersion();
}

} // namespace rag3db_rs
