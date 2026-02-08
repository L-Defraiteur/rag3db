#pragma once

#include "planner/operator/logical_plan.h"

namespace rag3db {
namespace main {
class ClientContext;
}

namespace planner {
class CardinalityEstimator;
}

namespace optimizer {

class Optimizer {
public:
    static void optimize(planner::LogicalPlan* plan, main::ClientContext* context,
        const planner::CardinalityEstimator& cardinalityEstimator);
};

} // namespace optimizer
} // namespace rag3db
