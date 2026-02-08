#pragma once

#include "function/function.h"

namespace rag3db {
namespace fts_extension {

struct TokenizeFunction {
    static constexpr const char* name = "TOKENIZE";

    static function::function_set getFunctionSet();
};

} // namespace fts_extension
} // namespace rag3db
