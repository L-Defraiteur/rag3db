#pragma once

#include "function/function.h"

namespace rag3db {
namespace lucivy_fts_extension {

struct InternalCloseLucivyFunction {
    static constexpr const char* name = "_CLOSE_LUCIVY_INDEX";
    static function::function_set getFunctionSet();
};

struct CloseLucivyFunction {
    static constexpr const char* name = "CLOSE_LUCIVY_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace lucivy_fts_extension
} // namespace rag3db
