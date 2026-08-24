# Doc 13 — Knowledge dump

Tout ce qu'il faut pour reprendre le projet à froid : architecture, procédures de
build et de test, et les pièges qui ont coûté du temps. Écrit pour être lu sans
contexte de session.

Compagnons : [11 — état des lieux](11-etat-des-lieux-24-aout.md) ·
[12 — ambitions](12-ambitions-et-roadmap.md)

---

## 1. Ce qu'est le projet

**rag3db** est un fork de **Kuzu v0.11.2.2** transformé en base multi-index pour
le RAG : graphe (Cypher) + FTS (lucivy) + vectoriel (HNSW) + sparse (portage
Qdrant) + spatial (R-tree maison).

**rag3weaver** (`extension/rag3weaver`, ~40 k LOC Rust) est l'orchestrateur RAG
au-dessus : ingestion, chunking, embedding, indexation, recherche hybride.

Deux dépôts séparés interviennent :

| Dépôt | Rôle |
|---|---|
| `~/git_workspaces/rag3db` | le fork kuzu + rag3weaver |
| `~/git_workspaces/lucivy` | le moteur FTS (fork de tantivy 0.26, ~5 800 lignes de delta) |

## 2. Architecture de rag3weaver

### Deux niveaux d'API

- **`Catalog`** (haut niveau) : `new` → `initialize()` → `register_entity()` /
  `register_kb()` → `create()` / `ingest_entities()` → `drain()` → `search()`
- **Dataflow** (bas niveau) : DAG typé de 26 nodes, ports, `ServiceRegistry`

### Deux modes d'entité — la distinction structurante

- **KB (Knowledge Base)** : agrège des champs de plusieurs entités dans
  `{KB}_Index` + `{KB}_Index_Chunk`. Pipeline `KBGather → KBUpdate → KBChunk →
  KBEmbed`. **Les lignes d'index sont écrites directement par `KBUpdateNode`,
  pas par `InsertRecordNode`** — piège majeur, cf §6.
- **Entité simple** : table `{Entity}` + `{Entity}_Chunk`. Pipeline
  `Insert → Chunk → Insert(chunks) → Link → Embed → Flush → SparseCommit`.

`Catalog::resolve_search_target(name)` unifie les deux derrière un `SearchTarget`.
**Un KB se résout par son nom de KB**, pas par celui de ses entités sources.

### Couches d'abstraction backend

| Trait | Impls |
|---|---|
| `DbConnection` (sync, 2 méthodes) | `Rag3dbConnection`, `PostgresConnection`, `CallbackConnection`, `MockConnection`, `WasmDbConnection` |
| `SchemaDialect` (**50** méthodes) | `Rag3dbDialect`, `PostgresDialect` |
| `SearchBackend` (6 méthodes) | `Rag3dbSearchBackend`, `PostgresSearchBackend` |
| `BlobStore` (de lucistore) | `CypherBlobStore`, `PostgresBlobStore`, `MemBlobStore` |

FTS et sparse **ne passent pas** par `SearchBackend` : ils utilisent leurs
handles Rust, adossés au `BlobStore`, donc portables Postgres gratuitement.

### Les trois signaux de recherche

`SearchSignals` est un bitflag (BM25 / VECTOR / SPARSE), combinable. `FusionConfig`
donne par signal un poids, un rôle (Fuse/Boost) et une normalisation. Stratégies
`Rrf` (k=60) et `Weighted`.

## 3. LA contrainte à ne jamais casser : alignement highlight ↔ chunk

BM25 cherche sur **l'entité parente entière**, vector et sparse sur des **chunks**.
Un hit BM25 est ré-attribué par **recouvrement d'intervalles en octets**
(`search.rs`, `finish_bm25_chunked`).

Deux systèmes de coordonnées :

- **KB** : highlights clés `_content` (la concaténation), offsets globaux ; le
  chunk se traduit `content_offset + start_char` → `content_offset + end_char`
- **Entité simple** : highlights clés par vrai nom de champ, appariement via
  `chunk.parent_field`, comparaison **locale au champ**

**Piège de nommage** : `ChunkRecord.start_char` contient en réalité des **octets**
(`record_nodes.rs` y écrit `chunk.start_byte`). Les highlights lucivy sont aussi
en octets — la comparaison est donc juste, seul le nom ment.

