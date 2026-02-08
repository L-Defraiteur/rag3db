#include "processor/operator/aggregate/base_aggregate_scan.h"

using namespace rag3db::common;
using namespace rag3db::function;

namespace rag3db {
namespace processor {

void BaseAggregateScan::initLocalStateInternal(ResultSet* resultSet,
    ExecutionContext* /*context*/) {
    for (auto& dataPos : scanInfo.aggregatesPos) {
        auto valueVector = resultSet->getValueVector(dataPos);
        aggregateVectors.push_back(valueVector);
    }
}

} // namespace processor
} // namespace rag3db
