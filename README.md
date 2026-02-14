# rag3db

Fork de [Kuzu](https://github.com/kuzudb/kuzu) v0.11.2.2 avec recherche full-text fuzzy (Tantivy) et support WASM pour le navigateur.

## Pourquoi

Kuzu est un graph database embarqué performant, mais il manque :
- Un vrai full-text search avec fuzzy, phrase, highlights
- Un build WASM fonctionnel avec persistence navigateur (IDBFS)
- L'intégration des deux (FTS + vector HNSW) pour du RAG hybride

rag3db ajoute l'extension **tantivy_fts** qui comble ces manques.

## Ce qui marche

### Extension tantivy_fts

Recherche full-text via [Tantivy](https://github.com/quickwit-oss/tantivy) (moteur Rust, bridge cxx) :

```cypher
-- Créer un index
CALL CREATE_TANTIVY_INDEX('docs', ['title', 'body'])

-- Recherche exacte
CALL QUERY_TANTIVY_INDEX('docs', '{"type":"contains","field":"body","value":"programming"}', 10)
RETURN node_id, score

-- Recherche fuzzy (tolère 1 faute)
CALL QUERY_TANTIVY_INDEX('docs', '{"type":"fuzzy","field":"body","value":"programing","distance":1}', 10)
RETURN node_id, score

-- Recherche par phrase
CALL QUERY_TANTIVY_INDEX('docs', '{"type":"phrase","field":"body","terms":["systems","programming"]}', 10)
RETURN node_id, score, highlights

-- Supprimer l'index
CALL DROP_TANTIVY_INDEX('docs')
```

Fonctionnalites :
- Types de requetes : contains, fuzzy, phrase, term, regex, boolean, parse
- Highlights (positions des matchs dans le texte)
- Filter fields natifs (INT64, DOUBLE, etc. indexees dans Tantivy)
- Filtrage par `allowed_ids` (liste de node IDs)
- Hooks DELETE/UPDATE (mise a jour automatique de l'index)
- Lazy commit (dirty flag, commit+reload avant chaque QUERY)

### Extension vector (HNSW)

L'extension vector de Kuzu fonctionne normalement :

```cypher
CALL CREATE_VECTOR_INDEX('docs', 'emb_idx', 'embedding', metric := 'cosine')
CALL QUERY_VECTOR_INDEX('docs', 'emb_idx', [0.1, 0.2, 0.3, 0.4], 10)
RETURN node.id, node.title, distance
```

### Recherche hybride

Les deux s'utilisent ensemble sur la meme table :

```cypher
-- FTS pour le texte
CALL QUERY_TANTIVY_INDEX('docs', '{"type":"fuzzy","field":"body","value":"machine learning","distance":1}', 50)
RETURN node_id, score
-- Vector pour les embeddings
CALL QUERY_VECTOR_INDEX('docs', 'emb_idx', $query_embedding, 50)
RETURN node.id, distance
```

## Targets valides

| Target | Statut | Tests |
|--------|--------|-------|
| Natif (Linux x86_64) | OK | 9 E2E GTest |
| Node.js natif (NAPI) | OK | 139 mocha |
| WASM NODEFS (Node.js) | OK | 94 mocha |
| WASM browser (IDBFS) | OK | 2 Playwright (8 sub-tests) |

Extensions liees statiquement en WASM : tantivy_fts, vector, json, algo.

## Builds rapides

### Natif

```bash
mkdir -p build/release && cd build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSIONS="tantivy_fts" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . -j$(nproc)
```

### WASM (browser)

```bash
source ~/emsdk/emsdk_env.sh
mkdir -p build/wasm && cd build/wasm
emcmake cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_WASM=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_BENCHMARK=FALSE
emmake cmake --build . -j$(nproc)
# Sortie : tools/wasm/build/rag3db/rag3db_wasm.js (17MB, single file)
```

### Tests Playwright (browser IDBFS)

```bash
cd tools/wasm
npm install
npx playwright test
```

Voir [docs/14-fevrier-2026-00h58/09-guide-builds-et-tests.md](../ragforge-core-exp-kuzu/docs/14-fevrier-2026-00h58/09-guide-builds-et-tests.md) pour le guide complet.

## Architecture

```
rag3db (fork Kuzu v0.11.2.2)
├── extension/tantivy_fts/       Extension C++ (CREATE/QUERY/DROP)
│   └── CMakeLists.txt           Build Rust via cargo (natif + WASM)
├── extension/tantivy/ld-tantivy/  Submodule git (fork Tantivy v0.26.0)
│   └── tantivy_fts/rust/        Crate FFI (bridge cxx, 9 structs, 15 fonctions)
├── extension/vector/            Extension HNSW (inchangee)
├── tools/wasm/                  Build + tests WASM
│   ├── test/browser/            Tests Playwright IDBFS
│   └── build/rag3db/            Sortie WASM
└── tools/nodejs_api/            Build + tests Node.js natif
```

Le bridge **cxx** (pas extern C) donne des structs typees Rust <-> C++, zero JSON sur le hot path.

En WASM, les extensions sont liees statiquement et chargees automatiquement au demarrage de la DB.

## Ce qui reste a faire

### Court terme
- **Rag3Weaver** : wrapper Node.js/TypeScript qui expose les fonctions Cypher en API haut niveau
- **npm publish** : packaging du WASM build pour distribution npm
- **Stemming** : tokenizers multi-langues (FR, EN, DE, ES) -- le code est la mais pas teste en WASM

### Moyen terme
- **CI/CD** : GitHub Actions pour build + tests auto (natif, Node.js, WASM, Playwright)
- **Benchmarks** : perfs FTS sur datasets realistes (Wikipedia, etc.)
- **Thread pool dynamique** : actuellement 16 Workers pre-crees au demarrage, a optimiser

### Limitations connues
- L'ancienne extension `fts` (BM25 natif de Kuzu) n'est pas disponible en WASM (depend de `fuzzy_fst` non compile pour WASM)
- Le WASM single-file fait 17MB -- compressible avec gzip/brotli mais reste lourd
- Tantivy utilise 1 seul writer thread en WASM (vs 8 en natif) pour eviter l'epuisement du pool de pthreads
- Les headers COOP/COEP sont obligatoires pour le navigateur (SharedArrayBuffer)

## Provenance

- **Kuzu** : [kuzudb/kuzu](https://github.com/kuzudb/kuzu) v0.11.2.2 (MIT License)
- **Tantivy** : [quickwit-oss/tantivy](https://github.com/quickwit-oss/tantivy) v0.26.0 (MIT License)
