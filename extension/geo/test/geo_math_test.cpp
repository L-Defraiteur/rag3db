#include <gtest/gtest.h>

#include "math/quaternion.h"
#include "math/matrix3.h"
#include "math/geometry.h"

#include <cmath>

using namespace rag3db::geo_extension::math;

// ─── Quaternion Tests ─────────────────────────────────────────────────────────

TEST(QuatTest, Identity) {
    auto q = Quat::identity();
    double ox, oy, oz;
    q.rotate(1.0, 2.0, 3.0, ox, oy, oz);
    ASSERT_NEAR(ox, 1.0, 1e-10);
    ASSERT_NEAR(oy, 2.0, 1e-10);
    ASSERT_NEAR(oz, 3.0, 1e-10);
}

TEST(QuatTest, Rotate90Y) {
    // 90° around Y: (1,0,0) → (0,0,-1)
    auto q = Quat::fromAxisAngle(0, 1, 0, M_PI / 2);
    double ox, oy, oz;
    q.rotate(1.0, 0.0, 0.0, ox, oy, oz);
    ASSERT_NEAR(ox, 0.0, 1e-10);
    ASSERT_NEAR(oy, 0.0, 1e-10);
    ASSERT_NEAR(oz, -1.0, 1e-10);
}

TEST(QuatTest, Rotate90X) {
    // 90° around X: (0,1,0) → (0,0,1)
    auto q = Quat::fromAxisAngle(1, 0, 0, M_PI / 2);
    double ox, oy, oz;
    q.rotate(0.0, 1.0, 0.0, ox, oy, oz);
    ASSERT_NEAR(ox, 0.0, 1e-10);
    ASSERT_NEAR(oy, 0.0, 1e-10);
    ASSERT_NEAR(oz, 1.0, 1e-10);
}

TEST(QuatTest, Rotate90Z) {
    // 90° around Z: (1,0,0) → (0,1,0)
    auto q = Quat::fromAxisAngle(0, 0, 1, M_PI / 2);
    double ox, oy, oz;
    q.rotate(1.0, 0.0, 0.0, ox, oy, oz);
    ASSERT_NEAR(ox, 0.0, 1e-10);
    ASSERT_NEAR(oy, 1.0, 1e-10);
    ASSERT_NEAR(oz, 0.0, 1e-10);
}

TEST(QuatTest, Inverse) {
    auto q = Quat::fromAxisAngle(0, 1, 0, M_PI / 2);
    auto qi = q.inverse();
    // q * qi should be identity.
    auto id = q * qi;
    ASSERT_NEAR(id.w, 1.0, 1e-10);
    ASSERT_NEAR(id.x, 0.0, 1e-10);
    ASSERT_NEAR(id.y, 0.0, 1e-10);
    ASSERT_NEAR(id.z, 0.0, 1e-10);
}

TEST(QuatTest, Multiply) {
    // Two 90° rotations around Y = 180° around Y.
    auto q90 = Quat::fromAxisAngle(0, 1, 0, M_PI / 2);
    auto q180 = q90 * q90;
    double ox, oy, oz;
    q180.rotate(1.0, 0.0, 0.0, ox, oy, oz);
    ASSERT_NEAR(ox, -1.0, 1e-10);
    ASSERT_NEAR(oy, 0.0, 1e-10);
    ASSERT_NEAR(oz, 0.0, 1e-10);
}

TEST(QuatTest, FromAxisAngleZeroAxis) {
    auto q = Quat::fromAxisAngle(0, 0, 0, 1.0);
    ASSERT_NEAR(q.w, 1.0, 1e-10);
    ASSERT_NEAR(q.x, 0.0, 1e-10);
}

TEST(QuatTest, ToMatrix3) {
    // 90° around Y.
    auto q = Quat::fromAxisAngle(0, 1, 0, M_PI / 2);
    double m[9];
    q.toMatrix3(m);
    // Expected 3x3 for 90° Y rotation:
    // [ 0  0  1]
    // [ 0  1  0]
    // [-1  0  0]
    ASSERT_NEAR(m[0], 0.0, 1e-10);
    ASSERT_NEAR(m[2], 1.0, 1e-10);
    ASSERT_NEAR(m[4], 1.0, 1e-10);
    ASSERT_NEAR(m[6], -1.0, 1e-10);
}

