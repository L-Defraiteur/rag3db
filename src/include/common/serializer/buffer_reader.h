#pragma once

#include <cstring>

#include "common/serializer/reader.h"

namespace rag3db {
namespace common {

struct BufferReader final : Reader {
    BufferReader(uint8_t* data, size_t dataSize) : data(data), dataSize(dataSize), readSize(0) {}

    void read(uint8_t* outputData, uint64_t size) override {
        memcpy(outputData, data + readSize, size);
        readSize += size;
    }

    bool finished() override { return readSize >= dataSize; }

    uint8_t* data;
    size_t dataSize;
    size_t readSize;
};

} // namespace common
} // namespace rag3db
