#pragma once
#include "function/function.h"

namespace rag3db {
namespace geo_extension {

struct GeoFrustumFromCameraFunction {
    static constexpr const char* name = "GEO_FRUSTUM_FROM_CAMERA";
    static function::function_set getFunctionSet();
};

} // namespace geo_extension
} // namespace rag3db
