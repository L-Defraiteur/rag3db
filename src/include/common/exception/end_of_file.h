#pragma once

#include "common/api.h"
#include "runtime.h"

namespace rag3db {
namespace common {

// A read asked for bytes past the end of a file. For a write-ahead log this is
// a torn end (the process stopped while appending), not a corruption in the
// middle of the journal: the replayer tells the two apart by this type.
class RAG3DB_API EndOfFileException : public RuntimeException {
public:
    explicit EndOfFileException(const std::string& msg) : RuntimeException(msg) {}
};

} // namespace common
} // namespace rag3db
