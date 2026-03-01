#pragma once
#include "function/function.h"

namespace rag3db {
namespace geo_extension {

struct GeoQuatRotateFunction {
    static constexpr const char* name = "GEO_QUAT_ROTATE";
    static function::function_set getFunctionSet();
};

} // namespace geo_extension
} // namespace rag3db
