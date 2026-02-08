#include "c_api/rag3db.h"
#include "common/exception/exception.h"
#include "main/rag3db.h"

namespace rag3db {
namespace common {
class Value;
}
} // namespace rag3db

using namespace rag3db::common;
using namespace rag3db::main;

rag3db_state rag3db_connection_init(rag3db_database* database, rag3db_connection* out_connection) {
    if (database == nullptr || database->_database == nullptr) {
        out_connection->_connection = nullptr;
        return Rag3dbError;
    }
    try {
        out_connection->_connection = new Connection(static_cast<Database*>(database->_database));
    } catch (Exception& e) {
        out_connection->_connection = nullptr;
        return Rag3dbError;
    }
    return Rag3dbSuccess;
}

void rag3db_connection_destroy(rag3db_connection* connection) {
    if (connection == nullptr) {
        return;
    }
    if (connection->_connection != nullptr) {
        delete static_cast<Connection*>(connection->_connection);
    }
}

rag3db_state rag3db_connection_set_max_num_thread_for_exec(rag3db_connection* connection,
    uint64_t num_threads) {
    if (connection == nullptr || connection->_connection == nullptr) {
        return Rag3dbError;
    }
    try {
        static_cast<Connection*>(connection->_connection)->setMaxNumThreadForExec(num_threads);
    } catch (Exception& e) {
        return Rag3dbError;
    }
    return Rag3dbSuccess;
}

rag3db_state rag3db_connection_get_max_num_thread_for_exec(rag3db_connection* connection,
    uint64_t* out_result) {
    if (connection == nullptr || connection->_connection == nullptr) {
        return Rag3dbError;
    }
    try {
        *out_result = static_cast<Connection*>(connection->_connection)->getMaxNumThreadForExec();
    } catch (Exception& e) {
        return Rag3dbError;
    }
    return Rag3dbSuccess;
}

rag3db_state rag3db_connection_query(rag3db_connection* connection, const char* query,
    rag3db_query_result* out_query_result) {
    if (connection == nullptr || connection->_connection == nullptr) {
        return Rag3dbError;
    }
    try {
        auto query_result =
            static_cast<Connection*>(connection->_connection)->query(query).release();
        if (query_result == nullptr) {
            return Rag3dbError;
        }
        out_query_result->_query_result = query_result;
        out_query_result->_is_owned_by_cpp = false;
        if (!query_result->isSuccess()) {
            return Rag3dbError;
        }
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}

rag3db_state rag3db_connection_prepare(rag3db_connection* connection, const char* query,
    rag3db_prepared_statement* out_prepared_statement) {
    if (connection == nullptr || connection->_connection == nullptr) {
        return Rag3dbError;
    }
    try {
        auto prepared_statement =
            static_cast<Connection*>(connection->_connection)->prepare(query).release();
        if (prepared_statement == nullptr) {
            return Rag3dbError;
        }
        out_prepared_statement->_prepared_statement = prepared_statement;
        out_prepared_statement->_bound_values =
            new std::unordered_map<std::string, std::unique_ptr<Value>>;
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
    return Rag3dbSuccess;
}

rag3db_state rag3db_connection_execute(rag3db_connection* connection,
    rag3db_prepared_statement* prepared_statement, rag3db_query_result* out_query_result) {
    if (connection == nullptr || connection->_connection == nullptr ||
        prepared_statement == nullptr || prepared_statement->_prepared_statement == nullptr ||
        prepared_statement->_bound_values == nullptr) {
        return Rag3dbError;
    }
    try {
        auto prepared_statement_ptr =
            static_cast<PreparedStatement*>(prepared_statement->_prepared_statement);
        auto bound_values = static_cast<std::unordered_map<std::string, std::unique_ptr<Value>>*>(
            prepared_statement->_bound_values);

        // Must copy the parameters for safety, and so that the parameters in the prepared statement
        // stay the same.
        std::unordered_map<std::string, std::unique_ptr<Value>> copied_bound_values;
        for (auto& [name, value] : *bound_values) {
            copied_bound_values.emplace(name, value->copy());
        }

        auto query_result =
            static_cast<Connection*>(connection->_connection)
                ->executeWithParams(prepared_statement_ptr, std::move(copied_bound_values))
                .release();
        if (query_result == nullptr) {
            return Rag3dbError;
        }
        out_query_result->_query_result = query_result;
        out_query_result->_is_owned_by_cpp = false;
        if (!query_result->isSuccess()) {
            return Rag3dbError;
        }
        return Rag3dbSuccess;
    } catch (Exception& e) {
        return Rag3dbError;
    }
}
void rag3db_connection_interrupt(rag3db_connection* connection) {
    static_cast<Connection*>(connection->_connection)->interrupt();
}

rag3db_state rag3db_connection_set_query_timeout(rag3db_connection* connection, uint64_t timeout_in_ms) {
    if (connection == nullptr || connection->_connection == nullptr) {
        return Rag3dbError;
    }
    try {
        static_cast<Connection*>(connection->_connection)->setQueryTimeOut(timeout_in_ms);
    } catch (Exception& e) {
        return Rag3dbError;
    }
    return Rag3dbSuccess;
}
