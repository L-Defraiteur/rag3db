#include "function/geo_quat_to_matrix.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// GEO_QUAT_TO_MATRIX(quat [x,y,z,w]) → LIST(DOUBLE) 9 elements (column-major, Three.js compat)
static void execFunc(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    result.resetAuxiliaryBuffer();
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        math::Quat q;
        if (!extractQuat(*parameters[0], *parameterSelVectors[0], i, q)) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        double m[9];
        q.toMatrix3(m);
        math::Mat3 mat{};
        for (int k = 0; k < 9; k++) mat.m[k] = m[k];
        writeMat3ToList(result, resPos, mat);
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments,
        LogicalType::LIST(LogicalType::DOUBLE()));
}

function_set GeoQuatToMatrixFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST}, LogicalTypeID::LIST, execFunc);
    func->bindFunc = bindFunc;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
