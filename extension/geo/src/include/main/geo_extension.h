#pragma once

#include "extension/extension.h"

namespace rag3db {
namespace geo_extension {

class GeoExtension final : public extension::Extension {
public:
    static constexpr char EXTENSION_NAME[] = "GEO";

    static void load(main::ClientContext* context);
};

} // namespace geo_extension
} // namespace rag3db
