#pragma once

#include "common/api.h"
#include "exception.h"

namespace rag3db {
namespace common {

class RAG3DB_API CatalogException : public Exception {
public:
    explicit CatalogException(const std::string& msg) : Exception("Catalog exception: " + msg){};
};

} // namespace common
} // namespace rag3db
