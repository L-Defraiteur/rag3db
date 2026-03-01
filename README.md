# rag3db

Fork of [Kuzu](https://github.com/kuzudb/kuzu) v0.11.2.2 — embeddable graph database with full-text search (Tantivy), vector search (HNSW), sparse vector search, spatial indexing (R-tree), and a complete RAG framework (rag3weaver). Native + browser WASM.

## Extensions

### tantivy_fts — Full-Text Search

Full-text search via [ld-tantivy](https://github.com/L-Defraiteur/tantivy/) (fork of Tantivy v0.26.0, cxx bridge):

```cypher
-- Create index (text fields + optional filter fields)
CALL CREATE_TANTIVY_INDEX('docs', ['title', 'body'],
     filter_fields := [('category', 'STRING'), ('year', 'INT64')])

-- Substring (trigram-accelerated, BM25, multi-field highlights)
CALL QUERY_TANTIVY_INDEX('docs',
     '{"type":"contains","field":"body","value":"programming"}', 10)
RETURN node_id, score, highlights

-- Fuzzy (tolerates typos)
CALL QUERY_TANTIVY_INDEX('docs',
     '{"type":"contains","field":"body","value":"programing","distance":1}', 10)
RETURN node_id, score, highlights

-- Regex (trigram-accelerated + regex check + BM25)
CALL QUERY_TANTIVY_INDEX('docs',
     '{"type":"contains","field":"body","value":"program[a-z]+","regex":true}', 10)
RETURN node_id, score, highlights

-- Boolean multi-field
CALL QUERY_TANTIVY_INDEX('docs',
     '{"type":"boolean","should":[
       {"type":"contains","field":"title","value":"rust"},
       {"type":"contains","field":"body","value":"rust"}
     ]}', 10)
RETURN node_id, score, highlights

-- Filter by IDs
CALL QUERY_TANTIVY_INDEX('docs', '...', 10, allowed_ids := [1, 5, 12])
RETURN node_id, score, highlights

-- Drop index
CALL DROP_TANTIVY_INDEX('docs')
```

8 query types: contains, fuzzy, regex, phrase, term, boolean, parse, regex+fuzzy hybrid.

Multi-field highlights: each field in a boolean query produces its own byte offsets (`{"title":[[0,4]],"body":[[100,120]]}`).

DELETE/UPDATE hooks: the index auto-updates on Cypher mutations. Lazy commit (dirty flag, commit+reload once before each QUERY).

### vector — HNSW Vector Search

Kuzu's vector extension with DELETE and UPDATE support:

```cypher
CALL CREATE_VECTOR_INDEX('docs', 'emb_idx', 'embedding', metric := 'cosine')
CALL QUERY_VECTOR_INDEX('docs', 'emb_idx', $query_embedding, 10)
RETURN node.id, node.title, distance
```

Node deletions and updates automatically propagate to the HNSW index (batched edge cleanup).

### sparse_vector — Sparse Vector Search

Sparse index for BM42/SPLADE-style lexical embeddings:

```cypher
CALL CREATE_SPARSE_VECTOR_INDEX('docs', 'sparse_idx', 'sparse_indices', 'sparse_weights')
CALL QUERY_SPARSE_VECTOR_INDEX('docs', 'sparse_idx', [1,5,42], [0.8,0.3,0.1], 10)
RETURN node_id, score
```

### geo — Spatial Indexing & Geometry

N-dimensional R-tree spatial index with 5 query modes, plus 19 scalar geometry functions:

```cypher
-- Create spatial index (haversine for lat/lon, euclidean for N-D)
CALL CREATE_SPATIAL_INDEX('places', 'geo_idx', ['lat', 'lon'], metric := 'haversine')

-- KNN: 10 nearest neighbors
CALL QUERY_SPATIAL_INDEX('places', 'geo_idx', [48.856, 2.352], 10)
RETURN node_id, distance

-- Radius: all points within 5km
CALL QUERY_SPATIAL_INDEX('places', 'geo_idx', [48.856, 2.352], 100,
    radius := 5000.0)
RETURN node_id, distance

-- OBB: oriented bounding box (3D with quaternion rotation)
CALL QUERY_SPATIAL_INDEX('objects', 'pos_idx', [0,0,0], 50,
    obb_center := [10.0, 5.0, 2.0],
    obb_half_extents := [2.0, 1.0, 0.5],
    obb_quaternion := [0.7071, 0.0, 0.7071, 0.0])
RETURN node_id, distance

-- Frustum: camera field-of-view query
CALL QUERY_SPATIAL_INDEX('objects', 'pos_idx', [0,0,0], 100,
    frustum_planes := geo_frustum_from_camera(
        [cam.x, cam.y, cam.z], [cam.qw, cam.qx, cam.qy, cam.qz],
        1.047, 0.785, 0.3, 50.0))
RETURN node_id, distance

CALL DROP_SPATIAL_INDEX('places', 'geo_idx')
```

**5 query modes**: KNN, radius, bounding box, oriented bounding box (OBB), frustum/convex hull.

**19 scalar functions**:

| Category | Functions |
|----------|-----------|
| Distance | `geo_distance` (haversine), `geo_distance_euclidean` (N-D) |
| 2D containment | `geo_within_bbox`, `geo_within_bbox_nd`, `geo_within_polygon`, `geo_within_circle` |
| 3D containment | `geo_within_sphere`, `geo_within_obb`, `geo_within_obb_matrix`, `geo_within_polygon_3d`, `geo_within_polygon_3d_matrix`, `geo_within_frustum`, `geo_within_convex` |
| Quaternion | `geo_quat_rotate`, `geo_quat_inverse`, `geo_quat_multiply`, `geo_quat_from_axis_angle`, `geo_quat_to_matrix` |
| Matrix3 | `geo_matrix_rotate`, `geo_matrix_multiply`, `geo_matrix_transpose` |
| Helper | `geo_frustum_from_camera` (position + quaternion + FOV -> 6 clip planes) |

Persistent R-tree with automatic CRUD hooks. Header-only math (quaternion, matrix3, geometry) — zero external dependencies, WASM-compatible.

## rag3weaver — RAG Framework

High-level Rust framework that orchestrates all extensions above. Provides CRUD, ingestion, chunking, embeddings, and hybrid search via a `Catalog` API. See [rag3weaver/README.md](extension/rag3weaver/README.md) for full documentation.

```rust
let mut catalog = Catalog::new(conn, embedder, config);
catalog.set_dual_embedder(bge_m3.clone()); // single forward pass for dense + sparse
catalog.initialize().await?;

// Ingestion
catalog.create("Document", data)?;
catalog.drain().await?; // pipeline: chunk -> insert -> link -> embed (dense+sparse)

// Hybrid search (vector + BM25 + sparse)
let response = catalog.search("main", "rust programming", &options).await?;
```

## Targets

| Target | Status | Tests |
|--------|--------|-------|
| Native (Linux x86_64) | OK | 15 GTest E2E + 1064 Rust tests (ld-tantivy) + 380 Rust tests (rag3weaver) |
| Node.js native (NAPI) | OK | contains/fuzzy/regex/phrase/parse verified |
| Browser WASM | OK | Playwright (FTS + vector + persistence IDBFS) |

Statically linked extensions in WASM: tantivy_fts, vector, sparse_vector, json, algo.

## Build

See **[BUILD.md](BUILD.md)** for the full guide.

```bash
# Native
mkdir -p build/release && cd build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSIONS="tantivy_fts;geo" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . -j$(nproc)

# E2E tests (rag3weaver)
cd extension/rag3weaver && bash run_e2e.sh phase0
```

## Architecture

```
rag3db (fork Kuzu v0.11.2.2)
|-- extension/tantivy_fts/           C++ FTS extension (CREATE/QUERY/DROP)
|-- extension/tantivy/ld-tantivy/    Submodule (fork Tantivy v0.26.0)
|   +-- tantivy_fts/rust/            FFI crate (cxx bridge)
|-- extension/vector/                HNSW extension (+ DELETE/UPDATE)
|-- extension/sparse_vector/         Sparse vector extension
|-- extension/geo/                   Spatial extension (R-tree + 19 geo functions)
|-- extension/rag3weaver/            RAG framework (Rust)
|   |-- src/                         Catalog, search, chunker, queue, embedders
|   +-- tests/                       E2E native (e2e_search.rs)
|-- tools/wasm/                      WASM build + tests
+-- tools/nodejs_api/                Node.js native build
```

The **cxx** bridge (not extern C) provides typed Rust <-> C++ structs, zero JSON on the hot path.

## Provenance

- **Kuzu**: [kuzudb/kuzu](https://github.com/kuzudb/kuzu) v0.11.2.2 (MIT License, see NOTICE)
- **Tantivy**: [L-Defraiteur/tantivy](https://github.com/L-Defraiteur/tantivy/) (fork of v0.26.0, MIT License)

## License

[Luciform Research Source License (LRSL) v1.2](LICENSE) — source-available, free under 100K EUR/year revenue.
