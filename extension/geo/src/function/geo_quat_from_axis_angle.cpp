#include "function/geo_quat_from_axis_angle.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// GEO_QUAT_FROM_AXIS_ANGLE(axis [x,y,z], angle_rad DOUBLE) → LIST(DOUBLE) [x,y,z,w]
static void execFunc(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    result.resetAuxiliaryBuffer();
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        double axis[3];
        double angle;
        if (!extractListDouble(*parameters[0], *parameterSelVectors[0], i, axis, 3) ||
            !extractDouble(*parameters[1], *parameterSelVectors[1], i, angle)) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        auto q = math::Quat::fromAxisAngle(axis[0], axis[1], axis[2], angle);
        writeQuatToList(result, resPos, q);
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments,
        LogicalType::LIST(LogicalType::DOUBLE()));
}

function_set GeoQuatFromAxisAngleFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST, LogicalTypeID::DOUBLE},
        LogicalTypeID::LIST, execFunc);
    func->bindFunc = bindFunc;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
