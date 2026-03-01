# rag3db Geo Extension — Complete Spatial Toolkit

## 1. Vision

Aucune base de données graphe (ni relationnelle grand public) ne propose nativement :
quaternions, oriented bounding boxes, frustum queries, polygon-in-3D, le tout avec un
index spatial N-dimensionnel persisté. rag3db le fait.

**Positionnement :**

| Capacité | PostGIS | Qdrant | MongoDB | Neo4j | **rag3db geo** |
|---|---|---|---|---|---|
| 2D Geo (haversine, radius, bbox) | oui | oui | oui | basique | **oui** |
| Polygon 2D (ray casting) | oui | oui | oui | non | **oui** |
| 3D AABB | basique | non | non | non | **oui (N-dim)** |
| 3D Sphere | non | non | non | non | **oui** |
| Oriented Bounding Box | non | non | non | non | **oui** |
| Quaternion/Matrix transforms | non | non | non | non | **oui** |
| Polygon 2D en 3D (ref frame) | non | non | non | non | **oui** |
| Frustum query | non | non | non | non | **oui** |
| Convex hull containment | non | non | non | non | **oui** |
| Spatial R-tree index (N-dim) | oui (GiST) | non | 2D only | non | **oui** |

---

## 2. Fonctions Scalaires

### 2.1 Distance

#### `geo_distance(lat1, lon1, lat2, lon2) → DOUBLE`
Distance haversine en mètres entre deux points géographiques (lat/lon en degrés).
```cypher
RETURN geo_distance(48.8566, 2.3522, 45.7640, 4.8357)
-- → ~392211.0 (mètres)
```

#### `geo_distance_euclidean(p1, p2) → DOUBLE`
Distance euclidienne N-dimensionnelle.
```cypher
RETURN geo_distance_euclidean([1.0, 2.0, 3.0], [4.0, 6.0, 3.0])
-- → 5.0
```

---

### 2.2 Transformations — Quaternions

Convention : quaternion = `[w, x, y, z]` (scalar-first).

#### `geo_quat_rotate(quat, point) → LIST(DOUBLE)`
Rotation d'un point 3D par un quaternion.
```cypher
-- Rotation 90° autour de Y : quat = [cos(45°), 0, sin(45°), 0]
RETURN geo_quat_rotate([0.7071, 0.0, 0.7071, 0.0], [1.0, 0.0, 0.0])
-- → [0.0, 0.0, -1.0]  (X devient -Z)
```

**Maths :** `p' = q * p * q⁻¹` (sandwich product)
```
t = 2 * cross(q.xyz, p)
p' = p + w * t + cross(q.xyz, t)
```

#### `geo_quat_inverse(quat) → LIST(DOUBLE)`
Quaternion conjugué (= inverse pour quaternion unitaire).
```cypher
RETURN geo_quat_inverse([0.7071, 0.0, 0.7071, 0.0])
-- → [0.7071, 0.0, -0.7071, 0.0]
```

#### `geo_quat_multiply(q1, q2) → LIST(DOUBLE)`
Composition de rotations : applique q2 puis q1.
```cypher
RETURN geo_quat_multiply(rot_y_90, rot_x_90)
-- → quaternion combiné
```

**Maths :**
```
w = q1.w*q2.w - q1.x*q2.x - q1.y*q2.y - q1.z*q2.z
x = q1.w*q2.x + q1.x*q2.w + q1.y*q2.z - q1.z*q2.y
y = q1.w*q2.y - q1.x*q2.z + q1.y*q2.w + q1.z*q2.x
z = q1.w*q2.z + q1.x*q2.y - q1.y*q2.x + q1.z*q2.w
```

#### `geo_quat_from_axis_angle(axis, angle_rad) → LIST(DOUBLE)`
Crée un quaternion à partir d'un axe et d'un angle (radians).
```cypher
RETURN geo_quat_from_axis_angle([0.0, 1.0, 0.0], 1.5708)
-- → [0.7071, 0.0, 0.7071, 0.0]  (90° autour Y)
```

#### `geo_quat_to_matrix(quat) → LIST(DOUBLE)`
Convertit un quaternion en matrice de rotation 3×3 (row-major, 9 éléments).
```cypher
RETURN geo_quat_to_matrix([0.7071, 0.0, 0.7071, 0.0])
-- → [0, 0, 1, 0, 1, 0, -1, 0, 0]  (3x3 row-major)
```

