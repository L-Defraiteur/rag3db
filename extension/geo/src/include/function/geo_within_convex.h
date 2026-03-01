#pragma once
#include "function/function.h"

namespace rag3db {
namespace geo_extension {

struct GeoWithinConvexFunction {
    static constexpr const char* name = "GEO_WITHIN_CONVEX";
    static function::function_set getFunctionSet();
};

} // namespace geo_extension
} // namespace rag3db
