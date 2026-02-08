#pragma once

#include "common/api.h"
#include "exception.h"

namespace rag3db {
namespace common {

class RAG3DB_API BinderException : public Exception {
public:
    explicit BinderException(const std::string& msg) : Exception("Binder exception: " + msg){};
};

} // namespace common
} // namespace rag3db
