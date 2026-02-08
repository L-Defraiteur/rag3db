#pragma once

#include "exception.h"

namespace rag3db {
namespace common {

class RAG3DB_API ExtensionException : public Exception {
public:
    explicit ExtensionException(const std::string& msg)
        : Exception("Extension exception: " + msg) {}
};

} // namespace common
} // namespace rag3db
