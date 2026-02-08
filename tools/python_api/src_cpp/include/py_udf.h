#pragma once

#include <string>

#include "common/types/types.h"
#include "function/function.h"
#include "pybind_include.h"

using rag3db::common::LogicalTypeID;
using rag3db::function::function_set;

namespace rag3db {
namespace main {
class ClientContext;
} // namespace main
} // namespace rag3db

class PyUDF {

public:
    static function_set toFunctionSet(const std::string& name, const py::function& udf,
        const py::list& paramTypes, const std::string& resultType, bool defaultNull,
        bool catchExceptions, rag3db::main::ClientContext* context);
};
