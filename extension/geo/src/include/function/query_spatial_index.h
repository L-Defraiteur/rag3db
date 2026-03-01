#pragma once

#include "function/function.h"

namespace rag3db {
namespace geo_extension {

struct QuerySpatialIndexFunction {
    static constexpr const char* name = "QUERY_SPATIAL_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace geo_extension
} // namespace rag3db