**Trois règles qui en découlent :**

1. Le texte indexé doit être **identique à l'octet près** à celui que voit le
   chunker. Sinon tous les offsets glissent et l'attribution se dégrade **en
   silence**.
2. Les **noms de champs sont la clé de jointure** : les clés de highlight *sont*
   les noms de champs du schéma. Tout préfixe ou remapping casse tout.
3. Le mode de résultat (`Detailed` vs `Aggregated`/`SourceResolved`) dépend du
   nombre de chunks appariés, donc de l'exactitude des offsets.

## 4. Construire et tester

### Tests unitaires (rapides, sans DB)

```bash
cd extension/rag3weaver
cargo test --lib                    # 589 tests, < 1 s
```

**Ne jamais utiliser `RUSTFLAGS=-D warnings`** : cargo ne plafonne pas les lints
des dépendances **par chemin**, et `ld-lucivy` en génère 179. La sévérité est
déclarée dans `[lints.rust]` du `Cargo.toml`, scopé au paquet.

### Matrice de features

```bash
cargo check --lib                                            # défaut
cargo check --lib --features bge-m3
cargo check --lib --no-default-features --features candle-wasm
cargo check --lib --no-default-features --features burn-embedder
cargo check --lib --features postgres
cargo check --lib --features wasm-emscripten
cargo check --examples
```

