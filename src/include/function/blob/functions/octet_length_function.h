#pragma once

#include "common/types/blob.h"

namespace rag3db {
namespace function {

struct OctetLength {
    static inline void operation(common::blob_t& input, int64_t& result) {
        result = input.value.len;
    }
};

} // namespace function
} // namespace rag3db
