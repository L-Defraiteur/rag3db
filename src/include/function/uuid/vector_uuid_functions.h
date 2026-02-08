#pragma once

#include "function/function.h"

namespace rag3db {
namespace function {

struct GenRandomUUIDFunction {
    static constexpr const char* name = "GEN_RANDOM_UUID";

    static function_set getFunctionSet();
};

} // namespace function
} // namespace rag3db
