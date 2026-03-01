#pragma once
#include "function/function.h"

namespace rag3db {
namespace geo_extension {

struct GeoWithinSphereFunction {
    static constexpr const char* name = "GEO_WITHIN_SPHERE";
    static function::function_set getFunctionSet();
};

} // namespace geo_extension
} // namespace rag3db
