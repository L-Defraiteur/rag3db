#pragma once

#include "common/api.h"
#include "exception.h"

namespace rag3db {
namespace common {

class RAG3DB_API ConnectionException : public Exception {
public:
    explicit ConnectionException(const std::string& msg)
        : Exception("Connection exception: " + msg){};
};

} // namespace common
} // namespace rag3db
