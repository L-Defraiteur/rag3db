#include "planner/operator/logical_order_by.h"
#include "planner/planner.h"

using namespace rag3db::binder;

namespace rag3db {
namespace planner {

void Planner::appendOrderBy(const expression_vector& expressions,
    const std::vector<bool>& isAscOrders, LogicalPlan& plan) {
    auto orderBy = make_shared<LogicalOrderBy>(expressions, isAscOrders, plan.getLastOperator());
    appendFlattens(orderBy->getGroupsPosToFlatten(), plan);
    orderBy->setChild(0, plan.getLastOperator());
    orderBy->computeFactorizedSchema();
    plan.setLastOperator(std::move(orderBy));
}

} // namespace planner
} // namespace rag3db
