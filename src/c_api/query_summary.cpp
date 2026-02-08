#include "main/query_summary.h"

#include <cstdlib>

#include "c_api/rag3db.h"

using namespace rag3db::main;

void rag3db_query_summary_destroy(rag3db_query_summary* query_summary) {
    if (query_summary == nullptr) {
        return;
    }
    // The query summary is owned by the query result, so it should not be deleted here.
    query_summary->_query_summary = nullptr;
}

double rag3db_query_summary_get_compiling_time(rag3db_query_summary* query_summary) {
    return static_cast<QuerySummary*>(query_summary->_query_summary)->getCompilingTime();
}

double rag3db_query_summary_get_execution_time(rag3db_query_summary* query_summary) {
    return static_cast<QuerySummary*>(query_summary->_query_summary)->getExecutionTime();
}
