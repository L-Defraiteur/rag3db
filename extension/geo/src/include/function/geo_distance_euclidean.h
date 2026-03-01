#pragma once
#include "function/function.h"

namespace rag3db {
namespace geo_extension {

struct GeoDistanceEuclideanFunction {
    static constexpr const char* name = "GEO_DISTANCE_EUCLIDEAN";
    static function::function_set getFunctionSet();
};

} // namespace geo_extension
} // namespace rag3db
