#pragma once

#include "binder/bound_database_statement.h"

namespace rag3db {
namespace binder {

class BoundDetachDatabase final : public BoundDatabaseStatement {
public:
    explicit BoundDetachDatabase(std::string dbName)
        : BoundDatabaseStatement{common::StatementType::DETACH_DATABASE, std::move(dbName)} {}
};

} // namespace binder
} // namespace rag3db
