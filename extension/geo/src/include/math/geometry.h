#pragma once
#include "math/quaternion.h"

#include <cmath>
#include <cstdint>

namespace rag3db {
namespace geo_extension {
namespace math {

static constexpr double EARTH_RADIUS_M = 6'371'000.0;
static constexpr double DEG_TO_RAD = M_PI / 180.0;

inline double haversine(double lat1, double lon1, double lat2, double lon2) {
    double dlat = (lat2 - lat1) * DEG_TO_RAD;
    double dlon = (lon2 - lon1) * DEG_TO_RAD;
    double a = std::sin(dlat / 2) * std::sin(dlat / 2) +
               std::cos(lat1 * DEG_TO_RAD) * std::cos(lat2 * DEG_TO_RAD) *
                   std::sin(dlon / 2) * std::sin(dlon / 2);
    return 2.0 * EARTH_RADIUS_M * std::asin(std::sqrt(a));
}

inline double euclideanDist(const double* a, const double* b, uint32_t dims) {
    double sum = 0;
    for (uint32_t i = 0; i < dims; i++) {
        double d = a[i] - b[i];
        sum += d * d;
    }
    return std::sqrt(sum);
}

inline double euclideanDistSq(const double* a, const double* b, uint32_t dims) {
    double sum = 0;
    for (uint32_t i = 0; i < dims; i++) {
        double d = a[i] - b[i];
        sum += d * d;
    }
    return sum;
}

inline bool pointInPolygon(double px, double py, const double* polyX, const double* polyY,
    uint32_t n) {
    bool inside = false;
    for (uint32_t i = 0, j = n - 1; i < n; j = i++) {
        if (((polyY[i] > py) != (polyY[j] > py)) &&
            (px < (polyX[j] - polyX[i]) * (py - polyY[i]) / (polyY[j] - polyY[i]) + polyX[i])) {
            inside = !inside;
        }
    }
    return inside;
}

inline bool insidePlane(double px, double py, double pz, double a, double b, double c,
    double d) {
    return a * px + b * py + c * pz + d >= 0.0;
}

inline bool insideConvex(double px, double py, double pz, const double* planes,
    uint32_t numPlanes) {
    for (uint32_t i = 0; i < numPlanes; i++) {
        if (!insidePlane(px, py, pz, planes[i * 4], planes[i * 4 + 1], planes[i * 4 + 2],
                planes[i * 4 + 3])) {
            return false;
        }
    }
    return true;
}

// Build 6 frustum planes from camera parameters.
// Convention: Three.js compatible — Y-up, camera looks toward -Z local.
// Output: 24 doubles (6 planes x 4 coefficients [a,b,c,d] where ax+by+cz+d >= 0 = inside).
// Plane order: left, right, bottom, top, near, far.
inline void frustumFromCamera(double posX, double posY, double posZ, double qx, double qy,
    double qz, double qw, double fovH, double fovV, double nearDist, double farDist,
    double planesOut[24]) {
    // Reconstruct camera axes from quaternion (Three.js convention: xyzw).
    Quat q{qw, qx, qy, qz};

    // Camera forward = -Z local, right = +X local, up = +Y local.
    double fx, fy, fz; // forward (-Z)
    double rx, ry, rz; // right (+X)
    double ux, uy, uz; // up (+Y)
    q.rotate(0, 0, -1, fx, fy, fz);
    q.rotate(1, 0, 0, rx, ry, rz);
    q.rotate(0, 1, 0, ux, uy, uz);

    double tanH = std::tan(fovH * 0.5);
    double tanV = std::tan(fovV * 0.5);

    // Near plane: normal = forward, point = pos + forward*near
    // n · (p - point) >= 0  =>  n·p - n·point >= 0  =>  a,b,c = n, d = -n·point
    auto setPlane = [&](int idx, double nx, double ny, double nz, double px, double py,
                        double pz) {
        double len = std::sqrt(nx * nx + ny * ny + nz * nz);
        if (len > 1e-15) {
            nx /= len;
            ny /= len;
            nz /= len;
        }
        planesOut[idx * 4 + 0] = nx;
        planesOut[idx * 4 + 1] = ny;
        planesOut[idx * 4 + 2] = nz;
        planesOut[idx * 4 + 3] = -(nx * px + ny * py + nz * pz);
    };

    // Near center and far center.
    double ncx = posX + fx * nearDist, ncy = posY + fy * nearDist, ncz = posZ + fz * nearDist;
    double fcx = posX + fx * farDist, fcy = posY + fy * farDist, fcz = posZ + fz * farDist;

    // Left plane: normal = normalize(forward + right * tanH), rotated 90° inward.
    // The left plane's inward normal is: right + forward * tanH (then normalized).
    double lnx = rx + fx * tanH, lny = ry + fy * tanH, lnz = rz + fz * tanH;
    setPlane(0, lnx, lny, lnz, posX, posY, posZ);

    // Right plane: inward normal = -right + forward * tanH.
    double rnx = -rx + fx * tanH, rny = -ry + fy * tanH, rnz = -rz + fz * tanH;
    setPlane(1, rnx, rny, rnz, posX, posY, posZ);

    // Bottom plane: inward normal = up + forward * tanV.
    double bnx = ux + fx * tanV, bny = uy + fy * tanV, bnz = uz + fz * tanV;
    setPlane(2, bnx, bny, bnz, posX, posY, posZ);

    // Top plane: inward normal = -up + forward * tanV.
    double tnx = -ux + fx * tanV, tny = -uy + fy * tanV, tnz = -uz + fz * tanV;
    setPlane(3, tnx, tny, tnz, posX, posY, posZ);

    // Near plane: inward normal = forward, point on plane = near center.
    setPlane(4, fx, fy, fz, ncx, ncy, ncz);

    // Far plane: inward normal = -forward, point on plane = far center.
    setPlane(5, -fx, -fy, -fz, fcx, fcy, fcz);
}

} // namespace math
} // namespace geo_extension
} // namespace rag3db
