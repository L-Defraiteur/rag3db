#pragma once

#include "common/api.h"
#include "exception.h"

namespace rag3db {
namespace common {

class RAG3DB_API ConversionException : public Exception {
public:
    explicit ConversionException(const std::string& msg)
        : Exception("Conversion exception: " + msg) {}
};

} // namespace common
} // namespace rag3db
