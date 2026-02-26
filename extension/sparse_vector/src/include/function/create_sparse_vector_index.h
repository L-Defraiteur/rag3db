#pragma once

#include "function/function.h"

namespace rag3db {
namespace sparse_vector_extension {

struct InternalCreateSparseVectorFunction {
    static constexpr const char* name = "_CREATE_SPARSE_VECTOR_INDEX";
    static function::function_set getFunctionSet();
};

struct CreateSparseVectorFunction {
    static constexpr const char* name = "CREATE_SPARSE_VECTOR_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace sparse_vector_extension
} // namespace rag3db
