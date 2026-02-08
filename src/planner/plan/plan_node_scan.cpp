#include "planner/planner.h"

using namespace rag3db::binder;

namespace rag3db {
namespace planner {

LogicalPlan Planner::getNodePropertyScanPlan(const NodeExpression& node) {
    auto properties = getProperties(node);
    auto scanPlan = LogicalPlan();
    if (properties.empty()) {
        return scanPlan;
    }
    appendScanNodeTable(node.getInternalID(), node.getTableIDs(), properties, scanPlan);
    return scanPlan;
}

} // namespace planner
} // namespace rag3db
