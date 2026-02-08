#pragma once

#include <cstdint>
#include <string>

namespace rag3db {
namespace common {

enum class DropType : uint8_t {
    TABLE = 0,
    SEQUENCE = 1,
    MACRO = 2,
};

struct DropTypeUtils {
    static std::string toString(DropType type);
};

} // namespace common
} // namespace rag3db
