#pragma once

#include "function/function.h"

namespace rag3db {
namespace sparse_vector_extension {

struct SparseSearchFunction {
    static constexpr const char* name = "SPARSE_SEARCH";
    static function::function_set getFunctionSet();
};

struct SparseScoreFunction {
    static constexpr const char* name = "SPARSE_SCORE";
    static function::function_set getFunctionSet();
};

} // namespace sparse_vector_extension
} // namespace rag3db
