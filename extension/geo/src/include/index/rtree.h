#pragma once

#include "math/quaternion.h"

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <functional>
#include <limits>
#include <memory>
#include <queue>
#include <unordered_map>
#include <utility>
#include <vector>

namespace rag3db {
namespace geo_extension {

struct BoundingBox {
    std::vector<double> minCoords;
    std::vector<double> maxCoords;

    BoundingBox() = default;
    explicit BoundingBox(uint32_t dims);
    static BoundingBox fromPoint(const std::vector<double>& point);

    uint32_t dims() const { return static_cast<uint32_t>(minCoords.size()); }

    void expand(const BoundingBox& other);
    double area() const;
    double enlargement(const BoundingBox& other) const;
    bool intersects(const BoundingBox& other) const;
    bool containsPoint(const std::vector<double>& point) const;

    // Minimum distance from a point to this bbox (0 if inside).
    double minDistToPoint(const std::vector<double>& point) const;
};

struct RTreeEntry {
    BoundingBox bbox;
    uint64_t data; // leaf: node_offset in table. internal: child index in nodes_ vector.
};

struct RTreeNode {
    bool isLeaf;
    std::vector<RTreeEntry> entries;

    RTreeNode() : isLeaf(true) {}
};

enum class DistanceMetric : uint8_t { EUCLIDEAN = 0, HAVERSINE = 1 };

class RTree {
public:
    RTree(uint32_t dimensions, uint32_t maxEntries = 16, uint32_t minEntries = 4,
        DistanceMetric metric = DistanceMetric::EUCLIDEAN);

    void insert(uint64_t nodeOffset, const std::vector<double>& coords);
    bool remove(uint64_t nodeOffset);
    void update(uint64_t nodeOffset, const std::vector<double>& newCoords);

    // KNN query: returns (nodeOffset, distance) sorted ascending.
    std::vector<std::pair<uint64_t, double>> knn(const std::vector<double>& query,
        uint32_t k) const;

    // Bounding box query: returns all nodeOffsets whose point intersects bbox.
    std::vector<uint64_t> searchBBox(const BoundingBox& bbox) const;

    // Radius query: returns (nodeOffset, distance) within radius.
    std::vector<std::pair<uint64_t, double>> searchRadius(const std::vector<double>& center,
        double radius) const;

    // OBB query: returns (nodeOffset, distance) for points inside the oriented bounding box.
    // quat is [w,x,y,z] internally.
    std::vector<std::pair<uint64_t, double>> searchOBB(const std::vector<double>& center,
        const std::vector<double>& halfExtents, const math::Quat& quat) const;

    // Frustum/convex query: returns (nodeOffset, distance) for points inside the convex hull.
    // planes is flat N*4 array, refPoint used for distance computation.
    std::vector<std::pair<uint64_t, double>> searchFrustum(const double* planes,
        uint32_t numPlanes, const std::vector<double>& refPoint) const;

    // Get stored coordinates for a nodeOffset (for post-filter queries).
    bool getCoords(uint64_t nodeOffset, std::vector<double>& coords) const;

    uint32_t dimensions() const { return dims_; }
    uint64_t size() const { return size_; }
    DistanceMetric metric() const { return metric_; }

    // Binary serialization.
    void serialize(std::vector<uint8_t>& buffer) const;
    static std::unique_ptr<RTree> deserialize(const uint8_t* data, size_t len);

private:
    uint32_t dims_;
    uint32_t maxEntries_;
    uint32_t minEntries_;
    uint64_t size_ = 0;
    DistanceMetric metric_;

    std::vector<RTreeNode> nodes_;
    uint32_t root_ = 0;
    // Map from leaf data (nodeOffset) to the node index that contains it, for fast removal.
    std::unordered_map<uint64_t, uint32_t> offsetToLeaf_;

    // Core algorithms.
    uint32_t chooseLeaf(const BoundingBox& bbox) const;
    void insertEntry(uint32_t nodeIdx, const RTreeEntry& entry);
    void splitNode(uint32_t nodeIdx);
    void adjustBBox(uint32_t nodeIdx);
    void condenseTree(uint32_t nodeIdx);
    uint32_t parentOf(uint32_t nodeIdx) const;
    void rebuildParentMap();

    // Distance computation.
    double pointDistance(const std::vector<double>& a, const std::vector<double>& b) const;
    static double haversineDistance(double lat1, double lon1, double lat2, double lon2);
    static double euclideanDistance(const std::vector<double>& a, const std::vector<double>& b);

    // Parent tracking (simple approach: stored in a map).
    std::unordered_map<uint32_t, uint32_t> parentMap_;
};

} // namespace geo_extension
} // namespace rag3db
