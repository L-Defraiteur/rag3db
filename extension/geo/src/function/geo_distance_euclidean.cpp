#include "function/geo_distance_euclidean.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"
#include "math/geometry.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// GEO_DISTANCE_EUCLIDEAN(point1 LIST(DOUBLE), point2 LIST(DOUBLE)) → DOUBLE
static void execFunc(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];

        std::vector<double> p1, p2;
        uint32_t s1, s2;
        if (!extractListDoubleVar(*parameters[0], *parameterSelVectors[0], i, p1, s1) ||
            !extractListDoubleVar(*parameters[1], *parameterSelVectors[1], i, p2, s2) ||
            s1 != s2 || s1 == 0) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        result.setValue<double>(resPos, math::euclideanDist(p1.data(), p2.data(), s1));
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments, LogicalType::DOUBLE());
}

function_set GeoDistanceEuclideanFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST, LogicalTypeID::LIST},
        LogicalTypeID::DOUBLE, execFunc);
    func->bindFunc = bindFunc;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
