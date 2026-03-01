#include "index/rtree.h"
#include "math/geometry.h"

#include <cassert>
#include <cstring>

namespace rag3db {
namespace geo_extension {

static constexpr double EARTH_RADIUS_M = 6'371'000.0;
static constexpr double DEG_TO_RAD = M_PI / 180.0;

// ─── BoundingBox ───────────────────────────────────────────────────────────────

BoundingBox::BoundingBox(uint32_t dims)
    : minCoords(dims, std::numeric_limits<double>::max()),
      maxCoords(dims, std::numeric_limits<double>::lowest()) {}

BoundingBox BoundingBox::fromPoint(const std::vector<double>& point) {
    BoundingBox bb;
    bb.minCoords = point;
    bb.maxCoords = point;
    return bb;
}

void BoundingBox::expand(const BoundingBox& other) {
    for (uint32_t d = 0; d < dims(); d++) {
        minCoords[d] = std::min(minCoords[d], other.minCoords[d]);
        maxCoords[d] = std::max(maxCoords[d], other.maxCoords[d]);
    }
}

double BoundingBox::area() const {
    double a = 1.0;
    for (uint32_t d = 0; d < dims(); d++) {
        a *= (maxCoords[d] - minCoords[d]);
    }
    return a;
}

double BoundingBox::enlargement(const BoundingBox& other) const {
    BoundingBox expanded = *this;
    expanded.expand(other);
    return expanded.area() - area();
}

bool BoundingBox::intersects(const BoundingBox& other) const {
    for (uint32_t d = 0; d < dims(); d++) {
        if (minCoords[d] > other.maxCoords[d] || maxCoords[d] < other.minCoords[d]) {
            return false;
        }
    }
    return true;
}

bool BoundingBox::containsPoint(const std::vector<double>& point) const {
    for (uint32_t d = 0; d < dims(); d++) {
        if (point[d] < minCoords[d] || point[d] > maxCoords[d]) {
            return false;
        }
    }
    return true;
}

double BoundingBox::minDistToPoint(const std::vector<double>& point) const {
    double dist2 = 0.0;
    for (uint32_t d = 0; d < dims(); d++) {
        double v = 0.0;
        if (point[d] < minCoords[d]) {
            v = minCoords[d] - point[d];
        } else if (point[d] > maxCoords[d]) {
            v = point[d] - maxCoords[d];
        }
        dist2 += v * v;
    }
    return std::sqrt(dist2);
}

// ─── RTree ─────────────────────────────────────────────────────────────────────

RTree::RTree(uint32_t dimensions, uint32_t maxEntries, uint32_t minEntries, DistanceMetric metric)
    : dims_(dimensions), maxEntries_(maxEntries), minEntries_(std::min(minEntries, maxEntries / 2)),
      metric_(metric) {
    // Create root node (empty leaf).
    nodes_.emplace_back();
    root_ = 0;
}

// ─── Distance ──────────────────────────────────────────────────────────────────

double RTree::haversineDistance(double lat1, double lon1, double lat2, double lon2) {
    double dlat = (lat2 - lat1) * DEG_TO_RAD;
    double dlon = (lon2 - lon1) * DEG_TO_RAD;
    double a = std::sin(dlat / 2) * std::sin(dlat / 2) +
               std::cos(lat1 * DEG_TO_RAD) * std::cos(lat2 * DEG_TO_RAD) *
                   std::sin(dlon / 2) * std::sin(dlon / 2);
    return 2.0 * EARTH_RADIUS_M * std::asin(std::sqrt(a));
}

double RTree::euclideanDistance(const std::vector<double>& a, const std::vector<double>& b) {
    double sum = 0.0;
    for (size_t i = 0; i < a.size(); i++) {
        double d = a[i] - b[i];
        sum += d * d;
    }
    return std::sqrt(sum);
}

double RTree::pointDistance(const std::vector<double>& a, const std::vector<double>& b) const {
    if (metric_ == DistanceMetric::HAVERSINE && dims_ >= 2) {
        return haversineDistance(a[0], a[1], b[0], b[1]);
    }
    return euclideanDistance(a, b);
}

// ─── Insert ────────────────────────────────────────────────────────────────────

uint32_t RTree::chooseLeaf(const BoundingBox& bbox) const {
    uint32_t current = root_;
    while (!nodes_[current].isLeaf) {
        double bestEnlargement = std::numeric_limits<double>::max();
        double bestArea = std::numeric_limits<double>::max();
        uint32_t bestChild = 0;
        for (const auto& entry : nodes_[current].entries) {
            double enl = entry.bbox.enlargement(bbox);
            double a = entry.bbox.area();
            if (enl < bestEnlargement || (enl == bestEnlargement && a < bestArea)) {
                bestEnlargement = enl;
                bestArea = a;
                bestChild = static_cast<uint32_t>(entry.data);
            }
        }
        current = bestChild;
    }
    return current;
}

void RTree::insert(uint64_t nodeOffset, const std::vector<double>& coords) {
    auto bbox = BoundingBox::fromPoint(coords);
    RTreeEntry entry{bbox, nodeOffset};

    uint32_t leafIdx = chooseLeaf(bbox);
    insertEntry(leafIdx, entry);
    offsetToLeaf_[nodeOffset] = leafIdx;
    size_++;
}

void RTree::insertEntry(uint32_t nodeIdx, const RTreeEntry& entry) {
    nodes_[nodeIdx].entries.push_back(entry);
    if (nodes_[nodeIdx].entries.size() > maxEntries_) {
        splitNode(nodeIdx);
    }
}

void RTree::splitNode(uint32_t nodeIdx) {
    // Move entries out before modifying nodes_ to avoid reference invalidation.
    auto entries = std::move(nodes_[nodeIdx].entries);
    bool isLeaf = nodes_[nodeIdx].isLeaf;
    nodes_[nodeIdx].entries.clear();

    // Quadratic split: pick seeds with maximum waste.
    size_t seed1 = 0, seed2 = 1;
    double maxWaste = std::numeric_limits<double>::lowest();
    for (size_t i = 0; i < entries.size(); i++) {
        for (size_t j = i + 1; j < entries.size(); j++) {
            BoundingBox combined = entries[i].bbox;
            combined.expand(entries[j].bbox);
            double waste = combined.area() - entries[i].bbox.area() - entries[j].bbox.area();
            if (waste > maxWaste) {
                maxWaste = waste;
                seed1 = i;
                seed2 = j;
            }
        }
    }

    // Create new sibling node. Note: this may invalidate all references into nodes_.
    uint32_t newNodeIdx = static_cast<uint32_t>(nodes_.size());
    nodes_.emplace_back();
    nodes_[newNodeIdx].isLeaf = isLeaf;

    // Assign seeds. Use index access, not references, since nodes_ may reallocate.
    nodes_[nodeIdx].entries.push_back(entries[seed1]);
    nodes_[newNodeIdx].entries.push_back(entries[seed2]);

    BoundingBox bb1 = entries[seed1].bbox;
    BoundingBox bb2 = entries[seed2].bbox;

    // Distribute remaining entries.
    for (size_t i = 0; i < entries.size(); i++) {
        if (i == seed1 || i == seed2) continue;

        // If one group needs all remaining to reach minEntries, assign to it.
        size_t remaining = entries.size() - i;
        if (nodes_[nodeIdx].entries.size() + remaining <= minEntries_) {
            nodes_[nodeIdx].entries.push_back(entries[i]);
            bb1.expand(entries[i].bbox);
            continue;
        }
        if (nodes_[newNodeIdx].entries.size() + remaining <= minEntries_) {
            nodes_[newNodeIdx].entries.push_back(entries[i]);
            bb2.expand(entries[i].bbox);
            continue;
        }

        double enl1 = bb1.enlargement(entries[i].bbox);
        double enl2 = bb2.enlargement(entries[i].bbox);
        if (enl1 < enl2 || (enl1 == enl2 && bb1.area() < bb2.area())) {
            nodes_[nodeIdx].entries.push_back(entries[i]);
            bb1.expand(entries[i].bbox);
        } else {
            nodes_[newNodeIdx].entries.push_back(entries[i]);
            bb2.expand(entries[i].bbox);
        }
    }

    // Update offsetToLeaf_ for entries that moved to new node.
    if (isLeaf) {
        for (const auto& e : nodes_[newNodeIdx].entries) {
            offsetToLeaf_[e.data] = newNodeIdx;
        }
        for (const auto& e : nodes_[nodeIdx].entries) {
            offsetToLeaf_[e.data] = nodeIdx;
        }
    } else {
        // Update parentMap_ for children that moved.
        for (const auto& e : nodes_[newNodeIdx].entries) {
            parentMap_[static_cast<uint32_t>(e.data)] = newNodeIdx;
        }
    }

    // Propagate split up to parent.
    if (nodeIdx == root_) {
        // Create new root. This may invalidate references again.
        uint32_t newRootIdx = static_cast<uint32_t>(nodes_.size());
        nodes_.emplace_back();
        nodes_[newRootIdx].isLeaf = false;

        BoundingBox bboxOld(dims_);
        for (const auto& e : nodes_[nodeIdx].entries) bboxOld.expand(e.bbox);
        BoundingBox bboxNew(dims_);
        for (const auto& e : nodes_[newNodeIdx].entries) bboxNew.expand(e.bbox);

        nodes_[newRootIdx].entries.push_back({bboxOld, nodeIdx});
        nodes_[newRootIdx].entries.push_back({bboxNew, newNodeIdx});

        parentMap_[nodeIdx] = newRootIdx;
        parentMap_[newNodeIdx] = newRootIdx;
        root_ = newRootIdx;
    } else {
        uint32_t parentIdx = parentMap_[nodeIdx];

        // Update existing entry bbox for nodeIdx.
        for (auto& e : nodes_[parentIdx].entries) {
            if (e.data == nodeIdx) {
                e.bbox = BoundingBox(dims_);
                for (const auto& ce : nodes_[nodeIdx].entries) e.bbox.expand(ce.bbox);
                break;
            }
        }

        // Add new entry for newNodeIdx.
        BoundingBox bboxNew(dims_);
        for (const auto& e : nodes_[newNodeIdx].entries) bboxNew.expand(e.bbox);
        parentMap_[newNodeIdx] = parentIdx;
        insertEntry(parentIdx, {bboxNew, newNodeIdx}); // May cause recursive split.
    }
}

// ─── Remove ────────────────────────────────────────────────────────────────────

bool RTree::remove(uint64_t nodeOffset) {
    auto it = offsetToLeaf_.find(nodeOffset);
    if (it == offsetToLeaf_.end()) return false;

    uint32_t leafIdx = it->second;
    auto& entries = nodes_[leafIdx].entries;

    // Remove the entry.
    for (auto eit = entries.begin(); eit != entries.end(); ++eit) {
        if (eit->data == nodeOffset) {
            entries.erase(eit);
            break;
        }
    }
    offsetToLeaf_.erase(it);
    size_--;

    // Condense tree: propagate underflow up.
    condenseTree(leafIdx);
    return true;
}

void RTree::condenseTree(uint32_t nodeIdx) {
    // Collect entries from underflowing nodes to reinsert.
    std::vector<RTreeEntry> orphans;

    uint32_t current = nodeIdx;
    while (current != root_) {
        uint32_t parent = parentMap_[current];
        if (nodes_[current].entries.size() < minEntries_) {
            // Remove current from parent.
            auto& parentEntries = nodes_[parent].entries;
            for (auto it = parentEntries.begin(); it != parentEntries.end(); ++it) {
                if (static_cast<uint32_t>(it->data) == current) {
                    parentEntries.erase(it);
                    break;
                }
            }
            // Collect orphans.
            if (nodes_[current].isLeaf) {
                for (auto& e : nodes_[current].entries) {
                    orphans.push_back(std::move(e));
                }
            } else {
                // For internal nodes, collect leaf entries recursively.
                std::queue<uint32_t> q;
                q.push(current);
                while (!q.empty()) {
                    uint32_t n = q.front();
                    q.pop();
                    if (nodes_[n].isLeaf) {
                        for (auto& e : nodes_[n].entries) {
                            orphans.push_back(std::move(e));
                        }
                    } else {
                        for (auto& e : nodes_[n].entries) {
                            q.push(static_cast<uint32_t>(e.data));
                        }
                    }
                }
            }
        } else {
            // Update parent's bbox for current.
            for (auto& pe : nodes_[parent].entries) {
                if (static_cast<uint32_t>(pe.data) == current) {
                    pe.bbox = BoundingBox(dims_);
                    for (const auto& e : nodes_[current].entries) {
                        pe.bbox.expand(e.bbox);
                    }
                    break;
                }
            }
        }
        current = parent;
    }

    // If root has only one child and is not a leaf, make child the new root.
    if (!nodes_[root_].isLeaf && nodes_[root_].entries.size() == 1) {
        root_ = static_cast<uint32_t>(nodes_[root_].entries[0].data);
    }

    // Reinsert orphans.
    for (auto& orphan : orphans) {
        uint64_t offset = orphan.data;
        uint32_t leafNode = chooseLeaf(orphan.bbox);
        insertEntry(leafNode, orphan);
        offsetToLeaf_[offset] = leafNode;
    }
}

// ─── Update ────────────────────────────────────────────────────────────────────

void RTree::update(uint64_t nodeOffset, const std::vector<double>& newCoords) {
    remove(nodeOffset);
    insert(nodeOffset, newCoords);
}

// ─── KNN Query ─────────────────────────────────────────────────────────────────

std::vector<std::pair<uint64_t, double>> RTree::knn(const std::vector<double>& query,
    uint32_t k) const {
    if (size_ == 0) return {};

    // Max-heap of (distance, nodeOffset) — keeps k nearest.
    using Result = std::pair<double, uint64_t>;
    std::priority_queue<Result> results;

    // Min-heap of (minDist, nodeIndex) for branch-and-bound.
    using BranchItem = std::pair<double, uint32_t>;
    std::priority_queue<BranchItem, std::vector<BranchItem>, std::greater<>> branches;

    branches.push({0.0, root_});

    while (!branches.empty()) {
        auto [minDist, idx] = branches.top();
        branches.pop();

        // Prune: if we already have k results and this branch is farther than the kth, skip.
        if (results.size() >= k && minDist >= results.top().first) {
            continue;
        }

        const auto& node = nodes_[idx];
        if (node.isLeaf) {
            for (const auto& entry : node.entries) {
                // entry.bbox is a point (min == max).
                double dist = pointDistance(query, entry.bbox.minCoords);
                if (results.size() < k) {
                    results.push({dist, entry.data});
                } else if (dist < results.top().first) {
                    results.pop();
                    results.push({dist, entry.data});
                }
            }
        } else {
            for (const auto& entry : node.entries) {
                double d = entry.bbox.minDistToPoint(query);
                if (results.size() < k || d < results.top().first) {
                    branches.push({d, static_cast<uint32_t>(entry.data)});
                }
            }
        }
    }

    // Collect results in ascending order.
    std::vector<std::pair<uint64_t, double>> out;
    out.reserve(results.size());
    while (!results.empty()) {
        out.push_back({results.top().second, results.top().first});
        results.pop();
    }
    std::reverse(out.begin(), out.end());
    return out;
}

// ─── Bounding Box Query ────────────────────────────────────────────────────────

std::vector<uint64_t> RTree::searchBBox(const BoundingBox& bbox) const {
    std::vector<uint64_t> results;
    if (size_ == 0) return results;

    std::queue<uint32_t> q;
    q.push(root_);
    while (!q.empty()) {
        uint32_t idx = q.front();
        q.pop();
        const auto& node = nodes_[idx];
        for (const auto& entry : node.entries) {
            if (!entry.bbox.intersects(bbox)) continue;
            if (node.isLeaf) {
                results.push_back(entry.data);
            } else {
                q.push(static_cast<uint32_t>(entry.data));
            }
        }
    }
    return results;
}

// ─── Radius Query ──────────────────────────────────────────────────────────────

std::vector<std::pair<uint64_t, double>> RTree::searchRadius(const std::vector<double>& center,
    double radius) const {
    std::vector<std::pair<uint64_t, double>> results;
    if (size_ == 0) return results;

    std::queue<uint32_t> q;
    q.push(root_);
    while (!q.empty()) {
        uint32_t idx = q.front();
        q.pop();
        const auto& node = nodes_[idx];
        for (const auto& entry : node.entries) {
            double minDist = entry.bbox.minDistToPoint(center);
            if (minDist > radius) continue;
            if (node.isLeaf) {
                double dist = pointDistance(center, entry.bbox.minCoords);
                if (dist <= radius) {
                    results.push_back({entry.data, dist});
                }
            } else {
                q.push(static_cast<uint32_t>(entry.data));
            }
        }
    }

    // Sort by distance ascending.
    std::sort(results.begin(), results.end(),
        [](const auto& a, const auto& b) { return a.second < b.second; });
    return results;
}

// ─── Serialization ─────────────────────────────────────────────────────────────

static void writeU32(std::vector<uint8_t>& buf, uint32_t val) {
    buf.insert(buf.end(), reinterpret_cast<const uint8_t*>(&val),
        reinterpret_cast<const uint8_t*>(&val) + 4);
}

static void writeU64(std::vector<uint8_t>& buf, uint64_t val) {
    buf.insert(buf.end(), reinterpret_cast<const uint8_t*>(&val),
        reinterpret_cast<const uint8_t*>(&val) + 8);
}

static void writeF64(std::vector<uint8_t>& buf, double val) {
    buf.insert(buf.end(), reinterpret_cast<const uint8_t*>(&val),
        reinterpret_cast<const uint8_t*>(&val) + 8);
}

static void writeU8(std::vector<uint8_t>& buf, uint8_t val) {
    buf.push_back(val);
}

static uint32_t readU32(const uint8_t*& ptr) {
    uint32_t val;
    std::memcpy(&val, ptr, 4);
    ptr += 4;
    return val;
}

static uint64_t readU64(const uint8_t*& ptr) {
    uint64_t val;
    std::memcpy(&val, ptr, 8);
    ptr += 8;
    return val;
}

static double readF64(const uint8_t*& ptr) {
    double val;
    std::memcpy(&val, ptr, 8);
    ptr += 8;
    return val;
}

static uint8_t readU8(const uint8_t*& ptr) {
    uint8_t val = *ptr;
    ptr++;
    return val;
}

void RTree::serialize(std::vector<uint8_t>& buffer) const {
    writeU32(buffer, dims_);
    writeU32(buffer, maxEntries_);
    writeU32(buffer, minEntries_);
    writeU8(buffer, static_cast<uint8_t>(metric_));
    writeU64(buffer, size_);
    writeU32(buffer, root_);

    uint32_t numNodes = static_cast<uint32_t>(nodes_.size());
    writeU32(buffer, numNodes);

    for (const auto& node : nodes_) {
        writeU8(buffer, node.isLeaf ? 1 : 0);
        writeU32(buffer, static_cast<uint32_t>(node.entries.size()));
        for (const auto& entry : node.entries) {
            for (uint32_t d = 0; d < dims_; d++) {
                writeF64(buffer, entry.bbox.minCoords[d]);
                writeF64(buffer, entry.bbox.maxCoords[d]);
            }
            writeU64(buffer, entry.data);
        }
    }
}

std::unique_ptr<RTree> RTree::deserialize(const uint8_t* data, size_t /*len*/) {
    const uint8_t* ptr = data;

    uint32_t dims = readU32(ptr);
    uint32_t maxEntries = readU32(ptr);
    uint32_t minEntries = readU32(ptr);
    auto metric = static_cast<DistanceMetric>(readU8(ptr));
    uint64_t size = readU64(ptr);
    uint32_t root = readU32(ptr);
    uint32_t numNodes = readU32(ptr);

    auto tree = std::make_unique<RTree>(dims, maxEntries, minEntries, metric);
    tree->nodes_.clear();
    tree->size_ = size;
    tree->root_ = root;

    for (uint32_t n = 0; n < numNodes; n++) {
        RTreeNode node;
        node.isLeaf = readU8(ptr) != 0;
        uint32_t numEntries = readU32(ptr);
        node.entries.reserve(numEntries);
        for (uint32_t e = 0; e < numEntries; e++) {
            RTreeEntry entry;
            entry.bbox.minCoords.resize(dims);
            entry.bbox.maxCoords.resize(dims);
            for (uint32_t d = 0; d < dims; d++) {
                entry.bbox.minCoords[d] = readF64(ptr);
                entry.bbox.maxCoords[d] = readF64(ptr);
            }
            entry.data = readU64(ptr);
            node.entries.push_back(std::move(entry));
        }
        tree->nodes_.push_back(std::move(node));
    }

    // Rebuild offsetToLeaf_ and parentMap_.
    for (uint32_t i = 0; i < tree->nodes_.size(); i++) {
        const auto& nd = tree->nodes_[i];
        if (nd.isLeaf) {
            for (const auto& e : nd.entries) {
                tree->offsetToLeaf_[e.data] = i;
            }
        } else {
            for (const auto& e : nd.entries) {
                tree->parentMap_[static_cast<uint32_t>(e.data)] = i;
            }
        }
    }

    return tree;
}

// ─── Helpers ───────────────────────────────────────────────────────────────────

void RTree::rebuildParentMap() {
    parentMap_.clear();
    for (uint32_t i = 0; i < nodes_.size(); i++) {
        if (!nodes_[i].isLeaf) {
            for (const auto& e : nodes_[i].entries) {
                parentMap_[static_cast<uint32_t>(e.data)] = i;
            }
        }
    }
}

// ─── getCoords ────────────────────────────────────────────────────────────────

bool RTree::getCoords(uint64_t nodeOffset, std::vector<double>& coords) const {
    auto it = offsetToLeaf_.find(nodeOffset);
    if (it == offsetToLeaf_.end()) {
        return false;
    }
    const auto& node = nodes_[it->second];
    for (const auto& entry : node.entries) {
        if (entry.data == nodeOffset) {
            coords.resize(dims_);
            for (uint32_t d = 0; d < dims_; d++) {
                coords[d] = entry.bbox.minCoords[d];
            }
            return true;
        }
    }
    return false;
}

// ─── searchOBB ────────────────────────────────────────────────────────────────

std::vector<std::pair<uint64_t, double>> RTree::searchOBB(const std::vector<double>& center,
    const std::vector<double>& halfExtents, const math::Quat& quat) const {
    if (dims_ != 3 || center.size() != 3 || halfExtents.size() != 3 || size_ == 0) {
        return {};
    }

    // Compute AABB that encloses the OBB by rotating the 8 corners.
    BoundingBox aabb(3);
    for (int sx = -1; sx <= 1; sx += 2) {
        for (int sy = -1; sy <= 1; sy += 2) {
            for (int sz = -1; sz <= 1; sz += 2) {
                double lx = sx * halfExtents[0];
                double ly = sy * halfExtents[1];
                double lz = sz * halfExtents[2];
                double wx, wy, wz;
                quat.rotate(lx, ly, lz, wx, wy, wz);
                wx += center[0];
                wy += center[1];
                wz += center[2];
                std::vector<double> corner = {wx, wy, wz};
                aabb.expand(BoundingBox::fromPoint(corner));
            }
        }
    }

    // Pre-filter with AABB.
    auto candidates = searchBBox(aabb);

    // Exact OBB test on each candidate.
    auto quatInv = quat.inverse();
    std::vector<std::pair<uint64_t, double>> results;
    for (uint64_t offset : candidates) {
        std::vector<double> coords;
        if (!getCoords(offset, coords)) continue;

        double dx = coords[0] - center[0];
        double dy = coords[1] - center[1];
        double dz = coords[2] - center[2];
        double lx, ly, lz;
        quatInv.rotate(dx, dy, dz, lx, ly, lz);

        if (std::fabs(lx) <= halfExtents[0] && std::fabs(ly) <= halfExtents[1] &&
            std::fabs(lz) <= halfExtents[2]) {
            double dist = std::sqrt(dx * dx + dy * dy + dz * dz);
            results.emplace_back(offset, dist);
        }
    }

    // Sort by distance.
    std::sort(results.begin(), results.end(),
        [](const auto& a, const auto& b) { return a.second < b.second; });
    return results;
}

// ─── searchFrustum ────────────────────────────────────────────────────────────

std::vector<std::pair<uint64_t, double>> RTree::searchFrustum(const double* planes,
    uint32_t numPlanes, const std::vector<double>& refPoint) const {
    if (dims_ != 3 || refPoint.size() != 3 || size_ == 0 || numPlanes == 0) {
        return {};
    }

    // Compute AABB from planes. We'll use a conservative large AABB
    // by finding the min/max extents from the plane constraints.
    // For a general convex hull, we scan all leaves — the AABB pre-filter
    // is an optimization for very large datasets.
    // Simple approach: scan all leaves and test each.
    std::vector<std::pair<uint64_t, double>> results;
    for (const auto& node : nodes_) {
        if (!node.isLeaf) continue;
        for (const auto& entry : node.entries) {
            double px = entry.bbox.minCoords[0];
            double py = entry.bbox.minCoords[1];
            double pz = entry.bbox.minCoords[2];
            if (math::insideConvex(px, py, pz, planes, numPlanes)) {
                double dx = px - refPoint[0];
                double dy = py - refPoint[1];
                double dz = pz - refPoint[2];
                double dist = std::sqrt(dx * dx + dy * dy + dz * dz);
                results.emplace_back(entry.data, dist);
            }
        }
    }

    std::sort(results.begin(), results.end(),
        [](const auto& a, const auto& b) { return a.second < b.second; });
    return results;
}

} // namespace geo_extension
} // namespace rag3db
