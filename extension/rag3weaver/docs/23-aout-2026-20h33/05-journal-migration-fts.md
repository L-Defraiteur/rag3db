# Journal — migration FTS lucivy v3 (branche `fts-lucivy-v3`)

**Ce document est un filet de reprise.** Il est écrit pour être lu à froid, sans
contexte de la session, si celle-ci est interrompue. Il est mis à jour à chaque
étape franchie.

Plan de référence : [`04-migration-fts-lucivy-v3-rust.md`](04-migration-fts-lucivy-v3-rust.md)
(passation écrite depuis la session lucivy).

---

## Objectif

Remplacer les `CALL *_LUCIVY_INDEX` de l'extension C++ par des appels Rust directs
au `ShardedHandle` v3 de lucivy. 25 call sites au départ.

## Prérequis d'environnement

- Le v3 vit sur la branche **`v3-recovery`** de `~/git_workspaces/lucivy`. Vérifier
  avec `git -C ~/git_workspaces/lucivy branch --show-current` avant de builder.
- `lucivy-core` et `ld-lucivy` sont des **dépendances par chemin**, plus crates.io.
- Un **`[patch.crates-io]`** dans `Cargo.toml` redirige `lucivy-core` vers le chemin
  local. **Sans lui, le build casse** : `sparse-vector` tire la 2.0.0 de crates.io,
  on se retrouve avec deux traits `BlobStore` distincts, et `CypherBlobStore`
  n'implémente plus celui qu'attend sparse. À retirer à la publication de 2.1.0.

## Vérifier que tout va bien

```bash
cd extension/rag3weaver
cargo test --lib                    # doit être vert
cargo test --lib fts_handle         # les tests du socle, dont le bout-en-bout
cargo check --lib --features postgres
cargo check --lib --features wasm-emscripten
cargo check --lib --no-default-features --features burn-embedder
```

**Ne pas utiliser `RUSTFLAGS=-D warnings`** : cargo ne plafonne pas les lints des
dépendances par chemin, et `ld-lucivy` en génère 179. La sévérité est déclarée
dans `[lints.rust]` du `Cargo.toml`, qui est scopé au paquet.

---

## LA contrainte à ne jamais casser : alignement highlight ↔ chunk

BM25 cherche sur **l'entité parente entière** ; vector et sparse travaillent sur des
**chunks**. Un hit BM25 est ré-attribué à des chunks par **recouvrement d'intervalles
en octets** entre spans de highlight et spans de chunk (`search.rs:2130-2157`).

Deux systèmes de coordonnées :

- **KB** — highlights clés `_content` (la concaténation des champs agrégés), offsets
  globaux ; le chunk se traduit en `content_offset + start_char` → `content_offset + end_char`.
- **Entité simple** — highlights clés par vrai nom de champ ; appariement via
  `chunk.parent_field`, comparaison **locale au champ**, sans `content_offset`.

Intersection : `h_end.min(chunk_end) - h_start.max(chunk_start)` saturating, `>0` =
match, tri décroissant. Aucun chunk apparié → résultat parent avec `chunk: None`
(match dans le titre, non chunké).

**Piège de nommage levé** : `ChunkRecord.start_char` contient en réalité des **octets**
(`record_nodes.rs:1070` y écrit `chunk.start_byte`). Les highlights lucivy sont aussi
en octets. La comparaison est donc juste — seul le nom ment.

**Les trois règles qui en découlent :**

1. Le texte indexé doit être **identique à l'octet près** à celui que le chunker voit.
   Sinon tous les offsets glissent et l'attribution se dégrade **en silence**.
