#pragma once

#include <cstdint>

#include "common/api.h"

namespace rag3db {
namespace common {

class Value;

class NestedVal {
public:
    RAG3DB_API static uint32_t getChildrenSize(const Value* val);

    RAG3DB_API static Value* getChildVal(const Value* val, uint32_t idx);
};

} // namespace common
} // namespace rag3db
