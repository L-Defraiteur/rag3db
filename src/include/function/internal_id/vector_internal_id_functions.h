#pragma once

#include "function/function.h"

namespace rag3db {
namespace function {

struct InternalIDCreationFunction {
    static constexpr const char* name = "internal_id";

    static function_set getFunctionSet();
};

} // namespace function
} // namespace rag3db
