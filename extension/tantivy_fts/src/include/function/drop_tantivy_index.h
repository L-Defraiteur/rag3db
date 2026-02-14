#pragma once

#include "function/function.h"

namespace rag3db {
namespace tantivy_fts_extension {

struct InternalDropTantivyFunction {
    static constexpr const char* name = "_DROP_TANTIVY_INDEX";
    static function::function_set getFunctionSet();
};

struct DropTantivyFunction {
    static constexpr const char* name = "DROP_TANTIVY_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace tantivy_fts_extension
} // namespace rag3db
