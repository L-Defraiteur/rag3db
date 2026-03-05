#pragma once

#include "function/function.h"

namespace rag3db {
namespace lucivy_fts_extension {

struct InternalCreateLucivyFunction {
    static constexpr const char* name = "_CREATE_LUCIVY_INDEX";
    static function::function_set getFunctionSet();
};

struct CreateLucivyFunction {
    static constexpr const char* name = "CREATE_LUCIVY_INDEX";
    static function::function_set getFunctionSet();
};

} // namespace lucivy_fts_extension
} // namespace rag3db