TEST(QuatTest, RotateMatchesMatrix) {
    auto q = Quat::fromAxisAngle(1, 1, 0, 0.7);
    double m[9];
    q.toMatrix3(m);
    Mat3 mat{};
    for (int i = 0; i < 9; i++) mat.m[i] = m[i];

    double ox1, oy1, oz1, ox2, oy2, oz2;
    q.rotate(3.0, -1.0, 2.0, ox1, oy1, oz1);
    mat.rotate(3.0, -1.0, 2.0, ox2, oy2, oz2);
    ASSERT_NEAR(ox1, ox2, 1e-10);
    ASSERT_NEAR(oy1, oy2, 1e-10);
    ASSERT_NEAR(oz1, oz2, 1e-10);
}

// ─── Matrix3 Tests ────────────────────────────────────────────────────────────

TEST(Mat3Test, Identity) {
    auto m = Mat3::identity();
    double ox, oy, oz;
    m.rotate(3.0, 4.0, 5.0, ox, oy, oz);
    ASSERT_NEAR(ox, 3.0, 1e-10);
    ASSERT_NEAR(oy, 4.0, 1e-10);
    ASSERT_NEAR(oz, 5.0, 1e-10);
}

TEST(Mat3Test, Rotate90Y) {
    // 90° around Y, row-major:
    // [ 0  0  1]
    // [ 0  1  0]
    // [-1  0  0]
    Mat3 m{{0, 0, 1, 0, 1, 0, -1, 0, 0}};
    double ox, oy, oz;
    m.rotate(1.0, 0.0, 0.0, ox, oy, oz);
    ASSERT_NEAR(ox, 0.0, 1e-10);
    ASSERT_NEAR(oy, 0.0, 1e-10);
    ASSERT_NEAR(oz, -1.0, 1e-10);
}

TEST(Mat3Test, Transpose) {
    Mat3 m{{1, 2, 3, 4, 5, 6, 7, 8, 9}};
    auto t = m.transpose();
    ASSERT_DOUBLE_EQ(t.m[0], 1);
    ASSERT_DOUBLE_EQ(t.m[1], 4);
    ASSERT_DOUBLE_EQ(t.m[2], 7);
    ASSERT_DOUBLE_EQ(t.m[3], 2);
    ASSERT_DOUBLE_EQ(t.m[4], 5);
    ASSERT_DOUBLE_EQ(t.m[5], 8);
}

TEST(Mat3Test, TransposeIsInverseForRotation) {
    // For orthogonal rotation matrix, transpose = inverse.
    auto q = Quat::fromAxisAngle(0, 1, 0, 0.5);
    double m[9];
    q.toMatrix3(m);
    Mat3 mat{};
    for (int i = 0; i < 9; i++) mat.m[i] = m[i];
    auto mt = mat.transpose();
    auto id = mat * mt;
    ASSERT_NEAR(id.m[0], 1.0, 1e-10);
    ASSERT_NEAR(id.m[4], 1.0, 1e-10);
    ASSERT_NEAR(id.m[8], 1.0, 1e-10);
    ASSERT_NEAR(id.m[1], 0.0, 1e-10);
}

TEST(Mat3Test, Multiply) {
    auto a = Mat3::identity();
    Mat3 b{{2, 0, 0, 0, 3, 0, 0, 0, 4}};
    auto c = a * b;
    ASSERT_DOUBLE_EQ(c.m[0], 2.0);
    ASSERT_DOUBLE_EQ(c.m[4], 3.0);
    ASSERT_DOUBLE_EQ(c.m[8], 4.0);
}

// ─── Geometry Tests ───────────────────────────────────────────────────────────

TEST(GeometryTest, Haversine) {
    // Paris (48.8566, 2.3522) to Lyon (45.7640, 4.8357) ~ 392 km
    double dist = haversine(48.8566, 2.3522, 45.7640, 4.8357);
    ASSERT_GT(dist, 380000);
    ASSERT_LT(dist, 400000);
}

