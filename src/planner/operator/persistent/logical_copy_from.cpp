#include "planner/operator/persistent/logical_copy_from.h"

using namespace rag3db::common;

namespace rag3db {
namespace planner {

void LogicalCopyFrom::computeFactorizedSchema() {
    copyChildSchema(0);
}

void LogicalCopyFrom::computeFlatSchema() {
    copyChildSchema(0);
}

} // namespace planner
} // namespace rag3db
