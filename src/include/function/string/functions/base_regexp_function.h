#pragma once

#include <regex>

#include "common/vector/value_vector.h"

namespace rag3db {
namespace function {

struct BaseRegexpOperation {
    static inline std::string parseCypherPattern(const std::string& pattern) {
        // Cypher parses escape characters with 2 backslash eg. for expressing '.' requires '\\.'
        // Since Regular Expression requires only 1 backslash '\.' we need to replace double slash
        // with single
        return std::regex_replace(pattern, std::regex(R"(\\\\)"), "\\");
    }

    static inline void copyToRag3dbString(const std::string& value, common::ku_string_t& kuString,
        common::ValueVector& valueVector) {
        common::StringVector::addString(&valueVector, kuString, value.data(), value.length());
    }
};

} // namespace function
} // namespace rag3db
