#include "storage/storage_version_info.h"

#include "common/build_id.h"

namespace rag3db {
namespace storage {

storage_version_t StorageVersionInfo::getStorageVersion() {
    return STORAGE_FORMAT_VERSION;
}

std::string StorageVersionInfo::getBuildID() {
    return RAG3DB_BUILD_ID;
}

} // namespace storage
} // namespace rag3db
