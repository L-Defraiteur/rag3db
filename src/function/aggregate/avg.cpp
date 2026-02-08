#include "function/aggregate/avg.h"

namespace rag3db {
namespace function {

using namespace rag3db::common;

function_set AggregateAvgFunction::getFunctionSet() {
    function_set result;
    for (auto typeID : LogicalTypeUtils::getNumericalLogicalTypeIDs()) {
        AggregateFunctionUtils::appendSumOrAvgFuncs<AvgFunction>(name, typeID, result);
    }
    return result;
}

} // namespace function
} // namespace rag3db
