#pragma once

#include "common/types/types.h"
#include "common/vector/value_vector.h"
#include "math/quaternion.h"
#include "math/matrix3.h"

namespace rag3db {
namespace geo_extension {

using namespace common;

// Extract a fixed-size LIST(DOUBLE) from a ValueVector into a double array.
// Returns false if null or wrong size.
inline bool extractListDouble(const ValueVector& vec, const SelectionVector& selVec,
    uint32_t idx, double* out, uint32_t expectedSize) {
    auto pos = selVec[vec.state->isFlat() ? 0 : idx];
    if (vec.isNull(pos)) {
        return false;
    }
    auto entry = vec.getValue<list_entry_t>(pos);
    if (entry.size != expectedSize) {
        return false;
    }
    auto* dataVec = ListVector::getDataVector(&vec);
    for (uint32_t k = 0; k < expectedSize; k++) {
        out[k] = dataVec->getValue<double>(entry.offset + k);
    }
    return true;
}

// Extract a variable-size LIST(DOUBLE). Caller provides output vector.
// Returns false if null. Sets outSize.
inline bool extractListDoubleVar(const ValueVector& vec, const SelectionVector& selVec,
    uint32_t idx, std::vector<double>& out, uint32_t& outSize) {
    auto pos = selVec[vec.state->isFlat() ? 0 : idx];
    if (vec.isNull(pos)) {
        return false;
    }
    auto entry = vec.getValue<list_entry_t>(pos);
    outSize = entry.size;
    out.resize(outSize);
    auto* dataVec = ListVector::getDataVector(&vec);
    for (uint32_t k = 0; k < outSize; k++) {
        out[k] = dataVec->getValue<double>(entry.offset + k);
    }
    return true;
}

// Extract a quaternion from a LIST(DOUBLE) of 4 elements.
// Three.js convention: [x, y, z, w] → Quat{w, x, y, z}.
inline bool extractQuat(const ValueVector& vec, const SelectionVector& selVec, uint32_t idx,
    math::Quat& q) {
    double v[4];
    if (!extractListDouble(vec, selVec, idx, v, 4)) {
        return false;
    }
    q = {v[3], v[0], v[1], v[2]}; // [x,y,z,w] → {w,x,y,z}
    return true;
}

// Extract a Mat3 from a LIST(DOUBLE) of 9 elements.
// Three.js convention: column-major [m00,m10,m20, m01,m11,m21, m02,m12,m22].
// Stored internally as row-major.
inline bool extractMat3(const ValueVector& vec, const SelectionVector& selVec, uint32_t idx,
    math::Mat3& mat) {
    double col[9];
    if (!extractListDouble(vec, selVec, idx, col, 9)) {
        return false;
    }
    // Column-major to row-major: m[row*3+col] = col[col*3+row]
    mat.m[0] = col[0];
    mat.m[1] = col[3];
    mat.m[2] = col[6];
    mat.m[3] = col[1];
    mat.m[4] = col[4];
    mat.m[5] = col[7];
    mat.m[6] = col[2];
    mat.m[7] = col[5];
    mat.m[8] = col[8];
    return true;
}

// Write a Quat to a LIST(DOUBLE) result vector.
// Three.js convention: output [x, y, z, w].
inline void writeQuatToList(ValueVector& result, uint32_t resPos, const math::Quat& q) {
    auto entry = ListVector::addList(&result, 4);
    result.setValue(resPos, entry);
    auto* dataVec = ListVector::getDataVector(&result);
    dataVec->setValue<double>(entry.offset + 0, q.x);
    dataVec->setValue<double>(entry.offset + 1, q.y);
    dataVec->setValue<double>(entry.offset + 2, q.z);
    dataVec->setValue<double>(entry.offset + 3, q.w);
}

// Write a Mat3 to a LIST(DOUBLE) result vector.
// Three.js convention: output column-major [m00,m10,m20, m01,m11,m21, m02,m12,m22].
inline void writeMat3ToList(ValueVector& result, uint32_t resPos, const math::Mat3& mat) {
    auto entry = ListVector::addList(&result, 9);
    result.setValue(resPos, entry);
    auto* dataVec = ListVector::getDataVector(&result);
    // Row-major to column-major.
    dataVec->setValue<double>(entry.offset + 0, mat.m[0]);
    dataVec->setValue<double>(entry.offset + 1, mat.m[3]);
    dataVec->setValue<double>(entry.offset + 2, mat.m[6]);
    dataVec->setValue<double>(entry.offset + 3, mat.m[1]);
    dataVec->setValue<double>(entry.offset + 4, mat.m[4]);
    dataVec->setValue<double>(entry.offset + 5, mat.m[7]);
    dataVec->setValue<double>(entry.offset + 6, mat.m[2]);
    dataVec->setValue<double>(entry.offset + 7, mat.m[5]);
    dataVec->setValue<double>(entry.offset + 8, mat.m[8]);
}

// Write a 3D point to a LIST(DOUBLE) result vector.
inline void writePoint3ToList(ValueVector& result, uint32_t resPos, double x, double y,
    double z) {
    auto entry = ListVector::addList(&result, 3);
    result.setValue(resPos, entry);
    auto* dataVec = ListVector::getDataVector(&result);
    dataVec->setValue<double>(entry.offset + 0, x);
    dataVec->setValue<double>(entry.offset + 1, y);
    dataVec->setValue<double>(entry.offset + 2, z);
}

// Write a flat double array to a LIST(DOUBLE) result vector.
inline void writeDoubleListToResult(ValueVector& result, uint32_t resPos, const double* data,
    uint32_t size) {
    auto entry = ListVector::addList(&result, size);
    result.setValue(resPos, entry);
    auto* dataVec = ListVector::getDataVector(&result);
    for (uint32_t k = 0; k < size; k++) {
        dataVec->setValue<double>(entry.offset + k, data[k]);
    }
}

// Extract a DOUBLE scalar parameter.
inline bool extractDouble(const ValueVector& vec, const SelectionVector& selVec, uint32_t idx,
    double& out) {
    auto pos = selVec[vec.state->isFlat() ? 0 : idx];
    if (vec.isNull(pos)) {
        return false;
    }
    out = vec.getValue<double>(pos);
    return true;
}

} // namespace geo_extension
} // namespace rag3db
