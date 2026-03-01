#include "function/geo_within_obb.h"

#include "common/vector/value_vector.h"
#include "function/geo_list_helpers.h"
#include "function/scalar_function.h"

#include <cmath>

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// GEO_WITHIN_OBB(point [x,y,z], center [x,y,z], half_extents [x,y,z], quat [x,y,z,w]) → BOOL
static void execFunc(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        double point[3], center[3], halfExt[3];
        math::Quat q;
        if (!extractListDouble(*parameters[0], *parameterSelVectors[0], i, point, 3) ||
            !extractListDouble(*parameters[1], *parameterSelVectors[1], i, center, 3) ||
            !extractListDouble(*parameters[2], *parameterSelVectors[2], i, halfExt, 3) ||
            !extractQuat(*parameters[3], *parameterSelVectors[3], i, q)) {
            result.setNull(resPos, true);
            continue;
        }
        result.setNull(resPos, false);
        // Transform point to OBB local frame.
        double dx = point[0] - center[0];
        double dy = point[1] - center[1];
        double dz = point[2] - center[2];
        double lx, ly, lz;
        q.inverse().rotate(dx, dy, dz, lx, ly, lz);
        bool inside = std::fabs(lx) <= halfExt[0] && std::fabs(ly) <= halfExt[1] &&
                      std::fabs(lz) <= halfExt[2];
        result.setValue<bool>(resPos, inside);
    }
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    return FunctionBindData::getSimpleBindData(input.arguments, LogicalType::BOOL());
}

function_set GeoWithinObbFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST, LogicalTypeID::LIST, LogicalTypeID::LIST,
            LogicalTypeID::LIST},
        LogicalTypeID::BOOL, execFunc);
    func->bindFunc = bindFunc;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
