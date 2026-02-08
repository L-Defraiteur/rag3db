#pragma once

#include <cstdint>

namespace rag3db {
namespace function {

enum class GDSDensityState : uint8_t {
    SPARSE = 0,
    DENSE = 1,
};

}
} // namespace rag3db
