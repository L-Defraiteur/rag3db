#pragma once

#include "function/function.h"

namespace rag3db {
namespace lucivy_fts_extension {

struct SearchFunction {
    static constexpr const char* name = "SEARCH";
    static function::function_set getFunctionSet();
};

struct SearchScoreFunction {
    static constexpr const char* name = "SEARCH_SCORE";
    static function::function_set getFunctionSet();
};

struct SearchHighlightsFunction {
    static constexpr const char* name = "SEARCH_HIGHLIGHTS";
    static function::function_set getFunctionSet();
};

} // namespace lucivy_fts_extension
} // namespace rag3db
