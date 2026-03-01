#include "function/geo_quat_multiply.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// GEO_QUAT_MULTIPLY(q1 [x,y,z,w], q2 [x,y,z,w]) → LIST(DOUBLE) [x,y,z,w]
static void execFunc(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    result.resetAuxiliaryBuffer();
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        math::Quat q1, q2;
        if (!extractQuat(*parameters[0], *parameterSelVectors[0], i, q1) ||
            !extractQuat(*parameters[1], *parameterSelVectors[1], i, q2)) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        writeQuatToList(result, resPos, q1 * q2);
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments,
        LogicalType::LIST(LogicalType::DOUBLE()));
}

function_set GeoQuatMultiplyFunction::getFunctionSet() {
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
