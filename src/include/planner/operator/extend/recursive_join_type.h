#pragma once

#include <cstdint>

namespace rag3db {
namespace planner {

enum class RecursiveJoinType : uint8_t {
    TRACK_NONE = 0,
    TRACK_PATH = 1,
};

} // namespace planner
} // namespace rag3db