---

### 2.3 Transformations — Matrices

Matrices 3×3 stockées en row-major : `[m00, m01, m02, m10, m11, m12, m20, m21, m22]`.

#### `geo_matrix_rotate(matrix, point) → LIST(DOUBLE)`
Rotation d'un point 3D par une matrice 3×3.
```cypher
RETURN geo_matrix_rotate([0,0,1, 0,1,0, -1,0,0], [1.0, 0.0, 0.0])
-- → [0.0, 0.0, -1.0]
```

#### `geo_matrix_multiply(m1, m2) → LIST(DOUBLE)`
Produit de deux matrices 3×3.

#### `geo_matrix_transpose(matrix) → LIST(DOUBLE)`
Transposée d'une matrice 3×3 (= inverse pour matrices de rotation orthogonales).

---

### 2.4 Tests de Contenance — 2D

#### `geo_within_bbox(lat, lon, min_lat, min_lon, max_lat, max_lon) → BOOL`
Point dans une bounding box 2D (axes alignés). Pour geo lat/lon.
```cypher
RETURN geo_within_bbox(48.856, 2.352, 48.0, 2.0, 49.0, 3.0)
-- → true
```

#### `geo_within_bbox_nd(point, min_corner, max_corner) → BOOL`
Point dans une AABB N-dimensionnelle. Généralisation LIST(DOUBLE).
```cypher
RETURN geo_within_bbox_nd([5.0, 3.0, 1.0], [0.0, 0.0, 0.0], [10.0, 10.0, 10.0])
-- → true
```

#### `geo_within_polygon(point_lat, point_lon, polygon_lats, polygon_lons) → BOOL`
Point dans un polygone 2D. Ray casting algorithm.
```cypher
RETURN geo_within_polygon(48.856, 2.352,
    [48.8, 48.9, 48.9, 48.8],
    [2.3, 2.3, 2.4, 2.4])
-- → true
```

#### `geo_within_circle(point_lat, point_lon, center_lat, center_lon, radius_m) → BOOL`
Point dans un cercle (distance haversine ≤ radius en mètres).
```cypher
RETURN geo_within_circle(48.856, 2.352, 48.860, 2.350, 1000.0)
-- → true (< 1km)
```

---

### 2.5 Tests de Contenance — 3D / N-D

#### `geo_within_sphere(point, center, radius) → BOOL`
Point dans une sphère N-dimensionnelle (distance euclidienne).
```cypher
RETURN geo_within_sphere([5.0, 3.0, 1.0], [5.0, 3.0, 0.0], 2.0)
-- → true (distance = 1.0 < 2.0)
```

#### `geo_within_obb(point, center, half_extents, quaternion) → BOOL`
Point dans une Oriented Bounding Box 3D.

**Algorithme :**
1. `p_local = quat_inverse.rotate(point - center)`
2. `|p_local[i]| ≤ half_extents[i]` pour chaque axe

```cypher
-- Box 4×2×1 centrée en (10, 5, 2), tournée 90° autour Y
RETURN geo_within_obb(
    [10.0, 5.5, 2.0],              -- point
    [10.0, 5.0, 2.0],              -- center
    [2.0, 1.0, 0.5],               -- half-extents (4m × 2m × 1m)
    [0.7071, 0.0, 0.7071, 0.0]     -- quaternion
)
-- → true
```

#### `geo_within_obb_matrix(point, center, half_extents, matrix) → BOOL`
Même chose mais avec une matrice de rotation 3×3 au lieu d'un quaternion.
```cypher
RETURN geo_within_obb_matrix(
    [10.0, 5.5, 2.0],
    [10.0, 5.0, 2.0],
    [2.0, 1.0, 0.5],
    [0,0,1, 0,1,0, -1,0,0]         -- matrice 3x3 row-major
)
```

---

### 2.6 Polygon 2D dans un Espace 3D

Le cas d'usage : "est-ce que ce point 3D est dans cette zone 2D positionnée et orientée
dans l'espace 3D ?" — utile pour des zones au sol, des murs, des plans de coupe.

#### `geo_within_polygon_3d(point, polygon_xs, polygon_ys, position, quaternion) → BOOL`

