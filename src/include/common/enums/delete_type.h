#pragma once

#include <cstdint>

namespace rag3db {
namespace common {

enum class DeleteNodeType : uint8_t {
    DELETE = 0,
    DETACH_DELETE = 1,
};

} // namespace common
} // namespace rag3db
