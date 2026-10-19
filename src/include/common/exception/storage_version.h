#pragma once

#include "common/api.h"
#include "exception.h"

namespace rag3db {
namespace common {

// Une base ou un journal d'un autre format que celui de ce moteur : refusé avant de lire quoi
// que ce soit d'autre, en nommant les deux versions et les deux bâtis. Ce n'est ni une
// corruption ni une panne : le rejeu ne l'avale pas.
class RAG3DB_API StorageVersionException : public Exception {
public:
    explicit StorageVersionException(const std::string& msg) : Exception(msg) {}
};

} // namespace common
} // namespace rag3db
