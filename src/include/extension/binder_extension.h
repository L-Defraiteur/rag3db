#pragma once

#include "binder/bound_statement.h"
#include "parser/statement.h"

namespace rag3db {
namespace extension {

class RAG3DB_API BinderExtension {
public:
    BinderExtension() {}

    virtual ~BinderExtension() = default;

    virtual std::unique_ptr<binder::BoundStatement> bind(const parser::Statement& statement) = 0;
};

} // namespace extension
} // namespace rag3db
