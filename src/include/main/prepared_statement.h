#pragma once

#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

#include "common/api.h"
#include "common/types/value/value.h"
#include "query_summary.h"

namespace rag3db {
namespace common {
class LogicalType;
}
namespace parser {
class Statement;
}
namespace binder {
class Expression;
}
namespace planner {
class LogicalPlan;
}

namespace main {

// The bound query of a prepared statement. It lives with the PreparedStatement that holds it and
// is never serialized to the client side.
struct CachedPreparedStatement {
    bool useInternalCatalogEntry = false;
    std::shared_ptr<parser::Statement> parsedStatement;
    std::unique_ptr<planner::LogicalPlan> logicalPlan;
    std::vector<std::shared_ptr<binder::Expression>> columns;

    CachedPreparedStatement();
    ~CachedPreparedStatement();

    std::vector<std::string> getColumnNames() const;
    std::vector<common::LogicalType> getColumnTypes() const;
};

/**
 * @brief A prepared statement is a parameterized query which can avoid planning the same query for
 * repeated execution.
 */
class PreparedStatement {
    friend class Connection;
    friend class ClientContext;

public:
    RAG3DB_API ~PreparedStatement();
    /**
     * @return the query is prepared successfully or not.
     */
    RAG3DB_API bool isSuccess() const;
    /**
     * @return the error message if the query is not prepared successfully.
     */
    RAG3DB_API std::string getErrorMessage() const;
    /**
     * @return the prepared statement is read-only or not.
     */
    RAG3DB_API bool isReadOnly() const;

    const std::unordered_set<std::string>& getUnknownParameters() const {
        return unknownParameters;
    }
    std::unordered_set<std::string> getKnownParameters();
    void updateParameter(const std::string& name, common::Value* value);
    void addParameter(const std::string& name, common::Value* value);

    // The bound query, for internal callers (tests read its logical plan). Null for a statement
    // that failed to prepare.
    CachedPreparedStatement* getCachedPreparedStatement() const {
        return cachedPreparedStatement.get();
    }

    common::StatementType getStatementType() const;

    static std::unique_ptr<PreparedStatement> getPreparedStatementWithError(
        const std::string& errorMessage);

private:
    bool success = true;
    bool readOnly = true;
    std::string errMsg;
    PreparedSummary preparedSummary;
    // Held by the statement itself, not by its connection: it is freed with the statement, even
    // after the connection or the database is gone, and a statement executed on another
    // connection runs its own query. The connection used to keep every prepared query in a store
    // indexed by a counter, which nothing emptied (ticket 2026-10-25-chaque-requete-preparee-
    // reste-en-memoire-pour-la-vie-de-sa-connexion).
    std::shared_ptr<CachedPreparedStatement> cachedPreparedStatement;
    std::unordered_set<std::string> unknownParameters;
    std::unordered_map<std::string, std::shared_ptr<common::Value>> parameterMap;
};

} // namespace main
} // namespace rag3db
