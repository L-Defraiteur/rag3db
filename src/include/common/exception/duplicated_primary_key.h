#pragma once

#include "common/api.h"
#include "common/exception/message.h"
#include "common/exception/runtime.h"

namespace rag3db {
namespace common {

// Une clé primaire en double à l'insertion. Un type à part : au rejeu du journal, l'index de clé
// note un doublon validé (un journal du moteur d'avant le verrou de clé, marche A3′ reprise) au
// lieu de lever ; hors reprise, c'est cette exception.
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
