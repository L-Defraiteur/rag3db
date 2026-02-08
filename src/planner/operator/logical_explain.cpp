#include "planner/operator/logical_explain.h"

using namespace rag3db::common;

namespace rag3db {
namespace planner {

void LogicalExplain::computeSchema() {
    switch (explainType) {
    case ExplainType::PROFILE:
        if (children[0]->getSchema()) {
            copyChildSchema(0);
        } else {
            createEmptySchema();
        }
        break;
    case ExplainType::PHYSICAL_PLAN:
    case ExplainType::LOGICAL_PLAN:
        createEmptySchema();
        break;
    default:
        KU_UNREACHABLE;
    }
}

void LogicalExplain::computeFlatSchema() {
    computeSchema();
}

void LogicalExplain::computeFactorizedSchema() {
    computeSchema();
}

} // namespace planner
} // namespace rag3db
