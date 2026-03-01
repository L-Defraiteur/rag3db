#pragma once

namespace rag3db {
namespace geo_extension {
namespace math {

struct Mat3 {
    double m[9]; // Row-major: [m00,m01,m02, m10,m11,m12, m20,m21,m22]

    static Mat3 identity() { return {{1, 0, 0, 0, 1, 0, 0, 0, 1}}; }

    void rotate(double px, double py, double pz, double& ox, double& oy,
        double& oz) const {
        ox = m[0] * px + m[1] * py + m[2] * pz;
        oy = m[3] * px + m[4] * py + m[5] * pz;
        oz = m[6] * px + m[7] * py + m[8] * pz;
    }

    Mat3 transpose() const {
        return {{m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]}};
    }

    Mat3 operator*(const Mat3& o) const {
        Mat3 r{};
        for (int i = 0; i < 3; i++) {
            for (int j = 0; j < 3; j++) {
                r.m[i * 3 + j] = 0;
                for (int k = 0; k < 3; k++) {
                    r.m[i * 3 + j] += m[i * 3 + k] * o.m[k * 3 + j];
                }
            }
        }
        return r;
    }
};

} // namespace math
} // namespace geo_extension
} // namespace rag3db
