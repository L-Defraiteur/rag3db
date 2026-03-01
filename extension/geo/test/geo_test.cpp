#include <gtest/gtest.h>

#include "index/rtree.h"

#include <cmath>

using namespace rag3db::geo_extension;

// ─── BoundingBox Tests ─────────────────────────────────────────────────────────

TEST(BoundingBoxTest, FromPoint) {
    auto bb = BoundingBox::fromPoint({1.0, 2.0});
    ASSERT_EQ(bb.dims(), 2);
    ASSERT_DOUBLE_EQ(bb.minCoords[0], 1.0);
    ASSERT_DOUBLE_EQ(bb.maxCoords[1], 2.0);
}

TEST(BoundingBoxTest, Expand) {
    auto bb = BoundingBox::fromPoint({1.0, 2.0});
    bb.expand(BoundingBox::fromPoint({3.0, 0.0}));
    ASSERT_DOUBLE_EQ(bb.minCoords[0], 1.0);
    ASSERT_DOUBLE_EQ(bb.maxCoords[0], 3.0);
    ASSERT_DOUBLE_EQ(bb.minCoords[1], 0.0);
    ASSERT_DOUBLE_EQ(bb.maxCoords[1], 2.0);
}

TEST(BoundingBoxTest, Area) {
    BoundingBox bb;
    bb.minCoords = {0.0, 0.0};
    bb.maxCoords = {3.0, 4.0};
    ASSERT_DOUBLE_EQ(bb.area(), 12.0);
}

TEST(BoundingBoxTest, Intersects) {
    BoundingBox a;
    a.minCoords = {0.0, 0.0};
    a.maxCoords = {2.0, 2.0};
    BoundingBox b;
    b.minCoords = {1.0, 1.0};
    b.maxCoords = {3.0, 3.0};
    ASSERT_TRUE(a.intersects(b));

    BoundingBox c;
    c.minCoords = {5.0, 5.0};
    c.maxCoords = {6.0, 6.0};
    ASSERT_FALSE(a.intersects(c));
}

TEST(BoundingBoxTest, ContainsPoint) {
    BoundingBox bb;
    bb.minCoords = {0.0, 0.0};
    bb.maxCoords = {10.0, 10.0};
    ASSERT_TRUE(bb.containsPoint({5.0, 5.0}));
    ASSERT_FALSE(bb.containsPoint({-1.0, 5.0}));
}

TEST(BoundingBoxTest, MinDistToPoint) {
    BoundingBox bb;
    bb.minCoords = {2.0, 2.0};
    bb.maxCoords = {4.0, 4.0};
    ASSERT_DOUBLE_EQ(bb.minDistToPoint({3.0, 3.0}), 0.0); // Inside
    ASSERT_NEAR(bb.minDistToPoint({0.0, 3.0}), 2.0, 1e-10); // Left
}

// ─── RTree Basic Tests ─────────────────────────────────────────────────────────

TEST(RTreeTest, InsertAndSize) {
    RTree tree(2);
    tree.insert(0, {1.0, 2.0});
    tree.insert(1, {3.0, 4.0});
    tree.insert(2, {5.0, 6.0});
    ASSERT_EQ(tree.size(), 3);
}

TEST(RTreeTest, RemoveAndSize) {
    RTree tree(2);
    tree.insert(0, {1.0, 2.0});
    tree.insert(1, {3.0, 4.0});
    ASSERT_TRUE(tree.remove(0));
    ASSERT_EQ(tree.size(), 1);
    ASSERT_FALSE(tree.remove(99)); // Non-existent
}

TEST(RTreeTest, KNN) {
    RTree tree(2);
    tree.insert(0, {0.0, 0.0});
    tree.insert(1, {1.0, 0.0});
    tree.insert(2, {10.0, 0.0});
    tree.insert(3, {100.0, 0.0});

    auto results = tree.knn({0.5, 0.0}, 2);
    ASSERT_EQ(results.size(), 2);
    // Nearest should be 0 and 1.
    ASSERT_TRUE(results[0].first == 0 || results[0].first == 1);
    ASSERT_TRUE(results[1].first == 0 || results[1].first == 1);
    ASSERT_LE(results[0].second, results[1].second);
}

TEST(RTreeTest, SearchBBox) {
    RTree tree(2);
    tree.insert(0, {1.0, 1.0});
    tree.insert(1, {5.0, 5.0});
    tree.insert(2, {10.0, 10.0});

    BoundingBox bbox;
    bbox.minCoords = {0.0, 0.0};
    bbox.maxCoords = {6.0, 6.0};
    auto results = tree.searchBBox(bbox);
    ASSERT_EQ(results.size(), 2);
}

