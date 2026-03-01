#include "function/geo_within_bbox_nd.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// GEO_WITHIN_BBOX_ND(point LIST(DOUBLE), min_corner LIST(DOUBLE), max_corner LIST(DOUBLE)) → BOOL
static void execFunc(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];

        std::vector<double> point, minC, maxC;
        uint32_t sp, smin, smax;
        if (!extractListDoubleVar(*parameters[0], *parameterSelVectors[0], i, point, sp) ||
            !extractListDoubleVar(*parameters[1], *parameterSelVectors[1], i, minC, smin) ||
            !extractListDoubleVar(*parameters[2], *parameterSelVectors[2], i, maxC, smax) ||
            sp != smin || sp != smax || sp == 0) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        bool inside = true;
        for (uint32_t d = 0; d < sp; d++) {
            if (point[d] < minC[d] || point[d] > maxC[d]) {
                inside = false;
                break;
            }
        }
        result.setValue<bool>(resPos, inside);
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments, LogicalType::BOOL());
}

function_set GeoWithinBboxNdFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST, LogicalTypeID::LIST, LogicalTypeID::LIST},
        LogicalTypeID::BOOL, execFunc);
    func->bindFunc = bindFunc;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
