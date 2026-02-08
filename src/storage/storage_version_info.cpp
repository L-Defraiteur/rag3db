#include "storage/storage_version_info.h"

namespace rag3db {
namespace storage {

storage_version_t StorageVersionInfo::getStorageVersion() {
    auto storageVersionInfo = getStorageVersionInfo();
    if (!storageVersionInfo.contains(RAG3DB_CMAKE_VERSION)) {
        // If the current RAG3DB_CMAKE_VERSION is not in the map,
        // then we must run the newest version of rag3db
        // LCOV_EXCL_START
        storage_version_t maxVersion = 0;
        for (auto& [_, versionNumber] : storageVersionInfo) {
            maxVersion = std::max(maxVersion, versionNumber);
        }
        return maxVersion;
        // LCOV_EXCL_STOP
    }
    return storageVersionInfo.at(RAG3DB_CMAKE_VERSION);
}

} // namespace storage
} // namespace rag3db