**Paramètres :**
- `point` : LIST(DOUBLE) — point 3D à tester
- `polygon_xs` : LIST(DOUBLE) — coordonnées X des sommets du polygone (dans son repère local 2D)
- `polygon_ys` : LIST(DOUBLE) — coordonnées Y des sommets du polygone (dans son repère local 2D)
- `position` : LIST(DOUBLE) — position de l'origine du polygone dans l'espace 3D
- `quaternion` : LIST(DOUBLE) — orientation du plan du polygone (quat [w,x,y,z])

**Algorithme :**
1. `p_relative = point - position`
2. `p_local = quat_inverse.rotate(p_relative)`
3. Projeter : ignorer `p_local.z` (ou vérifier `|p_local.z| < tolerance`)
4. Ray casting sur `(p_local.x, p_local.y)` vs le polygone 2D

**Paramètre optionnel :** `thickness := 0.1` — épaisseur du plan. Si fourni,
vérifie `|p_local.z| ≤ thickness/2`. Si non fourni, seule la projection compte
(pas de vérification de distance au plan).

```cypher
-- Zone au sol (polygone dans le plan XY, positionné en (100, 200, 0))
RETURN geo_within_polygon_3d(
    [105.0, 205.0, 0.5],                   -- point 3D (légèrement au-dessus du sol)
    [0.0, 10.0, 10.0, 0.0],                -- polygon X vertices (local)
    [0.0, 0.0, 10.0, 10.0],                -- polygon Y vertices (local)
    [100.0, 200.0, 0.0],                   -- position dans le monde
    [1.0, 0.0, 0.0, 0.0],                  -- quaternion identité (plan horizontal)
    thickness := 1.0                        -- tolérance ±0.5m en Z
)
-- → true (point est dans la zone, à 0.5m du plan, dans la tolérance)
```

#### `geo_within_polygon_3d_matrix(point, polygon_xs, polygon_ys, position, matrix) → BOOL`
Variante avec matrice 3×3 au lieu de quaternion.
```cypher
RETURN geo_within_polygon_3d_matrix(
    [105.0, 205.0, 0.5],
    [0.0, 10.0, 10.0, 0.0],
    [0.0, 0.0, 10.0, 10.0],
    [100.0, 200.0, 0.0],
    [1,0,0, 0,1,0, 0,0,1],                 -- matrice identité
    thickness := 1.0
)
```

---

### 2.7 Frustum & Convex Hull

#### `geo_within_frustum(point, planes) → BOOL`
Point dans un frustum (pyramide tronquée) défini par 6 plans.
Chaque plan = `[a, b, c, d]` tel que `ax + by + cz + d ≥ 0` = côté intérieur.

**Use cases :** champ de vision caméra, radar, lidar, éclairage.

```cypher
-- Frustum simple (boîte, pour l'exemple)
RETURN geo_within_frustum(
    [5.0, 5.0, 5.0],
    [
        [1, 0, 0, 0],      -- x ≥ 0      (plan gauche)
        [-1, 0, 0, 10],     -- x ≤ 10     (plan droit)
        [0, 1, 0, 0],       -- y ≥ 0      (plan bas)
        [0, -1, 0, 10],     -- y ≤ 10     (plan haut)
        [0, 0, 1, 0],       -- z ≥ 0      (plan proche)
        [0, 0, -1, 10]      -- z ≤ 10     (plan loin)
    ]
)
-- → true
```

**Construction d'un vrai frustum caméra :**
```cypher
-- Frustum depuis une caméra (position, orientation, fov, near, far)
-- L'utilisateur pré-calcule les 6 plans ou on fournit un helper :
RETURN geo_frustum_from_camera(
    [0, 0, 0],              -- camera position
    [1, 0, 0, 0],           -- camera quaternion (orientation)
    1.0472,                  -- fov horizontal (60° en radians)
    0.7854,                  -- fov vertical (45° en radians)
    0.1,                     -- near plane
    100.0                    -- far plane
) -- → LIST(LIST(DOUBLE)) : 6 plans
```

#### `geo_within_convex(point, planes) → BOOL`
Généralisation du frustum : point dans un polyèdre convexe défini par N plans.
Même convention que frustum (`ax + by + cz + d ≥ 0` = intérieur).
```cypher
-- Tétraèdre, octaèdre, ou tout polyèdre convexe
RETURN geo_within_convex([1.0, 1.0, 1.0], [
    [1, 0, 0, 0],
    [0, 1, 0, 0],
    [0, 0, 1, 0],
    [-1, -1, -1, 2.5]
])
```

