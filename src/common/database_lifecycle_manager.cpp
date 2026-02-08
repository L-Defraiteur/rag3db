#include "common/database_lifecycle_manager.h"

#include "common/exception/runtime.h"

namespace rag3db {
namespace common {
void DatabaseLifeCycleManager::checkDatabaseClosedOrThrow() const {
    if (isDatabaseClosed) {
        throw RuntimeException(
            "The current operation is not allowed because the parent database is closed.");
    }
}
} // namespace common
} // namespace rag3db
