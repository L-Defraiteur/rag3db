# rag3db

Fork de [Kuzu](https://github.com/kuzudb/kuzu) v0.11.2.2 — graph database embarque avec full-text search avance (Tantivy) et support WASM navigateur.

## Pourquoi

Kuzu est un graph database embarque performant, mais il manque :
- Un vrai full-text search avec fuzzy, regex, phrase, highlights et scoring BM25
- Un build WASM fonctionnel avec persistence navigateur (IDBFS)
- L'integration des deux (FTS + vector HNSW) pour du RAG hybride

rag3db ajoute l'extension **tantivy_fts** qui comble ces manques.

## Ce qui marche

### Extension tantivy_fts

Recherche full-text via [Tantivy](https://github.com/quickwit-oss/tantivy) (moteur Rust, bridge cxx) :

```cypher
-- Creer un index
CALL CREATE_TANTIVY_INDEX('docs', ['title', 'body'])

-- Recherche substring (trigram-acceleree, BM25)
CALL QUERY_TANTIVY_INDEX('docs', '{"type":"contains","field":"body","value":"programming"}', 10)
RETURN node_id, score, highlights

-- Recherche fuzzy (tolere les fautes de frappe)
CALL QUERY_TANTIVY_INDEX('docs', '{"type":"contains","field":"body","value":"programing","distance":1}', 10)
RETURN node_id, score

-- Recherche regex (trigram-acceleree + verification regex + BM25)
CALL QUERY_TANTIVY_INDEX('docs', '{"type":"contains","field":"body","value":"program[a-z]+","regex":true}', 10)
RETURN node_id, score, highlights

-- Regex + fuzzy hybride (le regex matche precis, le fuzzy rattrape les fautes)
CALL QUERY_TANTIVY_INDEX('docs', '{"type":"contains","field":"body","value":"programing[a-z]+","regex":true,"distance":1}', 10)
RETURN node_id, score, highlights

-- Recherche par phrase
CALL QUERY_TANTIVY_INDEX('docs', '{"type":"phrase","field":"body","terms":["systems","programming"]}', 10)
RETURN node_id, score, highlights

-- Supprimer l'index
CALL DROP_TANTIVY_INDEX('docs')
```

### Pipeline contains unifie

Le coeur de tantivy_fts est un **pipeline `"contains"` unifie** qui gere fuzzy, regex et les deux combines via un seul `NgramContainsQuery`. Le mode est selectionne par `"regex": true/false` (defaut: false).

```
                     Query JSON
                         |
             ┌───────────┴───────────┐
             │ regex: false          │ regex: true
             ▼                       ▼
     Tokenize texte           regex_syntax::parse()
     → tokens                 → Hir → Extractor
             │                → litteraux obligatoires
             └───────────┬───────────┘
                         │
                 trigram_sources
                         │
         ┌───────────────┼───────────────┐
     Fuzzy            Regex           Regex (short)
     exact+ngram      ngram union     full-scan
     intersection     (lits >= 3)     (lits < 3)
         │               │               │
         └───────────────┼───────────────┘
                         │
             Pour chaque candidat :
             load stored text
                         │
         ┌───────────────┴───────────────┐
     Fuzzy                            Regex
     Levenshtein → tf          1. regex::find_iter → tf_regex
                               2. si distance > 0 :
                                  fuzzy sur lits → tf_fuzzy
                               → tf = max(tf_regex, tf_fuzzy)
         │                               │
         └───────────────┬───────────────┘
                         │
                 BM25 score + Highlights
```

Ce qui rend ce pipeline unique :

- **Regex accelere par trigrams** : le pattern regex est parse (`regex_syntax`), les litteraux obligatoires sont extraits, convertis en trigrams, et utilises pour reduire les candidats AVANT d'executer le regex. Sur un index de 100K docs, seule une fraction est verifiee.
- **Scoring BM25 uniforme** : fuzzy et regex partagent le meme scorer BM25. Pas de `ConstScorer` artificiel — le score reflete la frequence reelle dans le document.
- **Hybride regex+fuzzy** : quand `regex: true` et `distance > 0`, la verification fait les deux : regex exact + fuzzy Levenshtein sur les litteraux. `tf = max(tf_regex, tf_fuzzy)`. Le regex matche precis, le fuzzy rattrape les fautes de frappe dans la partie litterale.
- **Full-scan fallback intelligent** : quand les litteraux du regex sont trop courts pour des trigrams (< 3 chars, ex: `v[0-9]`), le scorer scanne tous les docs du segment mais garde le BM25. Pas de degradation en ConstScorer.

### Autres fonctionnalites

- **8 types de requetes** : contains, fuzzy, regex, phrase, term, boolean, parse + mode hybride
- **Highlights** : positions exactes (byte offsets) des matchs dans le texte source
- **Filter fields natifs** : colonnes non-texte (INT64, DOUBLE, etc.) indexees dans Tantivy
- **Filtrage par `allowed_ids`** : restreindre la recherche a une liste de node IDs
- **Hooks DELETE/UPDATE** : mise a jour automatique de l'index sur mutations Cypher
- **Lazy commit** : dirty flag, commit+reload une seule fois avant chaque QUERY

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
| Natif (Linux x86_64) | OK | 11 E2E GTest + 1025 tests Rust |
| Node.js natif (NAPI) | OK | contains/fuzzy/regex/phrase valides |
| WASM Node.js (sync) | OK | contains/fuzzy/regex/hybrid valides |
| WASM browser (IDBFS) | OK | 2 Playwright (contains/fuzzy/regex/hybrid/vector/persistence) |

Extensions liees statiquement en WASM : tantivy_fts, vector, json, algo.

## Builds

Voir **[BUILD.md](BUILD.md)** pour le guide complet (natif, Node.js, WASM, tests, problemes courants).

Quick start natif :

```bash
mkdir -p build/release && cd build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSIONS="tantivy_fts" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . -j$(nproc)
```

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
│   └── package/nodejs/rag3db/   Sortie WASM (genere par cmake)
└── tools/nodejs_api/            Build + tests Node.js natif
    └── src_js/rag3dbjs.node     Sortie .node (genere par cmake)
```

Le bridge **cxx** (pas extern C) donne des structs typees Rust <-> C++, zero JSON sur le hot path.

En WASM, les extensions sont liees statiquement et chargees automatiquement au demarrage de la DB.

Les builds cmake detectent automatiquement les changements dans les sources Rust (`file(GLOB_RECURSE)` + `DEPENDS`) — pas besoin de rebuild cargo manuellement.

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