---

### 2.8 Helper : Frustum Camera

#### `geo_frustum_from_camera(position, quaternion, fov_h, fov_v, near, far) → LIST(LIST(DOUBLE))`

Construit les 6 plans d'un frustum de caméra. Retourne une liste de 6 plans `[a,b,c,d]`.

**Convention caméra :** la caméra regarde vers -Z local, X local = droite, Y local = haut.

**Algorithme :**
1. Calculer les directions forward, right, up depuis le quaternion
2. Construire les plans near, far, left, right, top, bottom
3. Chaque plan normalisé avec la normale pointant vers l'intérieur du frustum

---

## 3. Index Spatial R-tree

### 3.1 `CREATE_SPATIAL_INDEX`

```cypher
-- 2D géographique
CALL CREATE_SPATIAL_INDEX('places', 'geo_idx', ['lat', 'lon'],
    metric := 'haversine')

-- 3D euclidien
CALL CREATE_SPATIAL_INDEX('objects', 'pos_idx', ['x', 'y', 'z'])

-- 2D euclidien
CALL CREATE_SPATIAL_INDEX('floor_tiles', 'tile_idx', ['pos_x', 'pos_y'])
```

**Paramètres :**
- Table name (STRING)
- Index name (STRING)
- Column names (LIST(STRING)) — doivent être DOUBLE/FLOAT, ≥ 2 colonnes
- `metric` optionnel : `'euclidean'` (défaut) ou `'haversine'` (2D lat/lon uniquement)

**Comportement :**
1. Valide la table et les colonnes
2. Scanne tous les nœuds existants → `rtree.insert(offset, coords)`
3. Enregistre dans le catalogue + crée CRUD hooks pour mutations futures
4. Persiste l'index dans `<db_dir>/spatial_indexes/<table>/<index_name>.rtree`

### 3.2 `QUERY_SPATIAL_INDEX`

#### Mode KNN (défaut)
```cypher
CALL QUERY_SPATIAL_INDEX('places', 'geo_idx', [48.856, 2.352], 10)
RETURN node_id, distance
```

#### Mode Radius
```cypher
CALL QUERY_SPATIAL_INDEX('places', 'geo_idx', [48.856, 2.352], 100,
    radius := 5000.0)
RETURN node_id, distance
```

#### Mode Bounding Box
```cypher
CALL QUERY_SPATIAL_INDEX('places', 'geo_idx', [48.0, 2.0], 100,
    bbox_min := [48.0, 2.0], bbox_max := [49.0, 3.0])
RETURN node_id, distance
```

