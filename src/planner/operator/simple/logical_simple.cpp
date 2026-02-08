#include "planner/operator/simple/logical_simple.h"

namespace rag3db {
namespace planner {

void LogicalSimple::computeFlatSchema() {
    createEmptySchema();
}

void LogicalSimple::computeFactorizedSchema() {
    createEmptySchema();
}

} // namespace planner
} // namespace rag3db
