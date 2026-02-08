#pragma once

#include "common/api.h"
#include "common/types/ku_string.h"
#include "common/vector/value_vector.h"

namespace rag3db {
namespace function {

struct BaseStrOperation {
public:
    RAG3DB_API static void operation(common::ku_string_t& input, common::ku_string_t& result,
        common::ValueVector& resultValueVector, uint32_t (*strOperation)(char* data, uint32_t len));
};

} // namespace function
} // namespace rag3db
