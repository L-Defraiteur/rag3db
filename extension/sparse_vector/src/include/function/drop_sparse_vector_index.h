#pragma once

#include "function/function.h"

namespace rag3db {
namespace sparse_vector_extension {

struct InternalDropSparseVectorFunction {
    static constexpr const char* name = "_DROP_SPARSE_VECTOR_INDEX";
    static function::function_set getFunctionSet();
};

struct DropSparseVectorFunction {
    static constexpr const char* name = "DROP_SPARSE_VECTOR_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace sparse_vector_extension
} // namespace rag3db
