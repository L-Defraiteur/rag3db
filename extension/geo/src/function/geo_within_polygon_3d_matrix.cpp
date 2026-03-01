#include "function/geo_within_polygon_3d_matrix.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"
#include "math/geometry.h"

#include <cmath>

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

static bool withinPolygon3dMatrixCore(const double point[3], const std::vector<double>& polyXs,
    const std::vector<double>& polyYs, const double position[3], const math::Mat3& mat,
    bool hasThickness, double thickness) {
    uint32_t n = std::min(static_cast<uint32_t>(polyXs.size()),
        static_cast<uint32_t>(polyYs.size()));
    if (n < 3) return false;

    double dx = point[0] - position[0];
    double dy = point[1] - position[1];
    double dz = point[2] - position[2];
    // For rotation matrices, inverse = transpose.
    auto matInv = mat.transpose();
    double lx, ly, lz;
    matInv.rotate(dx, dy, dz, lx, ly, lz);

    if (hasThickness && std::fabs(lz) > thickness * 0.5) {
        return false;
    }

    return math::pointInPolygon(lx, ly, polyXs.data(), polyYs.data(), n);
}

// 5-param (no thickness).
static void execFunc5(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        double point[3], position[3];
        math::Mat3 mat;
        std::vector<double> polyXs, polyYs;
        uint32_t sxs, sys;
        if (!extractListDouble(*parameters[0], *parameterSelVectors[0], i, point, 3) ||
            !extractListDoubleVar(*parameters[1], *parameterSelVectors[1], i, polyXs, sxs) ||
            !extractListDoubleVar(*parameters[2], *parameterSelVectors[2], i, polyYs, sys) ||
            !extractListDouble(*parameters[3], *parameterSelVectors[3], i, position, 3) ||
            !extractMat3(*parameters[4], *parameterSelVectors[4], i, mat)) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        result.setValue<bool>(resPos,
            withinPolygon3dMatrixCore(point, polyXs, polyYs, position, mat, false, 0));
    }
}

// 6-param (with thickness).
static void execFunc6(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        double point[3], position[3], thickness;
        math::Mat3 mat;
        std::vector<double> polyXs, polyYs;
        uint32_t sxs, sys;
        if (!extractListDouble(*parameters[0], *parameterSelVectors[0], i, point, 3) ||
            !extractListDoubleVar(*parameters[1], *parameterSelVectors[1], i, polyXs, sxs) ||
            !extractListDoubleVar(*parameters[2], *parameterSelVectors[2], i, polyYs, sys) ||
            !extractListDouble(*parameters[3], *parameterSelVectors[3], i, position, 3) ||
            !extractMat3(*parameters[4], *parameterSelVectors[4], i, mat) ||
            !extractDouble(*parameters[5], *parameterSelVectors[5], i, thickness)) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        result.setValue<bool>(resPos,
            withinPolygon3dMatrixCore(point, polyXs, polyYs, position, mat, true, thickness));
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments, LogicalType::BOOL());
}

function_set GeoWithinPolygon3dMatrixFunction::getFunctionSet() {
    function_set result;
    auto func5 = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST, LogicalTypeID::LIST, LogicalTypeID::LIST,
            LogicalTypeID::LIST, LogicalTypeID::LIST},
        LogicalTypeID::BOOL, execFunc5);
    func5->bindFunc = bindFunc;
    result.push_back(std::move(func5));
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
