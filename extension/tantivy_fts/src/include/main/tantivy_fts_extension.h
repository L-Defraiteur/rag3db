#pragma once
#include "extension/extension.h"

namespace kuzu {
namespace tantivy_fts_extension {

class TantivyFtsExtension final : public extension::Extension {
public:
    static constexpr char EXTENSION_NAME[] = "TANTIVY_FTS";
    static void load(main::ClientContext* context);
};

} // namespace tantivy_fts_extension
} // namespace kuzu
