#pragma once

namespace rag3db {
namespace common {
struct DatabaseLifeCycleManager {
    bool isDatabaseClosed = false;
    void checkDatabaseClosedOrThrow() const;
};
} // namespace common
} // namespace rag3db
