#pragma once

#include "planner/operator/operator_print_info.h"

namespace rag3db {
namespace processor {

struct ExtensionPrintInfo : OPPrintInfo {
    std::string extensionName;

    explicit ExtensionPrintInfo(std::string extensionName)
        : extensionName{std::move(extensionName)} {}
};

} // namespace processor
} // namespace rag3db
