#include "planner/operator/logical_create_macro.h"

namespace rag3db {
namespace planner {

std::string LogicalCreateMacroPrintInfo::toString() const {
    std::string result = "Macro: ";
    result += macroName;
    return result;
}

void LogicalCreateMacro::computeFlatSchema() {
    createEmptySchema();
}

void LogicalCreateMacro::computeFactorizedSchema() {
    createEmptySchema();
}

} // namespace planner
} // namespace rag3db