TEST(RTreeTest, SearchRadius) {
    RTree tree(2);
    tree.insert(0, {0.0, 0.0});
    tree.insert(1, {1.0, 0.0});
    tree.insert(2, {5.0, 0.0});

    auto results = tree.searchRadius({0.0, 0.0}, 2.0);
    ASSERT_EQ(results.size(), 2); // 0 and 1
}

TEST(RTreeTest, Update) {
    RTree tree(2);
    tree.insert(0, {1.0, 1.0});
    tree.insert(1, {10.0, 10.0});

    // Move node 0 far away.
    tree.update(0, {100.0, 100.0});

    auto results = tree.knn({0.0, 0.0}, 1);
    ASSERT_EQ(results.size(), 1);
    ASSERT_EQ(results[0].first, 1); // Node 1 is now nearest.
}

// ─── 3D Tests ──────────────────────────────────────────────────────────────────

TEST(RTreeTest, ThreeDimensional) {
    RTree tree(3);
    tree.insert(0, {0.0, 0.0, 0.0});
    tree.insert(1, {1.0, 1.0, 1.0});
    tree.insert(2, {10.0, 10.0, 10.0});

    auto results = tree.knn({0.0, 0.0, 0.0}, 2);
    ASSERT_EQ(results.size(), 2);
    ASSERT_EQ(results[0].first, 0);
    ASSERT_EQ(results[1].first, 1);
}

// ─── Haversine Tests ───────────────────────────────────────────────────────────

TEST(RTreeTest, HaversineMetric) {
    RTree tree(2, 16, 4, DistanceMetric::HAVERSINE);
    // Paris
    tree.insert(0, {48.8566, 2.3522});
    // Lyon
    tree.insert(1, {45.7640, 4.8357});
    // New York
    tree.insert(2, {40.7128, -74.0060});

    auto results = tree.knn({48.0, 2.0}, 2);
    ASSERT_EQ(results.size(), 2);
    // Paris should be closest, then Lyon.
    ASSERT_EQ(results[0].first, 0);
    ASSERT_EQ(results[1].first, 1);
    // Distance to Paris should be ~100km, to Lyon ~300km.
    ASSERT_GT(results[0].second, 50000);
    ASSERT_LT(results[0].second, 200000);
}

// ─── Serialization Tests ───────────────────────────────────────────────────────

TEST(RTreeTest, SerializeDeserialize) {
    RTree tree(2, 4, 2);
    for (uint64_t i = 0; i < 20; i++) {
        tree.insert(i, {static_cast<double>(i), static_cast<double>(i * 2)});
    }

    std::vector<uint8_t> buffer;
    tree.serialize(buffer);
    ASSERT_GT(buffer.size(), 0);

    auto tree2 = RTree::deserialize(buffer.data(), buffer.size());
    ASSERT_EQ(tree2->size(), 20);
    ASSERT_EQ(tree2->dimensions(), 2);

    // Verify KNN still works.
    auto results = tree2->knn({5.0, 10.0}, 1);
    ASSERT_EQ(results.size(), 1);
    ASSERT_EQ(results[0].first, 5); // Point (5, 10) is exact match.
}

// ─── Stress test: many inserts + splits ────────────────────────────────────────

TEST(RTreeTest, ManyInserts) {
    RTree tree(2, 8, 3);
    for (uint64_t i = 0; i < 1000; i++) {
        double x = static_cast<double>(i % 100);
        double y = static_cast<double>(i / 100);
        tree.insert(i, {x, y});
    }
    ASSERT_EQ(tree.size(), 1000);

    // KNN should return correct results.
    auto results = tree.knn({50.0, 5.0}, 5);
    ASSERT_EQ(results.size(), 5);

    // All nearest should be close to (50, 5).
    for (auto& [offset, dist] : results) {
        ASSERT_LT(dist, 3.0);
    }
}

TEST(RTreeTest, InsertDeleteMany) {
    RTree tree(2, 8, 3);
    for (uint64_t i = 0; i < 100; i++) {
        tree.insert(i, {static_cast<double>(i), 0.0});
    }
    ASSERT_EQ(tree.size(), 100);

    // Delete half.
    for (uint64_t i = 0; i < 50; i++) {
        ASSERT_TRUE(tree.remove(i));
    }
    ASSERT_EQ(tree.size(), 50);

    // KNN should only find remaining nodes.
    auto results = tree.knn({0.0, 0.0}, 1);
    ASSERT_EQ(results.size(), 1);
    ASSERT_GE(results[0].first, 50); // Only nodes 50-99 remain.
}