#### Mode OBB (3D)
```cypher
CALL QUERY_SPATIAL_INDEX('objects', 'pos_idx', [0,0,0], 50,
    obb_center := [10.0, 5.0, 2.0],
    obb_half_extents := [2.0, 1.0, 0.5],
    obb_quaternion := [0.7071, 0.0, 0.7071, 0.0])
RETURN node_id, distance
```
**Algorithme :** AABB pré-filtre (AABB englobant l'OBB) → test exact OBB → tri par distance.

#### Mode Frustum (3D)
```cypher
CALL QUERY_SPATIAL_INDEX('objects', 'pos_idx', [0,0,0], 100,
    frustum_planes := [[1,0,0,0], [-1,0,0,10], [0,1,0,0], [0,-1,0,10], [0,0,1,0], [0,0,-1,10]])
RETURN node_id, distance
```
**Algorithme :** AABB pré-filtre (AABB englobant le frustum) → test exact 6 plans → tri par distance.

### 3.3 `DROP_SPATIAL_INDEX`

```cypher
CALL DROP_SPATIAL_INDEX('places', 'geo_idx')
```

---

## 4. Architecture Interne

### 4.1 Arborescence

```
extension/geo/
├── CMakeLists.txt
├── SPATIAL_EXTENSION_DESIGN.md          ← ce document
├── src/
│   ├── include/
│   │   ├── main/geo_extension.h
│   │   ├── function/
│   │   │   ├── geo_distance.h
│   │   │   ├── geo_distance_euclidean.h
│   │   │   ├── geo_within_bbox.h
│   │   │   ├── geo_within_bbox_nd.h
│   │   │   ├── geo_within_polygon.h
│   │   │   ├── geo_within_circle.h
│   │   │   ├── geo_within_sphere.h
│   │   │   ├── geo_within_obb.h
│   │   │   ├── geo_within_polygon_3d.h
│   │   │   ├── geo_within_frustum.h
│   │   │   ├── geo_within_convex.h
│   │   │   ├── geo_frustum_from_camera.h
│   │   │   ├── geo_quat_rotate.h
│   │   │   ├── geo_quat_inverse.h
│   │   │   ├── geo_quat_multiply.h
│   │   │   ├── geo_quat_from_axis_angle.h
│   │   │   ├── geo_quat_to_matrix.h
│   │   │   ├── geo_matrix_rotate.h
│   │   │   ├── geo_matrix_multiply.h
│   │   │   ├── geo_matrix_transpose.h
│   │   │   ├── create_spatial_index.h
│   │   │   ├── query_spatial_index.h
│   │   │   └── drop_spatial_index.h
│   │   ├── math/
│   │   │   ├── quaternion.h             ← quaternion math pure (inline)
│   │   │   ├── matrix3.h               ← matrice 3x3 math pure (inline)
│   │   │   └── geometry.h              ← ray casting, plane tests (inline)
│   │   ├── index/
│   │   │   ├── rtree.h
│   │   │   └── rtree_index.h
│   │   └── catalog/
│   │       └── spatial_index_catalog_entry.h
│   ├── function/
│   │   ├── CMakeLists.txt
│   │   ├── geo_distance.cpp
│   │   ├── geo_distance_euclidean.cpp
│   │   ├── geo_within_bbox.cpp
│   │   ├── geo_within_bbox_nd.cpp
│   │   ├── geo_within_polygon.cpp
│   │   ├── geo_within_circle.cpp
│   │   ├── geo_within_sphere.cpp
│   │   ├── geo_within_obb.cpp
│   │   ├── geo_within_polygon_3d.cpp
│   │   ├── geo_within_frustum.cpp
│   │   ├── geo_within_convex.cpp
│   │   ├── geo_frustum_from_camera.cpp
│   │   ├── geo_quat_rotate.cpp
│   │   ├── geo_quat_inverse.cpp
│   │   ├── geo_quat_multiply.cpp
│   │   ├── geo_quat_from_axis_angle.cpp
│   │   ├── geo_quat_to_matrix.cpp
│   │   ├── geo_matrix_rotate.cpp
│   │   ├── geo_matrix_multiply.cpp
│   │   └── geo_matrix_transpose.cpp
│   ├── index/
│   │   ├── CMakeLists.txt
│   │   ├── rtree.cpp
│   │   └── rtree_index.cpp
│   ├── catalog/
│   │   ├── CMakeLists.txt
│   │   └── spatial_index_catalog_entry.cpp
│   └── main/
│       ├── CMakeLists.txt
│       └── geo_extension.cpp
└── test/
    ├── CMakeLists.txt
    ├── geo_test.cpp                     ← tests R-tree (existant)
    ├── geo_math_test.cpp                ← tests quaternion, matrice, geometry
    └── geo_functions_test.cpp           ← tests E2E scalar functions (avec DB)
```

### 4.2 Couche Math (headers-only)

Les maths pures sont dans `src/include/math/` en header-only. Zéro dépendance rag3db.
Testables isolément. Utilisées à la fois par les scalar functions et par le R-tree.

#### `quaternion.h`
```cpp
namespace rag3db::geo_extension::math {

struct Quat {
    double w, x, y, z;

    static Quat identity() { return {1, 0, 0, 0}; }
    static Quat fromAxisAngle(double ax, double ay, double az, double angle);
    Quat inverse() const { return {w, -x, -y, -z}; }  // Unit quat only
    Quat operator*(const Quat& o) const;                // Hamilton product
    void rotate(double px, double py, double pz,
                double& ox, double& oy, double& oz) const;
    void toMatrix3(double m[9]) const;                   // Row-major
};

} // namespace
```

#### `matrix3.h`
```cpp
namespace rag3db::geo_extension::math {

struct Mat3 {
    double m[9];  // Row-major: [m00, m01, m02, m10, m11, m12, m20, m21, m22]

    static Mat3 identity();
    void rotate(double px, double py, double pz,
                double& ox, double& oy, double& oz) const;
    Mat3 transpose() const;
    Mat3 operator*(const Mat3& o) const;
};

} // namespace
```

#### `geometry.h`
```cpp
namespace rag3db::geo_extension::math {

// Haversine distance (lat/lon in degrees → meters)
double haversine(double lat1, double lon1, double lat2, double lon2);

// Euclidean distance N-dim
double euclideanDist(const double* a, const double* b, uint32_t dims);

// Ray casting point-in-polygon 2D
bool pointInPolygon(double px, double py,
                    const double* polyX, const double* polyY, uint32_t numVertices);

// Point inside half-space: ax + by + cz + d >= 0
inline bool insidePlane(double px, double py, double pz,
                        double a, double b, double c, double d) {
    return a * px + b * py + c * pz + d >= 0.0;
}

// Point inside convex hull defined by N planes
bool insideConvex(double px, double py, double pz,
                  const double* planes, uint32_t numPlanes);  // planes = N×4

// Frustum from camera params → 6 planes (output = 24 doubles)
void frustumFromCamera(double posX, double posY, double posZ,
                       double qw, double qx, double qy, double qz,
                       double fovH, double fovV, double nearDist, double farDist,
                       double planesOut[24]);

} // namespace
```

### 4.3 Pattern Scalar Function

Toutes les scalar functions suivent le même pattern. Exemple pour `geo_within_obb` :

```cpp
// geo_within_obb.cpp
#include "function/geo_within_obb.h"
#include "math/quaternion.h"

static void withinObbExecFunc(
    const std::vector<std::shared_ptr<ValueVector>>& params,
    const std::vector<SelectionVector*>& paramSelVecs,
    ValueVector& result, SelectionVector* resultSelVec, void*) {

    // params[0] = point LIST(DOUBLE)
    // params[1] = center LIST(DOUBLE)
    // params[2] = half_extents LIST(DOUBLE)
    // params[3] = quaternion LIST(DOUBLE)

    for (auto i = 0u; i < resultSelVec->getSelSize(); ++i) {
        auto resPos = (*resultSelVec)[i];
        // 1. Extract lists
        // 2. p_local = quat_inverse.rotate(point - center)
        // 3. Check |p_local[k]| <= half_extents[k] for each axis
        result.setValue<bool>(resPos, inside);
    }
}
```

### 4.4 R-tree Query Modes

Le R-tree supporte en interne AABB, KNN, et radius. Pour OBB et frustum, on utilise
un pattern pré-filtre + test exact :

```
searchOBB(center, halfExtents, quat):
    aabb = computeAABBFromOBB(center, halfExtents, quat)  // AABB englobant
    candidates = searchBBox(aabb)                          // Pré-filtre R-tree
    results = []
    for offset in candidates:
        point = getCoords(offset)
        if testOBB(point, center, halfExtents, quat):     // Test exact
            results.push(offset, distance(point, center))
    return results

searchFrustum(planes[6]):
    aabb = computeAABBFromPlanes(planes)                   // AABB englobant
    candidates = searchBBox(aabb)
    results = []
    for offset in candidates:
        point = getCoords(offset)
        if insideConvex(point, planes, 6):                 // Test exact 6 plans
            results.push(offset, distance(point, frustumCenter))
    return results
```

---

## 5. Cas d'Usage

### 5.1 Géolocalisation classique (2D)
```cypher
-- Restaurants à moins de 2km
CALL QUERY_SPATIAL_INDEX('restaurants', 'geo_idx', [48.856, 2.352], 20,
    radius := 2000.0)
RETURN node_id, distance

-- Dans un quartier (polygone)
MATCH (r:restaurants)
WHERE geo_within_polygon(r.lat, r.lon,
    [48.85, 48.86, 48.86, 48.85],
    [2.34, 2.34, 2.36, 2.36])
RETURN r.name
```

### 5.2 Robotique — Champ de vision
```cypher
-- Objets dans le FOV d'un robot
WITH geo_frustum_from_camera(
    [robot.x, robot.y, robot.z],
    [robot.qw, robot.qx, robot.qy, robot.qz],
    1.0472, 0.7854, 0.3, 50.0
) AS frustum
CALL QUERY_SPATIAL_INDEX('objects', 'pos_idx', [robot.x, robot.y, robot.z], 100,
    frustum_planes := frustum)
RETURN node_id, distance
```

### 5.3 Architecture / BIM — Éléments dans une pièce
```cypher
-- Tous les meubles dans cette pièce (OBB orientée)
CALL QUERY_SPATIAL_INDEX('furniture', 'pos_idx', [0,0,0], 200,
    obb_center := [room.cx, room.cy, room.cz],
    obb_half_extents := [room.w/2, room.h/2, room.d/2],
    obb_quaternion := [room.qw, room.qx, room.qy, room.qz])
RETURN node_id, distance
```

### 5.4 Gaming / VR — Zone au sol
```cypher
-- Joueurs dans la zone de capture (polygone au sol, incliné)
MATCH (p:players)
WHERE geo_within_polygon_3d(
    [p.x, p.y, p.z],
    [0, 10, 10, 0], [0, 0, 8, 8],          -- polygone 10×8
    [zone.x, zone.y, zone.z],               -- position
    [zone.qw, zone.qx, zone.qy, zone.qz],  -- orientation
    thickness := 3.0                         -- hauteur joueur
)
RETURN p.name
```

### 5.5 Astronomie — Cône d'observation
```cypher
-- Étoiles dans le champ du télescope (frustum étroit)
WITH geo_frustum_from_camera(
    [0, 0, 0],                              -- observateur
    [scope.qw, scope.qx, scope.qy, scope.qz],
    0.01745, 0.01745,                        -- 1° FOV
    1.0, 1e15                                -- near/far
) AS cone
MATCH (s:stars)
WHERE geo_within_frustum([s.x, s.y, s.z], cone)
RETURN s.name, s.magnitude
```

---

## 6. Ordre d'Implémentation

### Phase 1 : Math + Fonctions pures (pas de DB)
1. `quaternion.h` — struct Quat, rotate, inverse, multiply, fromAxisAngle, toMatrix3
2. `matrix3.h` — struct Mat3, rotate, transpose, multiply
3. `geometry.h` — haversine, euclidean, pointInPolygon, insidePlane, insideConvex, frustumFromCamera
4. `geo_math_test.cpp` — tests unitaires pour toutes les maths

### Phase 2 : Scalar functions simples
5. `geo_distance.cpp` (existant, à vérifier)
6. `geo_distance_euclidean.cpp`
7. `geo_within_bbox.cpp` (existant)
8. `geo_within_bbox_nd.cpp`
9. `geo_within_polygon.cpp` (existant = geo_contains renommé)
10. `geo_within_circle.cpp`
11. `geo_within_sphere.cpp`

### Phase 3 : Scalar functions 3D + quaternions
12. `geo_quat_rotate.cpp`
13. `geo_quat_inverse.cpp`
14. `geo_quat_multiply.cpp`
15. `geo_quat_from_axis_angle.cpp`
16. `geo_quat_to_matrix.cpp`
17. `geo_matrix_rotate.cpp`
18. `geo_matrix_multiply.cpp`
19. `geo_matrix_transpose.cpp`

### Phase 4 : Containment 3D
20. `geo_within_obb.cpp` (+ variante matrix)
21. `geo_within_polygon_3d.cpp` (+ variante matrix)
22. `geo_within_frustum.cpp`
23. `geo_within_convex.cpp`
24. `geo_frustum_from_camera.cpp`

### Phase 5 : R-tree + Index (existant, à compléter)
25. Ajouter `searchOBB()` et `searchFrustum()` au R-tree
26. Ajouter modes OBB/frustum à `QUERY_SPATIAL_INDEX`
27. Tests E2E complets

### Phase 6 : Build + Tests
28. Fix CMakeLists (GTest, includes)
29. Build natif
30. Tests unitaires math + R-tree + E2E

---

## 7. Considérations

### Performance
- Quaternion rotation : ~20 FLOPs/point — négligeable
- OBB test : rotation inverse + 3 comparaisons — ~30 FLOPs/point
- Frustum test : 6 dot products — ~30 FLOPs/point
- R-tree pré-filtre : élimine 90%+ des candidats avant le test exact
- Tout est CPU-bound, zéro allocation dans le hot path

### Mémoire
- Maths header-only : zéro overhead
- R-tree : ~64 bytes/point (inchangé)
- Pas de structures supplémentaires en mémoire

### Précision
- Double precision (64-bit) partout
- Quaternions unitaires : normaliser si |q| ≠ 1 (tolérance 1e-10)
- Haversine : précision ~0.3% pour distances < 1000km

### WASM
- Tout C++ pur, zéro dépendance externe
- Compatible WASM statique (comme tantivy_fts)
- `<cmath>` (sin, cos, sqrt, atan2) disponible en WASM
