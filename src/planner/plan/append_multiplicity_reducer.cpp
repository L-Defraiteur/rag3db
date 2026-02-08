#include "planner/operator/logical_multiplcity_reducer.h"
#include "planner/planner.h"

namespace rag3db {
namespace planner {

void Planner::appendMultiplicityReducer(LogicalPlan& plan) {
    auto multiplicityReducer = make_shared<LogicalMultiplicityReducer>(plan.getLastOperator());
    multiplicityReducer->computeFactorizedSchema();
    plan.setLastOperator(std::move(multiplicityReducer));
}

} // namespace planner
} // namespace rag3db
