#pragma once

#include "function/function.h"

namespace rag3db {
namespace lucivy_fts_extension {

struct InternalFlushLucivyFunction {
    static constexpr const char* name = "_FLUSH_LUCIVY_INDEX";
    static function::function_set getFunctionSet();
};

struct FlushLucivyFunction {
    static constexpr const char* name = "FLUSH_LUCIVY_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace lucivy_fts_extension
} // namespace rag3db