⚠️ En **fish**, `cargo check --lib $f` avec `$f="--features postgres"` passe la
chaîne comme **un seul argument** et cargo la rejette. Utiliser une fonction avec
`"$@"`, sinon on obtient des faux négatifs (ça m'est arrivé deux fois).

Chaque combinaison doit être testée **séparément** : un `--all-features` vert ne
prouve rien, l'unification des features de cargo pouvant masquer un `dep:`
manquant.

### E2E (nécessitent le build natif)

```bash
# 1. build cmake (long la première fois, ~10 G de disque)
cd extension/rag3weaver && bash run_e2e.sh --build-only --no-cuda

# 2. lancer, avec les variables qui comptent
B=/home/lucied/git_workspaces/rag3db/build/native-test/src
export RAG3DB_SHARED=1 \
       RAG3DB_LIBRARY_DIR="$B" RAG3DB_INCLUDE_DIR="$B" \
       RAG3DB_ROOT=/home/lucied/git_workspaces/rag3db \
       LD_LIBRARY_PATH="$B"
cargo test --features rag3db-native --test e2e_search -- --ignored --test-threads=1
```

**`RAG3DB_SHARED=1` est indispensable.** Sans lui, les extensions sont liées à la
lib **release** du cmake pendant que le test charge la lib **debug** de cargo :
`undefined symbol: IndexAuxInfo`. Le header `rag3db.hpp` est produit par la cible
`single_file_header`, dans ce même répertoire.

Les 152 tests E2E sont tous `#[ignore]` + `#![cfg(feature = "rag3db-native")]`.

**`--release` n'aide presque pas** : 53 s contre 60 s. Le coût dominant est le
chargement des vrais modèles, refait par test — pas la compilation, pas le GPU
(qui n'accélère pas un chargement).

## 5. Environnement

**Machine AMD** : 2× Radeon AI PRO R9700 (Navi 48, gfx1201, RDNA4), ROCm 7.2.4,
**aucun CUDA**. `candle 0.8.4` n'expose que `accelerate · cuda · cudnn · metal ·
mkl` — pas de ROCm. La feature `cuda` est donc morte ici ; utiliser `--no-cuda`.

**burn/Vulkan marche** sur cette carte (RADV), et les **deux GPU sont adressables
séparément** (`DeviceKind::DiscreteGpu(0)` et `(1)`).

**Git** : la config globale porte l'email du travail ; chaque dépôt perso a un
override **local** `user.email = luciedefraiteur@gmail.com`. L'aiguillage des
clés SSH se fait par **l'URL du remote** (`git@github.com:` → clé perso) avec
`IdentitiesOnly yes`. Aucun `gh auth switch` nécessaire. **Jamais de mention
d'outil IA dans les messages de commit.**

## 6. Les pièges, et ce qu'ils ont coûté

### FTS / lucivy

- **Le pipeline KB n'appelle pas `InsertRecordNode`.** `KBUpdateNode` écrit les
  lignes `{KB}_Index` directement via le dialect. Un hook posé seulement sur
  `InsertRecordNode` ne les voit jamais → handle vide → recherche à 0, **en
  silence** (une table sans handle est simplement sautée).
- **Trois points d'entrée d'ingestion**, pas un : `build_ingestion_graph`,
  `ingest_entities` et `drain_resume` construisent chacun leur graphe et
  enregistrent leur propre service. D'où `Catalog::open_fts_handles_for()`,
  appelé aux trois.
- **`add_document_json` échoue sur un nom de champ inconnu** (à dessein). Le
  filtrage sur le schéma est donc **nécessaire**, pas cosmétique.
- **`add_document` n'est pas un merge.** Ré-indexer avec seulement les champs
  modifiés fait disparaître les autres. D'où la relecture complète de la ligne
  dans `UpdateRecordNode`.
- **`reindex` ne peut pas s'appuyer sur `UpdateRecordNode`** : celui-ci saute les
  enregistrements au hash inchangé, ce qui est exactement le cas d'un reindex.
  La détection de changement reste précieuse sur le chemin normal (elle évite de
  re-chunker et surtout de **re-embedder**) — les deux chemins veulent des
  comportements opposés.
- **`SUBSTRING` est 1-indexé** en Cypher comme en SQL. Une erreur d'un octet dans
  `load_range` décalerait tout l'index sans planter.
- **Le `.bpk` n'est pas reproductible octet à octet.** Deux générations donnent
  la même taille, des octets différents, des sorties numériquement identiques.
  Conséquence : ne pas le mettre en git (chaque régénération ajouterait une copie
  définitive), et republier le checksum si on régénère.

### Build C++

- **GCC 13+ ne fournit plus `<cstdint>` transitivement.** 613 fichiers du cœur
  s'y fiaient. Réglé par `-include cstdint` dans `build.rs` **et** `CMakeLists.txt`
  — mais **restreint au C++** par expression de générateur, sinon les unités C
  échouent sur `fatal error: cstdint`.
- **`bridge.rs.h` est généré par la cible Rust** via cxx. Les cibles OBJECT des
  extensions doivent déclarer `add_dependencies(..._rust)`, sinon `make -j`
  compile le C++ avant. Échec **intermittent**, donc trompeur.
- **Le submodule `ld-lucivy` doit être épinglé sur le même commit** que la
  dépendance par chemin de rag3weaver, sinon C++ et Rust compilent contre deux
  versions du moteur.

### Lifetimes / natif

- **Rust droppe les champs de struct dans l'ordre de déclaration.** `conn` doit
  rester le **dernier** champ du `Catalog`, après tout ce qui peut l'appeler
  pendant sa destruction. Il était premier ; la connexion C++ était donc détruite
  avant les index qui écrivent à travers elle.
- **Une bibliothèque Rust ne devrait jamais segfaulter.** Un SIGSEGV implique du
  `unsafe` (mmap, cxx). Le symptôme observé venait de lucivy : `close()` mettait
  les acteurs « au repos » mais pas « inertes ». C'est le genre de chose qu'il
  faut remonter en amont plutôt que contourner.

## 7. Où sont les choses

```
extension/rag3weaver/
├── src/
│   ├── catalog.rs              orchestrateur (~4500 l.)
│   ├── search.rs               primitives de recherche, fusion
│   ├── dialect.rs              SchemaDialect, 50 méthodes, 2 impls
│   ├── fts_handle.rs           socle FTS lucivy v3 (créé le 23 août)
│   ├── burn_bge_m3_embedder.rs embedder burn (créé le 23 août)
│   ├── bge_m3_embedder.rs      embedder candle (+ from_local_dir)
│   └── dataflow/               26 nodes, runtime, checkpoint
├── generated/                  code produit par burn-onnx, NON écrit à la main
├── examples/
│   ├── bge_m3_reference.rs     oracle candle (CPU forcé)
│   ├── burn_vs_candle.rs       parité via les traits publics
│   └── burn_throughput.rs      balayage batch × longueur
├── docs/23-aout-2026-20h33/    04→13 : passation lucivy, journal, ces docs
└── run_e2e.sh
```

Convention de docs : **`extension/rag3weaver/docs/<date>/`** pour rag3weaver,
`docs/<date>/` à la racine pour le fork kuzu et ses extensions C++.
