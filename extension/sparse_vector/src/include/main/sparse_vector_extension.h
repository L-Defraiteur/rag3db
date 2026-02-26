#pragma once
#include "extension/extension.h"

namespace rag3db {
namespace sparse_vector_extension {

class SparseVectorExtension final : public extension::Extension {
public:
    static constexpr char EXTENSION_NAME[] = "SPARSE_VECTOR";
    static void load(main::ClientContext* context);
};

} // namespace sparse_vector_extension
} // namespace rag3db
