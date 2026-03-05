#pragma once

#include "function/function.h"

namespace rag3db {
namespace lucivy_fts_extension {

struct InternalDropLucivyFunction {
    static constexpr const char* name = "_DROP_LUCIVY_INDEX";
    static function::function_set getFunctionSet();
};

struct DropLucivyFunction {
    static constexpr const char* name = "DROP_LUCIVY_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace lucivy_fts_extension
} // namespace rag3db
