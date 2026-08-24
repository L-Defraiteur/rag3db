# Migration FTS → lucivy v3 en Rust direct (passation)

23 août 2026. Écrit depuis la session lucivy, après validation complète du v3.
Tout ce document est actionnable **ici, dans rag3weaver**, sans toucher au repo
lucivy. Référence détaillée côté moteur :
`~/git_workspaces/lucivy/docs/22-aout-2026-19h47/10-plan-rag3weaver-v3.md`
(et `09-knowledge-dump-algorithmique.md` pour comprendre le moteur).

## Partage des responsabilités

**Reste côté lucivy (ne pas faire ici, signaler si bloquant) :**
1. ~~Chargement paresseux~~ **FAIT (24 août), optionnel** :
   `.with_load_mode(BlobLoadMode::Lazy)` sur `BlobShardStorage` — rien de
   téléchargé à l'ouverture, lectures de footer servies par plage, fichier
   matérialisé en mmap à la 4e lecture. Défaut inchangé (Eager). Pour le
   plein effet, implémenter sur vos stores les deux méthodes à défaut du
   trait `BlobStore` :
   `blob_len` (`SELECT LENGTH(_data)` / `RETURN SIZE(b._data)`) et
   `load_range` (`SUBSTRING(_data FROM $1+1 FOR $2)`). Sans elles, le lazy
   retombe sur « téléchargement complet au premier accès du fichier » —
   correct mais sans gain. Benchmarker Eager vs Lazy sur vos vrais index.
2. Publication crates.io (`lucivy-core` 2.1.0 + `ld-lucivy`, `luciole`,
   `lucistore`) — en attendant, dépendance par chemin (ci-dessous).
3. API « drop index » propre (palliatif : `store.list(prefix)` + `delete`).
4. Le binding emscripten C (mais vous n'en avez pas besoin : compilez
   `lucivy-core` en Rust dans votre wasm, comme le reste).

**Tout le reste est faisable ici** : câblage Catalog, inserts par offsets,
commit, recherche, suppression des hooks C++. Si un comportement du moteur
paraît faux (documents manqués, spans décalées), c'est un bug lucivy : le
signaler avec un repro, ne pas le contourner.

## Dépendance (avant publication)

```toml
# Cargo.toml — remplace lucivy-core = "2.0.0"
lucivy-core = { path = "../../../lucivy/lucivy_core" }
```
⚠️ le v3 vit sur la branche **`v3-recovery`** du repo lucivy
(`~/git_workspaces/lucivy`), pas sur main. Vérifier `git branch --show-current`
là-bas avant de builder. Le trait `BlobStore` n'a pas changé :
`CypherBlobStore`/`PostgresBlobStore` compilent tels quels.

## Ce qui est validé côté moteur (aujourd'hui, spans exactes vs disque)

`ShardedHandle` : `search`, `search_filtered(allowed_ids)` (pré-filtrage BDD),
`delete_by_node_id`, deltas LUCIDS, distribué (export_stats → merge →
search_with_global_stats), fuzzy/regex multi-shards, `close()`,
`query_warnings`. **ACID v3** : `BlobShardStorage` sur un `BlobStore`, blobs =
source de vérité, cache mmap local jetable, réouverture depuis les blobs seuls
(`lucivy_core/tests/test_acid_blob_v3.rs` = le modèle à copier).
`sfx_version: 3` est le défaut pour tout nouvel index.

## Setup

```rust
use lucivy_core::sharded_handle::{ShardedHandle, BlobShardStorage};

let store = catalog.blob_store();                       // CypherBlobStore existant
let storage = BlobShardStorage::new(store, table_name, cache_base);
let handle = ShardedHandle::create_with_storage(Box::new(storage), &config)?;
// réouverture : ShardedHandle::open_with_storage(Box::new(storage))?
```
`config` = `SchemaConfig` désérialisé du même JSON que les bindings :
`{"fields":[{"name":"_title","type":"text","stored":true},...],"shards":N}`.
`filter_fields` du DDL → champs non-texte dans `fields` + `QueryConfig.filters`
(FilterClause) à la requête — vérifier la parité au branchement.

## Mapping des opérations (remplace les CALL Cypher)

| Aujourd'hui | Demain |
|---|---|
| `CREATE_LUCIVY_INDEX` | `create_with_storage` (ci-dessus) |
| hooks C++ insert | `add_document_json(offset, &json!({"_title": t, "_content": c}))` — champs par nom, types vérifiés par le schéma, erreurs explicites ; ou `add_document(doc, offset)` (le `_node_id` est estampillé automatiquement depuis le 24 août, un id contradictoire dans le doc est refusé) |
| delete implicite | `delete_by_node_id(offset)` |
| `FLUSH_LUCIVY_INDEX` | `handle.commit()` (idempotent ; merges par policy en tâche de fond) |
| `CLOSE_LUCIVY_INDEX` | `handle.close()` (draine les merges) |
| `DROP_LUCIVY_INDEX` | palliatif : `store.list("Lucivy_{name}…")` + `delete` par clé |
| `QUERY_LUCIVY_INDEX(json, limit, allowed_ids)` | `search_filtered(&query_config, limit, Some(sink), allowed_ids)` — le JSON `build_bm25_query` se désérialise tel quel en `QueryConfig` |
| colonne `highlights` | `HighlightSink` par requête ; ou `search_with_docs` → `SearchHit.highlights: HashMap<String, Vec<[usize;2]>>` (même forme que `parse_highlights_json`) |

En plus, gratuit : `handle.query_warnings(&config)` → `Vec<String>` de
limitations honnêtes (littéral court, regex sans littéral = full scan, fuzzy
trop lâche, segments v2) — à remonter dans les diagnostics de recherche.

## Pièges connus (appris en les payant)

- ~~`_node_id` dans le document~~ réglé le 24 août : `add_document` l'estampille
  lui-même ; un doc portant un autre id est refusé avec un message clair.
- Après `commit()`, la lecture est rechargée par le handle ; mais une copie
  brute des fichiers (snapshot maison) doit attendre les merges :
  `shard.writer.lock().unwrap().as_ref().unwrap().drain_merges()`.
- `Consistency::Strict` → `commit()` suffit (il flush + reload).
- Chargement : en Eager (défaut), l'ouverture télécharge tout l'index ; en
  Lazy (option ci-dessus), quasi rien. Dans les deux cas, garder la map
  `entity → handle` ouverte paresseusement comme `sparse_handles` reste une
  bonne idée (un handle ouvert = writer lock + scheduler).
- Sémantique v3 utile : les séparateurs (`_`, `:`, …) sont des frontières de
  mots ; `term`/`startsWith` s'ancrent sur ces mots ; le mode relaxed les
  ignore (`__init` cherche `init`). Fuzzy toujours relaxed.

## Ordre proposé

1. Map `fts_handles: HashMap<String, ShardedHandle>` dans `Catalog` à côté de
   `sparse_handles`, ouverture lazy, `BlobShardStorage` + `CypherBlobStore`.
2. Inserts Rust par offsets (pattern de `record_nodes.rs:773-781` pour sparse) ;
   `FlushNode` → `commit()`.
3. `search_bm25*` → `search_filtered` + sink ; garder `build_bm25_query` tel
   quel (désérialisation `QueryConfig`).
4. Deletes/updates → `delete_by_node_id` explicite.
5. Parité mesurée (mêmes requêtes via C++ et via Rust sur un même corpus),
   puis débrancher les hooks C++ — ça enterre au passage le bug `_ngram`/`_raw`
   qui casse le mode Contains côté extension.
