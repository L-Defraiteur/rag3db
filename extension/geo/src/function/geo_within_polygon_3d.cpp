#include "function/geo_within_polygon_3d.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"
#include "math/geometry.h"

#include <cmath>

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// Core logic: transform point to polygon's local 2D frame, then ray cast.
static bool withinPolygon3dCore(const double point[3], const std::vector<double>& polyXs,
    const std::vector<double>& polyYs, const double position[3], const math::Quat& q,
    bool hasThickness, double thickness) {
    uint32_t n = std::min(static_cast<uint32_t>(polyXs.size()),
        static_cast<uint32_t>(polyYs.size()));
    if (n < 3) return false;

    // Transform point to polygon's local frame.
    double dx = point[0] - position[0];
    double dy = point[1] - position[1];
    double dz = point[2] - position[2];
    double lx, ly, lz;
    q.inverse().rotate(dx, dy, dz, lx, ly, lz);

    // Check thickness (distance to polygon plane along local Z).
    if (hasThickness && std::fabs(lz) > thickness * 0.5) {
        return false;
    }

    // Ray cast in 2D (lx, ly) against the polygon.
    return math::pointInPolygon(lx, ly, polyXs.data(), polyYs.data(), n);
}

// 5-param: no thickness check (projection only)
// GEO_WITHIN_POLYGON_3D(point [x,y,z], polygon_xs LIST, polygon_ys LIST, position [x,y,z], quat [x,y,z,w])
static void execFunc5(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        double point[3], position[3];
        math::Quat q;
        std::vector<double> polyXs, polyYs;
        uint32_t sxs, sys;
        if (!extractListDouble(*parameters[0], *parameterSelVectors[0], i, point, 3) ||
            !extractListDoubleVar(*parameters[1], *parameterSelVectors[1], i, polyXs, sxs) ||
            !extractListDoubleVar(*parameters[2], *parameterSelVectors[2], i, polyYs, sys) ||
            !extractListDouble(*parameters[3], *parameterSelVectors[3], i, position, 3) ||
            !extractQuat(*parameters[4], *parameterSelVectors[4], i, q)) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        result.setValue<bool>(resPos,
            withinPolygon3dCore(point, polyXs, polyYs, position, q, false, 0));
    }
}

// 6-param: with thickness DOUBLE
static void execFunc6(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        double point[3], position[3], thickness;
        math::Quat q;
        std::vector<double> polyXs, polyYs;
        uint32_t sxs, sys;
        if (!extractListDouble(*parameters[0], *parameterSelVectors[0], i, point, 3) ||
            !extractListDoubleVar(*parameters[1], *parameterSelVectors[1], i, polyXs, sxs) ||
            !extractListDoubleVar(*parameters[2], *parameterSelVectors[2], i, polyYs, sys) ||
            !extractListDouble(*parameters[3], *parameterSelVectors[3], i, position, 3) ||
            !extractQuat(*parameters[4], *parameterSelVectors[4], i, q) ||
            !extractDouble(*parameters[5], *parameterSelVectors[5], i, thickness)) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        result.setValue<bool>(resPos,
            withinPolygon3dCore(point, polyXs, polyYs, position, q, true, thickness));
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments, LogicalType::BOOL());
}

function_set GeoWithinPolygon3dFunction::getFunctionSet() {
    function_set result;
    // 5-param (no thickness).
    auto func5 = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST, LogicalTypeID::LIST, LogicalTypeID::LIST,
            LogicalTypeID::LIST, LogicalTypeID::LIST},
        LogicalTypeID::BOOL, execFunc5);
    func5->bindFunc = bindFunc;
    result.push_back(std::move(func5));
    // 6-param (with thickness).
    auto func6 = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST, LogicalTypeID::LIST, LogicalTypeID::LIST,
            LogicalTypeID::LIST, LogicalTypeID::LIST, LogicalTypeID::DOUBLE},
        LogicalTypeID::BOOL, execFunc6);
    func6->bindFunc = bindFunc;
    result.push_back(std::move(func6));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
