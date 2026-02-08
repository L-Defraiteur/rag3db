#include "main/prepared_statement.h"

#include "c_api/helpers.h"
#include "c_api/rag3db.h"
#include "common/types/value/value.h"

using namespace rag3db::common;
using namespace rag3db::main;

void rag3db_prepared_statement_bind_cpp_value(rag3db_prepared_statement* prepared_statement,
    const char* param_name, std::unique_ptr<Value> value) {
    auto* bound_values = static_cast<std::unordered_map<std::string, std::unique_ptr<Value>>*>(
        prepared_statement->_bound_values);
    bound_values->erase(param_name);
    bound_values->insert({param_name, std::move(value)});
}

void rag3db_prepared_statement_destroy(rag3db_prepared_statement* prepared_statement) {
    if (prepared_statement == nullptr) {
        return;
    }
    if (prepared_statement->_prepared_statement != nullptr) {
        delete static_cast<PreparedStatement*>(prepared_statement->_prepared_statement);
    }
    if (prepared_statement->_bound_values != nullptr) {
        delete static_cast<std::unordered_map<std::string, std::unique_ptr<Value>>*>(
            prepared_statement->_bound_values);
    }
}

bool rag3db_prepared_statement_is_success(rag3db_prepared_statement* prepared_statement) {
    return static_cast<PreparedStatement*>(prepared_statement->_prepared_statement)->isSuccess();
}

char* rag3db_prepared_statement_get_error_message(rag3db_prepared_statement* prepared_statement) {
    auto error_message =
        static_cast<PreparedStatement*>(prepared_statement->_prepared_statement)->getErrorMessage();
    if (error_message.empty()) {
        return nullptr;
    }
    return convertToOwnedCString(error_message);
}

rag3db_state rag3db_prepared_statement_bind_bool(rag3db_prepared_statement* prepared_statement,
    const char* param_name, bool value) {
    try {
        auto value_ptr = std::make_unique<Value>(value);
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_int64(rag3db_prepared_statement* prepared_statement,
    const char* param_name, int64_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(value);
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_int32(rag3db_prepared_statement* prepared_statement,
    const char* param_name, int32_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(value);
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_int16(rag3db_prepared_statement* prepared_statement,
    const char* param_name, int16_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(value);
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_int8(rag3db_prepared_statement* prepared_statement,
    const char* param_name, int8_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(value);
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_uint64(rag3db_prepared_statement* prepared_statement,
    const char* param_name, uint64_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(value);
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_uint32(rag3db_prepared_statement* prepared_statement,
    const char* param_name, uint32_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(value);
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_uint16(rag3db_prepared_statement* prepared_statement,
    const char* param_name, uint16_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(value);
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_uint8(rag3db_prepared_statement* prepared_statement,
    const char* param_name, uint8_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(value);
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_double(rag3db_prepared_statement* prepared_statement,
    const char* param_name, double value) {
    try {
        auto value_ptr = std::make_unique<Value>(value);
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_float(rag3db_prepared_statement* prepared_statement,
    const char* param_name, float value) {
    try {
        auto value_ptr = std::make_unique<Value>(value);
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_date(rag3db_prepared_statement* prepared_statement,
    const char* param_name, rag3db_date_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(date_t(value.days));
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_timestamp_ns(rag3db_prepared_statement* prepared_statement,
    const char* param_name, rag3db_timestamp_ns_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(timestamp_ns_t(value.value));
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_timestamp_ms(rag3db_prepared_statement* prepared_statement,
    const char* param_name, rag3db_timestamp_ms_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(timestamp_ms_t(value.value));
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_timestamp_sec(rag3db_prepared_statement* prepared_statement,
    const char* param_name, rag3db_timestamp_sec_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(timestamp_sec_t(value.value));
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_timestamp_tz(rag3db_prepared_statement* prepared_statement,
    const char* param_name, rag3db_timestamp_tz_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(timestamp_tz_t(value.value));
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_timestamp(rag3db_prepared_statement* prepared_statement,
    const char* param_name, rag3db_timestamp_t value) {
    try {
        auto value_ptr = std::make_unique<Value>(timestamp_t(value.value));
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_interval(rag3db_prepared_statement* prepared_statement,
    const char* param_name, rag3db_interval_t value) {
    try {
        auto value_ptr =
            std::make_unique<Value>(interval_t(value.months, value.days, value.micros));
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_string(rag3db_prepared_statement* prepared_statement,
    const char* param_name, const char* value) {
    try {
        auto value_ptr = std::make_unique<Value>(std::string(value));
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_prepared_statement_bind_value(rag3db_prepared_statement* prepared_statement,
    const char* param_name, rag3db_value* value) {
    try {
        auto value_ptr = std::make_unique<Value>(*static_cast<Value*>(value->_value));
        rag3db_prepared_statement_bind_cpp_value(prepared_statement, param_name,
            std::move(value_ptr));
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}
