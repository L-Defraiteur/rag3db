#pragma once

#include <atomic>
#include <cstdint>
#include <string>

#include "common/api.h"
#include "common/cast.h"
#include "function/table/table_function.h"

namespace rag3db {
namespace common {

class FileSystem;

struct RAG3DB_API FileInfo {
    FileInfo(std::string path, FileSystem* fileSystem)
        : path{std::move(path)}, fileSystem{fileSystem} {}

    virtual ~FileInfo() = default;

    uint64_t getFileSize() const;

    void readFromFile(void* buffer, uint64_t numBytes, uint64_t position);

    int64_t readFile(void* buf, size_t nbyte);

    void writeFile(const uint8_t* buffer, uint64_t numBytes, uint64_t offset);
    // Les octets passés à writeFile depuis le début du processus, tous fichiers confondus :
    // la mesure de ce qu'un point de reprise écrit (différence avant/après, dans
    // [checkpoint-profile]). Compté avant l'appel au système de fichiers, donc aussi ce qu'une
    // écriture refusée aurait écrit.
    static uint64_t bytesWritten() { return totalBytesWritten.load(std::memory_order_relaxed); }

    void syncFile() const;

    int64_t seek(uint64_t offset, int whence);

    void reset();

    void truncate(uint64_t size);

    bool canPerformSeek() const;

    virtual function::TableFunction getHandleFunction() const { KU_UNREACHABLE; }

    template<class TARGET>
    TARGET* ptrCast() {
        return common::ku_dynamic_cast<TARGET*>(this);
    }

    template<class TARGET>
    const TARGET* constPtrCast() const {
        return common::ku_dynamic_cast<const TARGET*>(this);
    }

    template<class TARGET>
    const TARGET& constCast() const {
        return common::ku_dynamic_cast<const TARGET&>(*this);
    }

    template<class TARGET>
    TARGET& cast() {
        return common::ku_dynamic_cast<TARGET&>(*this);
    }

    const std::string path;

    FileSystem* fileSystem;

private:
    static std::atomic<uint64_t> totalBytesWritten;
};

} // namespace common
} // namespace rag3db
