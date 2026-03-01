#include "function/geo_within_convex.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"
#include "math/geometry.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// GEO_WITHIN_CONVEX(point [x,y,z], planes LIST(DOUBLE) flat N*4) → BOOL
// Same as frustum but for any convex polyhedron (N planes, not limited to 6).
static void execFunc(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        double point[3];
        std::vector<double> planes;
        uint32_t planesSize;
        if (!extractListDouble(*parameters[0], *parameterSelVectors[0], i, point, 3) ||
            !extractListDoubleVar(*parameters[1], *parameterSelVectors[1], i, planes,
                planesSize) ||
            planesSize % 4 != 0 || planesSize == 0) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        uint32_t numPlanes = planesSize / 4;
        result.setValue<bool>(resPos,
            math::insideConvex(point[0], point[1], point[2], planes.data(), numPlanes));
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments, LogicalType::BOOL());
}

function_set GeoWithinConvexFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST, LogicalTypeID::LIST},
        LogicalTypeID::BOOL, execFunc);
    func->bindFunc = bindFunc;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
