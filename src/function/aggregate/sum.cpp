#include "function/aggregate/sum.h"

namespace rag3db {
namespace function {

using namespace rag3db::common;

function_set AggregateSumFunction::getFunctionSet() {
    function_set result;
    for (auto typeID : LogicalTypeUtils::getNumericalLogicalTypeIDs()) {
        AggregateFunctionUtils::appendSumOrAvgFuncs<SumFunction>(name, typeID, result);
    }
    return result;
}

} // namespace function
} // namespace rag3db
