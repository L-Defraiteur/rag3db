#pragma once

#include "function/function.h"

namespace rag3db {
namespace geo_extension {

struct InternalCreateSpatialIndexFunction {
    static constexpr const char* name = "_CREATE_SPATIAL_INDEX";
    static function::function_set getFunctionSet();
};

struct CreateSpatialIndexFunction {
    static constexpr const char* name = "CREATE_SPATIAL_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace geo_extension
} // namespace rag3db
