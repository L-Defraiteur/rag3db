#include "function/geo_within_sphere.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"
#include "math/geometry.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// GEO_WITHIN_SPHERE(point LIST(DOUBLE), center LIST(DOUBLE), radius DOUBLE) → BOOL
static void execFunc(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];

        std::vector<double> point, center;
        uint32_t sp, sc;
        double radius;
        if (!extractListDoubleVar(*parameters[0], *parameterSelVectors[0], i, point, sp) ||
            !extractListDoubleVar(*parameters[1], *parameterSelVectors[1], i, center, sc) ||
            !extractDouble(*parameters[2], *parameterSelVectors[2], i, radius) ||
            sp != sc || sp == 0) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        double dist = math::euclideanDist(point.data(), center.data(), sp);
        result.setValue<bool>(resPos, dist <= radius);
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments, LogicalType::BOOL());
}

function_set GeoWithinSphereFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST, LogicalTypeID::LIST, LogicalTypeID::DOUBLE},
        LogicalTypeID::BOOL, execFunc);
    func->bindFunc = bindFunc;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
