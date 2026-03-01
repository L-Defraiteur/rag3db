#include "function/geo_matrix_multiply.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// GEO_MATRIX_MULTIPLY(m1 9 col-major, m2 9 col-major) → LIST(DOUBLE) 9 col-major
static void execFunc(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    result.resetAuxiliaryBuffer();
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        math::Mat3 m1, m2;
        if (!extractMat3(*parameters[0], *parameterSelVectors[0], i, m1) ||
            !extractMat3(*parameters[1], *parameterSelVectors[1], i, m2)) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        writeMat3ToList(result, resPos, m1 * m2);
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments,
        LogicalType::LIST(LogicalType::DOUBLE()));
}

function_set GeoMatrixMultiplyFunction::getFunctionSet() {
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
