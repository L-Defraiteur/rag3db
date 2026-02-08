#pragma once

#include "common/api.h"
#include "exception.h"

namespace rag3db {
namespace common {

class RAG3DB_API CopyException : public Exception {
public:
    explicit CopyException(const std::string& msg) : Exception("Copy exception: " + msg){};
};

} // namespace common
} // namespace rag3db
