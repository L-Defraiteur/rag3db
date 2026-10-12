#include "common/roaring_mask.h"

namespace rag3db {
namespace common {

offset_vec_t Roaring32BitmapSemiMask::collectMaskedNodes(uint64_t size) const {
    offset_vec_t result;
    result.reserve(size);
    auto it = roaring->begin();
    for (; it != roaring->end(); it++) {
        auto value = *it;
        result.push_back(value);
        if (result.size() == size) {
            break;
        }
    }
    return result;
}

void Roaring32BitmapSemiMask::collectFrom(offset_t start, uint64_t maxCount,
    offset_vec_t& out) const {
    if (start > UINT32_MAX) {
        return; // rien au-delà de ce qu'un masque 32 bits représente
    }
    auto it = roaring->begin();
    it.equalorlarger(static_cast<uint32_t>(start));
    for (; it != roaring->end() && maxCount > 0; it++, maxCount--) {
        out.push_back(*it);
    }
}

offset_vec_t Roaring32BitmapSemiMask::range(uint32_t start, uint32_t end) {
    auto it = roaring->begin();
    it.equalorlarger(start);
    offset_vec_t ans;
    for (; it != roaring->end(); it++) {
        auto value = *it;
        if (value >= end) {
            break;
        }
        ans.push_back(value);
    }
    return ans;
}

offset_vec_t Roaring64BitmapSemiMask::collectMaskedNodes(uint64_t size) const {
    offset_vec_t result;
    result.reserve(size);
    auto it = roaring->begin();
    for (; it != roaring->end(); it++) {
        auto value = *it;
        result.push_back(value);
        if (result.size() == size) {
            break;
        }
    }
    return result;
}

void Roaring64BitmapSemiMask::collectFrom(offset_t start, uint64_t maxCount,
    offset_vec_t& out) const {
    auto it = roaring->begin();
    it.move(start);
    for (; it != roaring->end() && maxCount > 0; it++, maxCount--) {
        out.push_back(*it);
    }
}

offset_vec_t Roaring64BitmapSemiMask::range(uint32_t start, uint32_t end) {
    auto it = roaring->begin();
    it.move(start);
    offset_vec_t ans;
    for (; it != roaring->end(); it++) {
        auto value = *it;
        if (value >= end) {
            break;
        }
        ans.push_back(value);
    }
    return ans;
}

} // namespace common
} // namespace rag3db
