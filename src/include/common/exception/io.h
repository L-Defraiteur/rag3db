#pragma once

#include "exception.h"

namespace rag3db {
namespace common {

class RAG3DB_API IOException : public Exception {
public:
    explicit IOException(const std::string& msg) : Exception("IO exception: " + msg) {}
};

} // namespace common
} // namespace rag3db
