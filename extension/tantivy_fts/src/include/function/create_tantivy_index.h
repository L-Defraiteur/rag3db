#pragma once

#include "function/function.h"

namespace rag3db {
namespace tantivy_fts_extension {

struct InternalCreateTantivyFunction {
    static constexpr const char* name = "_CREATE_TANTIVY_INDEX";
    static function::function_set getFunctionSet();
};

struct CreateTantivyFunction {
    static constexpr const char* name = "CREATE_TANTIVY_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace tantivy_fts_extension
} // namespace rag3db
