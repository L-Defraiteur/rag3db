#pragma once

#include "function/function.h"

namespace rag3db {
namespace vector_extension {

struct VectorSearchFunction {
    static constexpr const char* name = "VECTOR_SEARCH";
    static function::function_set getFunctionSet();
};

struct VectorDistanceFunction {
    static constexpr const char* name = "VECTOR_DISTANCE";
    static function::function_set getFunctionSet();
};

} // namespace vector_extension
} // namespace rag3db