TEST(GeometryTest, HaversineSamePoint) {
    double dist = haversine(48.8566, 2.3522, 48.8566, 2.3522);
    ASSERT_DOUBLE_EQ(dist, 0.0);
}

TEST(GeometryTest, EuclideanDist) {
    double a[] = {0, 0, 0};
    double b[] = {3, 4, 0};
    ASSERT_DOUBLE_EQ(euclideanDist(a, b, 3), 5.0);
}

TEST(GeometryTest, EuclideanDist4D) {
    double a[] = {1, 2, 3, 4};
    double b[] = {1, 2, 3, 4};
    ASSERT_DOUBLE_EQ(euclideanDist(a, b, 4), 0.0);
}

TEST(GeometryTest, PointInPolygonSquare) {
    double xs[] = {0, 10, 10, 0};
    double ys[] = {0, 0, 10, 10};
    ASSERT_TRUE(pointInPolygon(5, 5, xs, ys, 4));
    ASSERT_FALSE(pointInPolygon(15, 5, xs, ys, 4));
    ASSERT_FALSE(pointInPolygon(-1, 5, xs, ys, 4));
}

TEST(GeometryTest, PointInPolygonTriangle) {
    double xs[] = {0, 10, 5};
    double ys[] = {0, 0, 10};
    ASSERT_TRUE(pointInPolygon(5, 3, xs, ys, 3));
    ASSERT_FALSE(pointInPolygon(0, 10, xs, ys, 3));
}

TEST(GeometryTest, InsidePlane) {
    // Plane x >= 0 : a=1, b=0, c=0, d=0.
    ASSERT_TRUE(insidePlane(1, 0, 0, 1, 0, 0, 0));
    ASSERT_TRUE(insidePlane(0, 0, 0, 1, 0, 0, 0)); // On the plane.
    ASSERT_FALSE(insidePlane(-1, 0, 0, 1, 0, 0, 0));
}

TEST(GeometryTest, InsideConvexCube) {
    // Unit cube [0,1]^3: 6 planes.
    double planes[] = {
        1, 0, 0, 0,    // x >= 0
        -1, 0, 0, 1,   // x <= 1
        0, 1, 0, 0,    // y >= 0
        0, -1, 0, 1,   // y <= 1
        0, 0, 1, 0,    // z >= 0
        0, 0, -1, 1,   // z <= 1
    };
    ASSERT_TRUE(insideConvex(0.5, 0.5, 0.5, planes, 6));
    ASSERT_FALSE(insideConvex(1.5, 0.5, 0.5, planes, 6));
    ASSERT_FALSE(insideConvex(0.5, -0.1, 0.5, planes, 6));
}

TEST(GeometryTest, FrustumFromCameraIdentity) {
    // Identity camera at origin, looking toward -Z.
    double planes[24];
    frustumFromCamera(0, 0, 0, 0, 0, 0, 1, M_PI / 2, M_PI / 2, 1.0, 100.0, planes);

    // Point at (0, 0, -50) should be inside the frustum.
    ASSERT_TRUE(insideConvex(0, 0, -50, planes, 6));

    // Point behind the camera (0, 0, 50) should be outside.
    ASSERT_FALSE(insideConvex(0, 0, 50, planes, 6));

    // Point too far (0, 0, -200) should be outside.
    ASSERT_FALSE(insideConvex(0, 0, -200, planes, 6));

    // Point too close (0, 0, -0.1) should be outside.
    ASSERT_FALSE(insideConvex(0, 0, -0.1, planes, 6));
}

TEST(GeometryTest, FrustumFromCameraRotated) {
    // Camera at origin, rotated 90° around Y → looking toward +X.
    auto q = Quat::fromAxisAngle(0, 1, 0, -M_PI / 2);
    double planes[24];
    frustumFromCamera(0, 0, 0, q.x, q.y, q.z, q.w, M_PI / 2, M_PI / 2, 1.0, 100.0, planes);

    // Point at (50, 0, 0) should be inside (camera looks +X).
    ASSERT_TRUE(insideConvex(50, 0, 0, planes, 6));

    // Point at (-50, 0, 0) should be outside (behind camera).
    ASSERT_FALSE(insideConvex(-50, 0, 0, planes, 6));
}
