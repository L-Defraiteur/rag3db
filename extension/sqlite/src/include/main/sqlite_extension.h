#pragma once

#include "extension/extension.h"

namespace rag3db {
namespace sqlite_extension {

class SqliteExtension final : public extension::Extension {
public:
    static constexpr char EXTENSION_NAME[] = "SQLITE";

public:
    static void load(main::ClientContext* context);
};

} // namespace sqlite_extension
} // namespace rag3db
