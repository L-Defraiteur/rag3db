#include "function/geo_frustum_from_camera.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"
#include "math/geometry.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// GEO_FRUSTUM_FROM_CAMERA(position [x,y,z], quat [x,y,z,w], fov_h, fov_v, near, far) → LIST(24)
static void execFunc(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    result.resetAuxiliaryBuffer();
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        double pos[3];
        math::Quat q;
        double fovH, fovV, nearDist, farDist;
        if (!extractListDouble(*parameters[0], *parameterSelVectors[0], i, pos, 3) ||
            !extractQuat(*parameters[1], *parameterSelVectors[1], i, q) ||
            !extractDouble(*parameters[2], *parameterSelVectors[2], i, fovH) ||
            !extractDouble(*parameters[3], *parameterSelVectors[3], i, fovV) ||
            !extractDouble(*parameters[4], *parameterSelVectors[4], i, nearDist) ||
            !extractDouble(*parameters[5], *parameterSelVectors[5], i, farDist)) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        double planes[24];
        math::frustumFromCamera(pos[0], pos[1], pos[2], q.x, q.y, q.z, q.w, fovH, fovV,
            nearDist, farDist, planes);
        writeDoubleListToResult(result, resPos, planes, 24);
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments,
        LogicalType::LIST(LogicalType::DOUBLE()));
}

function_set GeoFrustumFromCameraFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST, LogicalTypeID::LIST, LogicalTypeID::DOUBLE,
            LogicalTypeID::DOUBLE, LogicalTypeID::DOUBLE, LogicalTypeID::DOUBLE},
        LogicalTypeID::LIST, execFunc);
    func->bindFunc = bindFunc;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
