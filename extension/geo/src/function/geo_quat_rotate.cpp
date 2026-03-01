#include "function/geo_quat_rotate.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// GEO_QUAT_ROTATE(quat [x,y,z,w], point [x,y,z]) → LIST(DOUBLE) [x,y,z]
static void execFunc(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    result.resetAuxiliaryBuffer();
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        math::Quat q;
        double point[3];
        if (!extractQuat(*parameters[0], *parameterSelVectors[0], i, q) ||
            !extractListDouble(*parameters[1], *parameterSelVectors[1], i, point, 3)) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        double ox, oy, oz;
        q.rotate(point[0], point[1], point[2], ox, oy, oz);
        writePoint3ToList(result, resPos, ox, oy, oz);
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments,
        LogicalType::LIST(LogicalType::DOUBLE()));
}

function_set GeoQuatRotateFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST, LogicalTypeID::LIST},
        LogicalTypeID::LIST, execFunc);
    func->bindFunc = bindFunc;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
