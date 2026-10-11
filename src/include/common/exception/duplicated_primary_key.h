#pragma once

#include "common/api.h"
#include "common/exception/message.h"
#include "common/exception/runtime.h"

namespace rag3db {
namespace common {

// Une clé primaire en double à l'insertion. Un type à part : au rejeu du journal, une transaction
// validée dont une insertion heurte un doublon (un journal du moteur d'avant le verrou de clé,
// marche A3′) est annulée et sautée, là où toute autre erreur reste une corruption du journal.
class RAG3DB_API DuplicatedPrimaryKeyException : public RuntimeException {
public:
    explicit DuplicatedPrimaryKeyException(const std::string& pkString)
        : RuntimeException(ExceptionMessage::duplicatePKException(pkString)), key{pkString} {}

    const std::string& getKey() const { return key; }

private:
    std::string key;
};

} // namespace common
} // namespace rag3db