2. Les **noms de champs sont la clé de jointure** : les clés de highlight *sont* les
   noms de champs du schéma. Tout préfixe ou remapping casse tout. (C'est ce que
   faisait le C++ avec ses `_ngram`/`_raw` dérivés — d'où le bug que la migration enterre.)
3. Le mode de résultat (`Detailed` vs `Aggregated`/`SourceResolved`) dépend du nombre
   de chunks appariés, donc de l'exactitude des offsets.

**Trouvaille à ne pas traiter maintenant** : `_core_start_char`/`_core_end_char` (zone
sans recouvrement) sont écrits à l'ingestion et déclarés au schéma, mais **jamais
relus**. L'attribution utilise le span *avec* recouvrement, donc un highlight en zone
d'overlap matche deux chunks adjacents et les deux sortent. Défendable, mais ça duplique.
La zone core avait manifestement été capturée pour permettre l'attribution exclusive.
**Ne pas corriger pendant la migration** : ça changerait la sémantique et rendrait la
parité de l'étape 5 ininterprétable. Décision à prendre après, consciemment.

---

## Avancement

### ✅ Étape 0 — socle (`src/fts_handle.rs`, 5 tests)

| Élément | Rôle |
|---|---|
| `DynBlobStore` | pont `Arc<dyn BlobStore>` → `impl BlobStore` (`BlobShardStorage<S>` exige `S: Sized`) |
| `build_schema_config` | DDL rag3db → `SchemaConfig` v3, `sfx_version: 3` explicite |
| `build_document` | document + **`_node_id` dans le document** (piège de la passation) |
| `node_id_of` | hit de recherche → offset rag3db |
| `FtsStorage` | topologie (a) `BlobBacked` / (b) `LocalFs` |

Test clé : `index_search_and_resolve_offsets_end_to_end` — index en mémoire
(`MemBlobStore`), 3 docs aux offsets **41, 77, 1337**, recherche `kmalloc`, attend
exactement `[41, 1337]`. Offsets non contigus délibérément : un code confondant
l'indice de boucle et l'offset échouerait.

**Décision d'archi actée** : topologie **(a) `BlobBacked`**. Le blob store fait foi,
le cache mmap est jetable. Coût assumé : `BlobDirectory::new` (`blob_directory.rs:67-83`)
efface son cache `{pid}/{seq}` et **rematérialise tout l'index à chaque ouverture** ;
le `Drop` le supprime. Acceptable pour un serveur long-vécu, rédhibitoire pour un
navigateur. **(b) est le bon mode pour le WASM offline** (copie durable + deltas LUCIDS),
mais le WASM n'a jamais été débuggé de bout en bout et n'est pas la priorité —
décision de Lucie, 23 août. Le enum permet d'y venir sans rien recâbler.

### ✅ Étape 1 — handles dans le Catalog

`fts_handles: HashMap<String, Arc<ShardedHandle>>` à côté de `sparse_handles`.
`ensure_fts_handle()` (open puis create en repli), `fts_handle()`, `set_fts_storage()`.
Exposé aux nodes comme service `"fts_handles"` (3 sites d'enregistrement).

### ✅ Étape 2 — inserts + commit

- **`InsertRecordNode`** : indexation au point où `InternalNodeId::parse` donne
  l'offset. On passe **toutes** les valeurs texte du record, `build_document` ne
  retient que les champs au schéma — le schéma est l'unique source de vérité, pas
  une seconde liste à synchroniser avec `bm25_fields`. On indexe **la valeur écrite
  en base**, donc celle que le chunker verra (règle n°1 ci-dessus).
- **`FlushNode`** : `handle.commit()` si un handle existe, **repli sur
  `CALL FLUSH_LUCIVY_INDEX` sinon**. Le repli est délibéré : les deux chemins doivent
  coexister pour que la parité de l'étape 5 soit mesurable.

### ✅ Étape 3 — recherche

`search_bm25` / `search_bm25_chunked` (`search.rs:1727` et `:2048`) →
`handle.search_filtered(&query_config, limit, Some(sink), allowed_ids)`.

- `build_bm25_query` est **conservé tel quel** : son JSON se désérialise directement
  en `QueryConfig`.
- Les highlights viennent d'un `HighlightSink` par requête, ou de
  `search_with_docs` → `SearchHit.highlights: HashMap<String, Vec<[usize;2]>>`,
  **même forme que `parse_highlights_json`**.
**Fait.** `fts_handle::search_hits()` rend le triplet `(offset, score, highlights)`
— **exactement la forme** que rendait `CALL QUERY_LUCIVY_INDEX ... RETURN node_id,
score, highlights`. Toute l'attribution aux chunks en aval est donc inchangée, ce
qui rend la parité mesurable terme à terme.

`search_bm25_chunked` prend un paramètre `fts: Option<&ShardedHandle>` et branche :
handle présent → Rust, absent → repli C++. La partie commune (résolution des offsets,
appariement, mise en forme) a été extraite dans **`finish_bm25_chunked`**, partagée
par les deux chemins : toute divergence viendra donc du moteur, pas de la mise en forme.

`SearchDiagnostics` gagne `engine_warnings: Vec<String>`, alimenté par
`handle.query_warnings()` (littéral trop court, regex sans littéral = full scan,
fuzzy trop lâche, segments v2). Vide sur le chemin C++, qui ne les expose pas.

Test : `search_hits_returns_offsets_highlights_and_honours_filter` — vérifie les
offsets, que les highlights sont clés par **nom de champ**, que les bornes tombent
**dans** le texte indexé (sinon le référentiel serait cassé), et que `allowed_ids`
est honoré.

### ✅ Étape 4 — deletes / updates

**`DeleteRecordNode`** : `cache.remove(uuid)` rend l'`InternalNodeId`, donc l'offset,
qui est exactement la clé d'indexation. On désindexe là. Sans ça l'index garderait des
documents fantômes, qui ressortiraient en recherche avec des offsets ne résolvant plus.

**`UpdateRecordNode`** : ré-indexation par `delete_by_node_id` + `add_document`.

⚠️ **Subtilité coûteuse** : `add_document` **n'est pas un merge**. Ré-indexer en ne
passant que les champs modifiés (`rec.data`) ferait disparaître silencieusement les
champs texte inchangés. On **relit donc la ligne entière** via
`dialect.select_by_uuids`, en ne demandant que les champs réellement présents au
schéma de l'index (`fts_handle::indexed_text_fields`). Si aucun champ modifié n'est
indexé, on ne fait rien.

Helpers ajoutés : `indexed_text_fields()` et `reindex_document()`.

Test `reindex_replaces_document_and_delete_removes_it` : indexe (titre, corps),
ré-indexe en changeant le corps, vérifie que **l'ancien contenu disparaît**, que le
nouveau est trouvable, et que **le titre non modifié survit** — ce dernier point
échouerait si on ré-indexait un sous-ensemble. Puis supprime et vérifie l'absence de
fantôme.

`RechunkDeleteNode` supprime des *chunks*, qui n'ont pas d'index FTS (il vit sur la
table parente) : rien à faire.

### ⬜ Étape 5 — parité puis débranchement

Mêmes requêtes via C++ et via Rust sur un même corpus, puis retrait des hooks C++.
Enterre au passage le bug `_ngram`/`_raw` qui casse le mode Contains côté extension.

### ✅ Ouverture paresseuse câblée

Deux points de déclenchement, tous deux gardés par `target.default_signals.bm25()` :

- **`Catalog::search`** (après `resolve_search_target`) — pour la lecture.
- **`build_ingestion_graph`** — pour l'écriture. Sans lui, `InsertRecordNode` ne
  trouve aucun handle et n'indexe rien.

Les deux passent par `resolve_search_target` pour prendre **exactement les mêmes
`bm25_fields`** : une divergence entre champs indexés et champs cherchés casserait
l'appariement des highlights.

Pas à `register_entity`, qui paierait la rematérialisation complète au démarrage.

---

## Sites C++ restants

```bash
grep -rn "QUERY_LUCIVY_INDEX\|CREATE_LUCIVY_INDEX\|FLUSH_LUCIVY\|CLOSE_LUCIVY\|DROP_LUCIVY" \
  extension/rag3weaver/src --include='*.rs' | wc -l
```

Au départ : 25.
