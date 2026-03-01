#pragma once
#include "function/function.h"

namespace rag3db {
namespace geo_extension {

struct GeoQuatToMatrixFunction {
    static constexpr const char* name = "GEO_QUAT_TO_MATRIX";
    static function::function_set getFunctionSet();
};

} // namespace geo_extension
} // namespace rag3db
