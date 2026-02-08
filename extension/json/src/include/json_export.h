#pragma once

#include "function/function.h"
#include "function/table/bind_data.h"

namespace rag3db {
namespace json_extension {

struct JsonExportFunction {
    static constexpr const char* name = "COPY_JSON";

    static function::function_set getFunctionSet();
};

} // namespace json_extension
} // namespace rag3db
