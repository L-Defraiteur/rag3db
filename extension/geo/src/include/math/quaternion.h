#pragma once
#include <cmath>

namespace rag3db {
namespace geo_extension {
namespace math {

struct Quat {
    double w, x, y, z;

    static Quat identity() { return {1, 0, 0, 0}; }

    static Quat fromAxisAngle(double ax, double ay, double az, double angle) {
        double halfAngle = angle * 0.5;
        double s = std::sin(halfAngle);
        double len = std::sqrt(ax * ax + ay * ay + az * az);
        if (len < 1e-15) {
            return identity();
        }
        double invLen = 1.0 / len;
        return {std::cos(halfAngle), ax * invLen * s, ay * invLen * s, az * invLen * s};
    }

    Quat inverse() const { return {w, -x, -y, -z}; }

    Quat operator*(const Quat& o) const {
        return {w * o.w - x * o.x - y * o.y - z * o.z,
            w * o.x + x * o.w + y * o.z - z * o.y,
            w * o.y - x * o.z + y * o.w + z * o.x,
            w * o.z + x * o.y - y * o.x + z * o.w};
    }

    // Optimized rotation: p' = p + 2w*(q.xyz x p) + 2*(q.xyz x (q.xyz x p))
    void rotate(double px, double py, double pz, double& ox, double& oy,
        double& oz) const {
        double tx = 2.0 * (y * pz - z * py);
        double ty = 2.0 * (z * px - x * pz);
        double tz = 2.0 * (x * py - y * px);
        ox = px + w * tx + (y * tz - z * ty);
        oy = py + w * ty + (z * tx - x * tz);
        oz = pz + w * tz + (x * ty - y * tx);
    }

    void toMatrix3(double m[9]) const {
        double xx = x * x, yy = y * y, zz = z * z;
        double xy = x * y, xz = x * z, yz = y * z;
        double wx = w * x, wy = w * y, wz = w * z;
        m[0] = 1 - 2 * (yy + zz);
        m[1] = 2 * (xy - wz);
        m[2] = 2 * (xz + wy);
        m[3] = 2 * (xy + wz);
        m[4] = 1 - 2 * (xx + zz);
        m[5] = 2 * (yz - wx);
        m[6] = 2 * (xz - wy);
        m[7] = 2 * (yz + wx);
        m[8] = 1 - 2 * (xx + yy);
    }
};

} // namespace math
} // namespace geo_extension
} // namespace rag3db
