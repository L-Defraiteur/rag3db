#include "main/version.h"

#include "c_api/helpers.h"
#include "c_api/rag3db.h"

char* rag3db_get_version() {
    return convertToOwnedCString(rag3db::main::Version::getVersion());
}

uint64_t rag3db_get_storage_version() {
    return rag3db::main::Version::getStorageVersion();
}
