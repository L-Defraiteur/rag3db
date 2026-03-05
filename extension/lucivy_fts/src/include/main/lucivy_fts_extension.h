#pragma once
#include "extension/extension.h"

namespace rag3db {
namespace lucivy_fts_extension {

class LucivyFtsExtension final : public extension::Extension {
public:
    static constexpr char EXTENSION_NAME[] = "LUCIVY_FTS";
    static void load(main::ClientContext* context);
};

} // namespace lucivy_fts_extension
} // namespace rag3db
