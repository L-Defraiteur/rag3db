#pragma once

#include "bound_statement.h"
#include "extension/extension_action.h"

namespace rag3db {
namespace binder {

using namespace rag3db::extension;

class BoundExtensionStatement final : public BoundStatement {
    static constexpr common::StatementType type_ = common::StatementType::EXTENSION;

public:
    explicit BoundExtensionStatement(std::unique_ptr<ExtensionAuxInfo> info)
        : BoundStatement{type_, BoundStatementResult::createSingleStringColumnResult()},
          info{std::move(info)} {}

    std::unique_ptr<ExtensionAuxInfo> getAuxInfo() const { return info->copy(); }

private:
    std::unique_ptr<ExtensionAuxInfo> info;
};

} // namespace binder
} // namespace rag3db
