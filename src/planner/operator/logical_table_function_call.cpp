#include "planner/operator/logical_table_function_call.h"

namespace rag3db {
namespace planner {

void LogicalTableFunctionCall::computeFlatSchema() {
    createEmptySchema();
    auto groupPos = schema->createGroup();
    for (auto& expr : bindData->columns) {
        schema->insertToGroupAndScope(expr, groupPos);
    }
}

void LogicalTableFunctionCall::computeFactorizedSchema() {
    createEmptySchema();
    auto groupPos = schema->createGroup();
    for (auto& expr : bindData->columns) {
        schema->insertToGroupAndScope(expr, groupPos);
    }
}

} // namespace planner
} // namespace rag3db
