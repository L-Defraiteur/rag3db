#pragma once

#include "function/function.h"

namespace rag3db {
namespace sparse_vector_extension {

struct QuerySparseVectorFunction {
    static constexpr const char* name = "QUERY_SPARSE_VECTOR_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace sparse_vector_extension
} // namespace rag3db
