#include "main/version.h"

#include "storage/storage_version_info.h"

namespace rag3db {
namespace main {
const char* Version::getVersion() {
    return RAG3DB_CMAKE_VERSION;
}

uint64_t Version::getStorageVersion() {
    return storage::StorageVersionInfo::getStorageVersion();
}
} // namespace main
} // namespace rag3db
