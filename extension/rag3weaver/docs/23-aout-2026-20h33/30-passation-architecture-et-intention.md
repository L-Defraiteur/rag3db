# Doc 30 — Passation : architecture et intention

Ce que le projet **est** et **veut**, tel qu'établi au 24 août 2026. Compagnons :
[29 — progression](29-passation-progression-24-aout-soir.md) ·
[31 — mode d'emploi](31-passation-mode-d-emploi.md) ·
[14 — mémoire des ambitions](14-memoire-des-ambitions.md) pour l'histoire longue.

## L'intention, en une phrase et son critère

Faire de **rag3weaver un moteur RAG / workflow / agentique tout terrain**, dont le
différenciateur est la **portabilité, l'embarquabilité, la packageabilité**. Le
critère qui tranche tout depuis le 6 février : **« Remplace Neo4j (zero Docker,
embedded) »** — ni Python, ni Docker, ni GPU d'une marque. C'est ce qui a fait
refuser PyTorch, choisir burn/Vulkan plutôt que candle, garder le fork kuzu plutôt
que Postgres, retirer BM42 (seule brique qui aurait exigé un export Python).

Cible immédiate : **un agent de code exposé en MCP**, sur base embarquée.
Horizon (doc 36) : **l'agent comme sous-graphe qui se compile en workflow**,
ses sessions et traces dans la même base, et des agents qui construisent des
agents par RAG sur les agents existants. Le
serveur MCP est le propriétaire unique de la base (kuzu verrouille au niveau
processus) ; les agents parlent au serveur.

Le différenciateur technique d'origine — chercher du **code** comme un humain
(`c++`, `std::collections`, `foo->bar`, séparateurs inclus, typos tolérées) —
est **tenu** depuis le 24 août : lucivy v3 le fait, `BM25Mode::Symbol` l'expose.

## La pile

```
rag3db  (fork Kuzu v0.11.2.2, licence LRSL v1.2, seuil 100 k€/an)
├── extension/vector         HNSW C++ (inchangé)               — vivant
├── extension/geo            R-tree, jamais testé E2E, 2 bugs   — orphelin
└── extension/rag3weaver     ~40 k lignes Rust                  — le produit
```

Supprimées le 24 août au soir : `extension/lucivy_fts` (C++ v2) et
`extension/sparse_vector` (C++ + copie du crate Rust). Le FTS et l'index
sparse sont des crates Rust compilés **dans** rag3weaver, tous deux issus du
workspace lucivy par chemin : `lucivy-core`, `luciole`, `sparse-vector`
(Apache-2.0, dérivé de Qdrant), sur `lucistore` (persistance commune).

**lucivy** (`~/git_workspaces/lucivy`, **arbre vivant de la session lucivy, par chemin — voulu, c'est l'usage naïf de rag3weaver qui valide la v3 non publiée** ; fork de tantivy 0.26 devenu sa propre lib —
ne jamais dire « fork de tantivy » devant Lucie) : SFX v3 (suffix FST), `contains`
cross-token avec séparateurs stricts ou relaxed, `parse` booléen traduit en
`boolean` de `contains`, `ShardedHandle` (4 shards par défaut chez nous),
`BlobShardStorage` sur `BlobStore`. Son scheduler d'acteurs est **luciole**
(pools, DAG, `execute_dag`), dont dépend aussi le search DAG de rag3weaver.

## rag3weaver, par couches

**Catalog** (`catalog.rs`) — la seule surface publique voulue : `register_entity`,
`register_kb`, `create`/`ingest_entities`, `drain`, `search`, `reindex`,
`shutdown`, et depuis le 24 août **`set_scope`/`scope`** (doc 37) : la cellule
`(org, project)` courante — stampe l'ingestion, sélectionne les index (un par
cellule), filtre la recherche ; `SearchOptions.scope`/`scopes` pour une autre
cellule ou un fan-out. Deux modes d'entité : **KB** (`{KB}_Index` + `{KB}_Index_Chunk`,
agrégation cross-entités, écrit par `KBUpdateNode`) et **simple** (`Entity` +
`Entity_Chunk`, `InsertRecordNode`). `Drop` ferme les handles FTS et flushe le
blob store — **jamais de panique qui s'échappe** (`catch_unwind`).

**Dataflow** (`dataflow/`) — DAG typé, 26 nœuds, checkpoint/undo. Deux runtimes
coexistent (dette de mai) : `DataflowRuntime` pour l'ingestion, luciole pour la
recherche. Nœuds FTS : `InsertRecordNode`/`UpdateRecordNode`/`DeleteRecordNode`
indexent via `ShardedHandle` ; `KBUpdateNode` aussi (`embed_get_offset`) ;
`FlushNode` = `handle.commit()`.

**Backend** — `DbConnection` (sync), `SchemaDialect` (50 méthodes, rag3db +
Postgres), `SearchBackend` (6 méthodes, vector + résolution seulement — **BM25 et
sparse ne dépendent d'aucun backend**, ce sont des moteurs Rust sur `BlobStore`).
Postgres compile, n'a jamais été exécuté ; `Catalog` ne choisit jamais
`PostgresBlobStore` (petit trou connu).

**BlobStore** — `CypherBlobStore` (table `_index_blobs`, MERGE/UNWIND) derrière
`BufferedBlobStore` (write-back, read-your-writes, flush aux frontières). Les
index FTS et sparse vivent **dans la base** : un seul fichier à sauvegarder. Le
cache mmap local est jetable.

**Recherche** (`search.rs`) — trois signaux (BM25, vector, sparse) fusionnés par
RRF (documenté comme approximatif, pas d'éval). BM25 : `Contains` (défaut,
relaxed, fuzzy), `ContainsSplit`, `Regex`, `Parse` (opt-in humain), `Symbol`
(exact, séparateurs inclus, fuzzy off). `meta.warnings` toujours peuplé.
**Invariant critique** : highlights ↔ chunks alignés par recouvrement
d'intervalles d'**octets** ; BM25 indexe le document entier, jamais les chunks ;
`_content` = `join("\n")` entre champs, sans terminateur.

**Embedders** — traits `Embedder` / `SparseEmbedder` / `DualEmbedder`. Produit :
`BurnMiniLmEmbedder` (384d, dense, navigateur) et `BurnBgeM3Embedder` (1024d,
dense + sparse appris, un forward). Oracle : candle (`CandleEmbedder`,
`BgeM3Embedder`), parité prouvée par `examples/*_reference.rs` +
`burn_*_vs_candle.rs`. Poids sur HF (`Lucie666/*-burnpack`), jamais dans git ;
code généré par burn-onnx dans `generated/`, commité, ~9-29 Ko compressés.

## Ce qui dort, et qui compte

- **`codeparsers/`** — 24 555 lignes, tree-sitter, 10 langages, `import_resolution`,
  `relationship_resolution`, `scope_extraction`. Référencé nulle part. C'est la
  brique qui produit les **arêtes** que la base graphe sait parcourir — le
  croisement graphe × recherche exacte qui justifie le fork kuzu.
- Les chantiers produit de mars jamais commencés : documents réels (Docling),
  reranking, **éval** (on ne mesure pas la pertinence), multi-tenancy, streaming,
  YAML universel. Doc 14 §5-6.
