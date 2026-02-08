#pragma once

#include <cstdint>

namespace rag3db {
namespace common {

enum class SubqueryType : uint8_t {
    COUNT = 1,
    EXISTS = 2,
};

}
} // namespace rag3db
