#pragma once

#include "function/function.h"

namespace rag3db {
namespace tantivy_fts_extension {

struct QueryTantivyFunction {
    static constexpr const char* name = "QUERY_TANTIVY_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace tantivy_fts_extension
} // namespace rag3db
