#pragma once
#include "function/function.h"

namespace rag3db {
namespace geo_extension {

struct GeoWithinBboxNdFunction {
    static constexpr const char* name = "GEO_WITHIN_BBOX_ND";
    static function::function_set getFunctionSet();
};

} // namespace geo_extension
} // namespace rag3db
