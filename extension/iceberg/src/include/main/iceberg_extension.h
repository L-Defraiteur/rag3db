#pragma once

#include "extension/extension.h"

namespace rag3db {
namespace iceberg_extension {

class IcebergExtension final : public extension::Extension {
public:
    static constexpr char EXTENSION_NAME[] = "ICEBERG";

public:
    static void load(main::ClientContext* context);
};

} // namespace iceberg_extension
} // namespace rag3db
