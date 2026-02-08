#pragma once

#include "extension/extension_installer.h"

namespace rag3db {
namespace duckdb_extension {

class RAG3DB_API DuckDBInstaller final : public extension::ExtensionInstaller {
public:
    DuckDBInstaller(const extension::InstallExtensionInfo& info, main::ClientContext& context)
        : ExtensionInstaller{info, context} {}

    bool install() override;
};

} // namespace duckdb_extension
} // namespace rag3db
