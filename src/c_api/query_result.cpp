#include "main/query_result.h"

#include "c_api/helpers.h"
#include "c_api/rag3db.h"

using namespace rag3db::main;
using namespace rag3db::common;
using namespace rag3db::processor;

void rag3db_query_result_destroy(rag3db_query_result* query_result) {
    if (query_result == nullptr) {
        return;
    }
    if (query_result->_query_result != nullptr) {
        if (!query_result->_is_owned_by_cpp) {
            delete static_cast<QueryResult*>(query_result->_query_result);
        }
    }
}

bool rag3db_query_result_is_success(rag3db_query_result* query_result) {
    return static_cast<QueryResult*>(query_result->_query_result)->isSuccess();
}

char* rag3db_query_result_get_error_message(rag3db_query_result* query_result) {
    auto error_message = static_cast<QueryResult*>(query_result->_query_result)->getErrorMessage();
    if (error_message.empty()) {
        return nullptr;
    }
    return convertToOwnedCString(error_message);
}

uint64_t rag3db_query_result_get_num_columns(rag3db_query_result* query_result) {
    return static_cast<QueryResult*>(query_result->_query_result)->getNumColumns();
}

rag3db_state rag3db_query_result_get_column_name(rag3db_query_result* query_result, uint64_t index,
    char** out_column_name) {
    auto column_names = static_cast<QueryResult*>(query_result->_query_result)->getColumnNames();
    if (index >= column_names.size()) {
        return Rag3dbError;
    }
    *out_column_name = convertToOwnedCString(column_names[index]);
    return Rag3dbSuccess;
}

rag3db_state rag3db_query_result_get_column_data_type(rag3db_query_result* query_result, uint64_t index,
    rag3db_logical_type* out_column_data_type) {
    auto column_data_types =
        static_cast<QueryResult*>(query_result->_query_result)->getColumnDataTypes();
    if (index >= column_data_types.size()) {
        return Rag3dbError;
    }
    const auto& column_data_type = column_data_types[index];
    out_column_data_type->_data_type = new LogicalType(column_data_type.copy());
    return Rag3dbSuccess;
}

uint64_t rag3db_query_result_get_num_tuples(rag3db_query_result* query_result) {
    return static_cast<QueryResult*>(query_result->_query_result)->getNumTuples();
}

rag3db_state rag3db_query_result_get_query_summary(rag3db_query_result* query_result,
    rag3db_query_summary* out_query_summary) {
    if (out_query_summary == nullptr) {
        return Rag3dbError;
    }
    auto query_summary = static_cast<QueryResult*>(query_result->_query_result)->getQuerySummary();
    out_query_summary->_query_summary = query_summary;
    return Rag3dbSuccess;
}

bool rag3db_query_result_has_next(rag3db_query_result* query_result) {
    return static_cast<QueryResult*>(query_result->_query_result)->hasNext();
}

bool rag3db_query_result_has_next_query_result(rag3db_query_result* query_result) {
    return static_cast<QueryResult*>(query_result->_query_result)->hasNextQueryResult();
}

rag3db_state rag3db_query_result_get_next_query_result(rag3db_query_result* query_result,
    rag3db_query_result* out_query_result) {
    if (!rag3db_query_result_has_next_query_result(query_result)) {
        return Rag3dbError;
    }
    auto next_query_result =
        static_cast<QueryResult*>(query_result->_query_result)->getNextQueryResult();
    if (next_query_result == nullptr) {
        return Rag3dbError;
    }
    out_query_result->_query_result = next_query_result;
    out_query_result->_is_owned_by_cpp = true;
    return Rag3dbSuccess;
}

rag3db_state rag3db_query_result_get_next(rag3db_query_result* query_result,
    rag3db_flat_tuple* out_flat_tuple) {
    try {
        auto flat_tuple = static_cast<QueryResult*>(query_result->_query_result)->getNext();
        out_flat_tuple->_flat_tuple = flat_tuple.get();
        out_flat_tuple->_is_owned_by_cpp = true;
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

char* rag3db_query_result_to_string(rag3db_query_result* query_result) {
    std::string result_string = static_cast<QueryResult*>(query_result->_query_result)->toString();
    return convertToOwnedCString(result_string);
}

void rag3db_query_result_reset_iterator(rag3db_query_result* query_result) {
    static_cast<QueryResult*>(query_result->_query_result)->resetIterator();
}

rag3db_state rag3db_query_result_get_arrow_schema(rag3db_query_result* query_result,
    ArrowSchema* out_schema) {
    try {
        *out_schema = *static_cast<QueryResult*>(query_result->_query_result)->getArrowSchema();
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_query_result_get_next_arrow_chunk(rag3db_query_result* query_result,
    int64_t chunk_size, ArrowArray* out_arrow_array) {
    try {
        *out_arrow_array =
            *static_cast<QueryResult*>(query_result->_query_result)->getNextArrowChunk(chunk_size);
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}
