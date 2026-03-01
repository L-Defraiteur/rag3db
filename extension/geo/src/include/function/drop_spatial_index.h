#pragma once

#include "function/function.h"

namespace rag3db {
namespace geo_extension {

struct InternalDropSpatialIndexFunction {
    static constexpr const char* name = "_DROP_SPATIAL_INDEX";
    static function::function_set getFunctionSet();
};

struct DropSpatialIndexFunction {
    static constexpr const char* name = "DROP_SPATIAL_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace geo_extension
} // namespace rag3db
