#pragma once
#include <cstdint>

#include "common/api.h"
namespace rag3db {
namespace main {

struct Version {
public:
    /**
     * @brief Get the version of the Rag3db library.
     * @return const char* The version of the Rag3db library.
     */
    RAG3DB_API static const char* getVersion();

    /**
     * @brief Get the storage version of the Rag3db library.
     * @return uint64_t The storage version of the Rag3db library.
     */
    RAG3DB_API static uint64_t getStorageVersion();
};
} // namespace main
} // namespace rag3db
