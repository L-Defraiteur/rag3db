# 01 — Extension Geo complète : Math, Quaternions, OBB, Frustum, Polygon3D

## Résumé

Extension spatiale complète pour rag3db avec R-tree N-dimensionnel, 22 scalar functions (dont 19 nouvelles), 3 table functions, compatibilité Three.js. Build OK, 41 tests passent.

## Ce qui existait avant

- R-tree N-dim (insert, remove, KNN, bbox, radius, serialize/deserialize)
- 3 scalar functions : `GEO_DISTANCE`, `GEO_CONTAINS`, `GEO_WITHIN_BBOX`
- 3 table functions : `CREATE_SPATIAL_INDEX`, `QUERY_SPATIAL_INDEX`, `DROP_SPATIAL_INDEX`
- Build cassé (API rag3db avait changé depuis l'écriture initiale)

## Ce qui a été ajouté

### Couche math (header-only, zéro dépendance rag3db)

| Fichier | Contenu |
|---|---|
| `src/include/math/quaternion.h` | `Quat{w,x,y,z}` — identity, fromAxisAngle, inverse, multiply, rotate (sandwich optimisé), toMatrix3 |
| `src/include/math/matrix3.h` | `Mat3{m[9]}` row-major — identity, rotate, transpose, multiply |
| `src/include/math/geometry.h` | haversine, euclideanDist, pointInPolygon, insidePlane, insideConvex, frustumFromCamera |

### Helper Three.js

`src/include/function/geo_list_helpers.h` — conversion automatique des conventions :
- Quaternion : Three.js `[x,y,z,w]` ↔ interne `{w,x,y,z}`
- Matrice : Three.js column-major ↔ interne row-major
- Extraction/écriture LIST(DOUBLE) de taille fixe/variable

### 19 nouvelles scalar functions

**Distance :**
- `GEO_DISTANCE_EUCLIDEAN(LIST, LIST) → DOUBLE` — distance euclidienne N-dim

**Containment 2D :**
- `GEO_WITHIN_BBOX_ND(LIST, LIST, LIST) → BOOL` — AABB N-dim
- `GEO_WITHIN_CIRCLE(lat, lon, cLat, cLon, radius) → BOOL` — haversine
- `GEO_WITHIN_SPHERE(LIST, LIST, DOUBLE) → BOOL` — euclidien

**Quaternion :**
- `GEO_QUAT_ROTATE(LIST(4), LIST(3)) → LIST(3)`
- `GEO_QUAT_INVERSE(LIST(4)) → LIST(4)`
- `GEO_QUAT_MULTIPLY(LIST(4), LIST(4)) → LIST(4)`
- `GEO_QUAT_FROM_AXIS_ANGLE(LIST(3), DOUBLE) → LIST(4)`
- `GEO_QUAT_TO_MATRIX(LIST(4)) → LIST(9)`

**Matrice :**
- `GEO_MATRIX_ROTATE(LIST(9), LIST(3)) → LIST(3)`
- `GEO_MATRIX_MULTIPLY(LIST(9), LIST(9)) → LIST(9)`
- `GEO_MATRIX_TRANSPOSE(LIST(9)) → LIST(9)`

**Containment 3D :**
- `GEO_WITHIN_OBB(point, center, halfExtents, quat) → BOOL`
- `GEO_WITHIN_OBB_MATRIX(point, center, halfExtents, mat3) → BOOL`
- `GEO_WITHIN_POLYGON_3D(point, polyX, polyY, origin, quat [, thickness]) → BOOL`
- `GEO_WITHIN_POLYGON_3D_MATRIX(point, polyX, polyY, origin, mat3 [, thickness]) → BOOL`

**Frustum / Convex :**
- `GEO_WITHIN_FRUSTUM(LIST(3), LIST(N*4)) → BOOL`
- `GEO_WITHIN_CONVEX(LIST(3), LIST(N*4)) → BOOL`
- `GEO_FRUSTUM_FROM_CAMERA(pos, quat, fovH, fovV, near, far) → LIST(24)`

### R-tree : nouveaux modes de requête

- `searchOBB(center, halfExtents, quat)` — AABB pré-filtre (8 coins rotés) + test exact OBB
- `searchFrustum(planes, numPlanes, refPoint)` — scan feuilles + insideConvex
- `getCoords(nodeOffset)` — lookup via offsetToLeaf_
- Wrappers mutex dans `RTreeIndex`

### QUERY_SPATIAL_INDEX : nouveaux optional params

```sql
CALL QUERY_SPATIAL_INDEX('table', 'idx', [x,y,z], 100,
    obb_center := [cx,cy,cz],
    obb_half_extents := [hx,hy,hz],
    obb_quaternion := [qx,qy,qz,qw],
    frustum_planes := [a,b,c,d, ...])
RETURN n._id, n.distance;
```

Priorité : OBB > frustum > bbox > radius > KNN.

## Bugs corrigés

### R-tree splitNode : référence invalidée (crash std::bad_alloc)

`auto& node = nodes_[nodeIdx]` devenait un dangling reference après `nodes_.emplace_back()` (réallocation du vector). Fix : utiliser `nodes_[nodeIdx]` partout au lieu de la référence locale, copier `isLeaf` avant le emplace_back.

### API rag3db cassée (mise à jour depuis l'écriture initiale)

| Problème | Fix |
|---|---|
| `common/serializer/buffered_serializer.h` supprimé | → `buffer_writer.h` + `buffer_reader.h` |
| `TableFunction(name, types, func, bind)` 4 args | → 2 args + `func->tableFunc = ...` |
| `initLocalState(TableFuncInput&, ...)` | → `initLocalState(const TableFuncInitLocalStateInput&)` |
| `nodeTable.initScanState(tx, columnIDs)` | → pattern lucivy : NodeTableScanState + setToTable + lookup |
| `storageManager->getPageAllocator()` supprimé | → `index->saveToFile()` directement |
| `copyVector(columns)` — Expression::copy() supprimé | → `*this` (default copy comme lucivy) |
| `Index::InsertState` sans qualifier | → `storage::Index::InsertState` |
| `BufferReader` non inclus | → ajout `#include "common/serializer/buffer_reader.h"` |
| `condenseTree` non déclaré dans header | → ajout déclaration dans rtree.h |

## Fichiers créés (43)

```
src/include/math/quaternion.h
src/include/math/matrix3.h
src/include/math/geometry.h
src/include/function/geo_list_helpers.h
src/include/function/geo_distance_euclidean.h
src/include/function/geo_within_bbox_nd.h
src/include/function/geo_within_circle.h
src/include/function/geo_within_sphere.h
src/include/function/geo_quat_rotate.h
src/include/function/geo_quat_inverse.h
src/include/function/geo_quat_multiply.h
src/include/function/geo_quat_from_axis_angle.h
src/include/function/geo_quat_to_matrix.h
src/include/function/geo_matrix_rotate.h
src/include/function/geo_matrix_multiply.h
src/include/function/geo_matrix_transpose.h
src/include/function/geo_within_obb.h
src/include/function/geo_within_obb_matrix.h
src/include/function/geo_within_polygon_3d.h
src/include/function/geo_within_polygon_3d_matrix.h
src/include/function/geo_within_frustum.h
src/include/function/geo_within_convex.h
src/include/function/geo_frustum_from_camera.h
src/function/geo_distance_euclidean.cpp
src/function/geo_within_bbox_nd.cpp
src/function/geo_within_circle.cpp
src/function/geo_within_sphere.cpp
src/function/geo_quat_rotate.cpp
src/function/geo_quat_inverse.cpp
src/function/geo_quat_multiply.cpp
src/function/geo_quat_from_axis_angle.cpp
src/function/geo_quat_to_matrix.cpp
src/function/geo_matrix_rotate.cpp
src/function/geo_matrix_multiply.cpp
src/function/geo_matrix_transpose.cpp
src/function/geo_within_obb.cpp
src/function/geo_within_obb_matrix.cpp
src/function/geo_within_polygon_3d.cpp
src/function/geo_within_polygon_3d_matrix.cpp
src/function/geo_within_frustum.cpp
src/function/geo_within_convex.cpp
src/function/geo_frustum_from_camera.cpp
test/geo_math_test.cpp
```

## Fichiers modifiés (10)

```
src/main/geo_extension.cpp          — 19 registrations ajoutées
src/function/CMakeLists.txt          — 19 .cpp ajoutés
src/function/geo_distance.cpp        — refacto → math::haversine()
src/function/geo_contains.cpp        — refacto → math::pointInPolygon()
src/function/create_spatial_index.cpp — fix API scan + TableFunction constructor
src/function/query_spatial_index.cpp  — OBB/frustum modes + fix API
src/function/drop_spatial_index.cpp   — fix TableFunction constructor
src/include/index/rtree.h            — searchOBB, searchFrustum, getCoords, condenseTree
src/include/index/rtree_index.h      — wrappers + includes + saveToFile public
src/index/rtree.cpp                  — searchOBB, searchFrustum, getCoords, fix splitNode
src/index/rtree_index.cpp            — wrappers + fix namespace qualifiers + includes
test/CMakeLists.txt                  — fix GTest (add_rag3db_test) + geo_math_test
```

## Tests

```
geo_test      : 17/17 PASSED (BoundingBox 6, RTree 11)
geo_math_test : 24/24 PASSED (Quat 9, Mat3 5, Geometry 10)
Total         : 41/41 PASSED
```

## Build

```bash
cd packages/rag3db/build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="geo;lucivy_fts" -DBUILD_SHELL=FALSE -DBUILD_TESTS=TRUE
cmake --build . --target rag3db_geo_extension -j$(nproc)
cmake --build . --target geo_test geo_math_test -j$(nproc)
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./extension/geo/test/geo_test
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./extension/geo/test/geo_math_test
```

Note : `LD_LIBRARY_PATH` nécessaire à cause de miniconda qui pollue avec un vieux libstdc++.

## Conventions Three.js

- Quaternion : entrée/sortie `[x, y, z, w]` (scalar-last)
- Matrice 3x3 : entrée/sortie column-major `[m00,m10,m20,m01,m11,m21,m02,m12,m22]`
- Système : Y-up, right-handed, caméra regarde vers -Z local
- Frustum : 6 plans `[a,b,c,d]` où `ax+by+cz+d >= 0` = inside
