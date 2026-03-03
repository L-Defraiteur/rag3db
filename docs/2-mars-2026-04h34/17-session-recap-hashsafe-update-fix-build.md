# Doc 17 — Session Recap: Hashsafe type-safe, update() fix, build script

**Date**: 3 mars 2026
**Branche**: `feature/kb-index-architecture`

---

## Résumé

Trois améliorations : (1) remplacement de l'heuristique `resolve_ref_or_uuid` par un struct `Hashsafe` type-safe, (2) fix du bug update() contentFor-only (Kuzu `WITH...SET...RETURN` retourne la valeur post-SET), (3) refonte du script `run_e2e.sh` pour éviter les rebuilds inutiles. **14/14 tests E2E phase0b passent.**

---

## 1. Struct `Hashsafe` (remplacement de `resolve_ref_or_uuid`)

### Problème

La méthode `resolve_ref_or_uuid()` dans `catalog.rs` détectait les lookup strings (ex: `"Directory:/repo/src/"`) par heuristique : préfixe `"EntityName:"` puis split par `:`. Deux faiblesses :
1. Si un UUID contenait `:` → faux positif (risque faible mais réel)
2. Pour multi-field hashsafe avec `:` dans les valeurs → split incorrect

### Solution

Nouveau struct `Hashsafe` dans `ops.rs` avec `From<Hashsafe> for RefOrUuid` qui calcule le UUID immédiatement via `hashsafe_uuid()` :

```rust
pub struct Hashsafe {
    pub entity: String,
    pub values: Vec<String>,
}

impl Hashsafe {
    pub fn new(entity: &str, values: &[&str]) -> Self { ... }
}

impl From<Hashsafe> for RefOrUuid {
    fn from(h: Hashsafe) -> Self {
        let strs: Vec<&str> = h.values.iter().map(|s| s.as_str()).collect();
        Self::Uuid(hashsafe_uuid(&h.entity, &strs))
    }
}
```

Usage :
```rust
// AVANT (ambigu — heuristique fragile)
catalog.link("HAS_FILE", "Directory:/repo/src/", file_ref, ...)

// APRÈS (explicite, type-safe)
catalog.link("HAS_FILE", Hashsafe::new("Directory", &["/repo/src/"]), file_ref, ...)
```

- `resolve_ref_or_uuid()` supprimé entièrement (30 lignes)
- `link()` simplifié : `let from_ref: RefOrUuid = from.into();`
- Si on a besoin du Hashsafe plus loin dans le pipeline → migration vers variant enum (Option B)

### Fichiers modifiés
| Fichier | Changement |
|---------|-----------|
| `src/ops.rs` | +Hashsafe struct, +Hashsafe::new(), +From<Hashsafe> for RefOrUuid |
| `src/lib.rs` | +export Hashsafe |
| `src/catalog.rs` | -resolve_ref_or_uuid() (supprimé), link() simplifié |
| `tests/e2e_phase0b.rs` | 2 call sites mis à jour vers Hashsafe::new() |

---

## 2. Fix update() contentFor-only (Bug C bis)

### Problème

Le test `phase0b_update_content_for_only` échouait : `drain after update: processed=0`. Grâce aux events pub/sub, on voit `status: Unchanged, reembedded: false` — `content_changed` est `false`.

### Diagnostic

La requête `update()` utilisait :
```cypher
MATCH (n:File {_uuid: $uuid})
WITH n, n._content_hash AS old_hash
SET n.name = $name, n._content_hash = $new_hash
RETURN old_hash
```

**Kuzu retourne `old_hash` avec la valeur POST-SET**, pas pre-SET. Le `WITH` capture une référence au noeud, pas un snapshot de la valeur. Résultat : `old_hash == new_hash` toujours → `content_changed = false` → 0 AggregateOps enqueués.

### Fix

Séparer en 2 queries :
```rust
// 1. Lire le old_hash AVANT le SET
let old_hash_query = format!(
    "MATCH (n:{entity_name} {{_uuid: $uuid}}) RETURN n._content_hash"
);
let old_result = self.conn.execute_with_params(&old_hash_query, ...).await?;
let old_hash = old_result.rows[0].get(0).and_then(|v| v.as_str()).unwrap_or("");

// 2. Appliquer le SET
let set_cypher = format!(
    "MATCH (n:{entity_name} {{_uuid: $uuid}}) SET {set_parts}"
);
self.conn.execute_with_params(&set_cypher, &params).await?;
```

### Résultat après fix
- `status: Updated, reembedded: true`
- `opi_23: aggregate FileKB` (File est titleFor)
- `opi_24: aggregate TreeKB` (File est contentFor, trouve Directory via HAS_FILE)
- `processed=15` : re-aggregate + re-chunk + re-embed pour les 2 KBs
- TreeKB_Index `_content` passe de `auth.ts` à `login.ts`

### Fichiers modifiés
| Fichier | Changement |
|---------|-----------|
| `src/catalog.rs` | update() : 2 queries séparées au lieu de WITH...SET...RETURN |

---

## 3. Refonte `run_e2e.sh`

### Problème

Le script rebuildit cmake à chaque invocation, même quand rien n'a changé côté C++. De plus, la feature `cuda` (candle+CUDA) rallonge la compilation cargo même si les tests n'utilisent que MockEmbedder.

### Changements

- **Default = skip build** si `librag3db.so` existe déjà
- **`--build`** pour forcer un rebuild (quand on change du C++ rag3db)
- **`--no-build`** gardé pour compat (maintenant le comportement par défaut)
- **`--no-cuda`** pour compiler sans CUDA (plus rapide pour tests pipeline)
- **`geo` ajouté** aux extensions : `vector;tantivy_fts;sparse_vector;geo`
- Build cmake reconfigured from scratch (ancien supprimé pour ajouter geo)

### Usage
```bash
./run_e2e.sh --test e2e_phase0b           # skip build, ~6s
./run_e2e.sh --build --test e2e_phase0b   # force rebuild C++ + tests
./run_e2e.sh --no-cuda --test e2e_phase0b # sans CUDA, compile plus vite
./run_e2e.sh --build-only                 # juste le build C++
```

### Fichiers modifiés
| Fichier | Changement |
|---------|-----------|
| `run_e2e.sh` | Logique inversée (skip build par défaut), +--build, +--no-cuda, +geo |

---

## 4. Résultats tests

```
test phase0b_ingest_and_schema ................... ok
test phase0b_content_offset_arithmetic ........... ok
test phase0b_title_truncation .................... ok
test phase0b_bm25_search_multi_entity ............ ok
test phase0b_bm25_highlight_chunk_single_entity .. ok
test phase0b_link_incremental_aggregate .......... ok
test phase0b_aggregate_skip_unchanged ............ ok
test phase0b_delete_content_for_only ............. ok
test phase0b_delete_one_of_multiple_files ........ ok
test phase0b_update_content_for_only ............. ok   ← NOUVEAU FIX
test phase0b_sourced_rels_multi_entity ........... ok
test phase0b_vector_chunk_to_source_entity ....... ok
test phase0b_debug_trace_pipeline ................ ok
test phase0b_tantivy_contains_vs_parse ........... ok

test result: ok. 14 passed; 0 failed
```

---

## Concessions / points d'attention

1. **update() fait 2 queries au lieu de 1** : une pour lire `_content_hash`, une pour le SET. Légèrement moins performant mais correct. Si Kuzu supporte un jour les snapshots dans WITH (comme Neo4j), on pourrait revenir à une seule query.

2. **build_content_text au update est partiel** : il ne hash que les champs présents dans `data`, pas tous les champs de l'entité. Si on update un champ non-texte (ex: `lines_of_code`), le hash sera différent du hash original (qui incluait tous les champs texte) → `content_changed = true` faux positif → re-aggregate inutile. Optimisation future : lire l'entité complète pour recalculer le hash, ou stocker les hash par-champ.

3. **`Hashsafe` struct vs variant enum** : on a choisi l'option A (struct externe + From, résolution immédiate). Si on a besoin de garder le Hashsafe non-résolu dans le pipeline (ex: pour du lazy resolution ou du logging), migration vers Option B (variant dans RefOrUuid).

---

## Prochaines étapes

1. **Non-régression** : `./run_e2e.sh` (e2e_search) pour vérifier les tests existants
2. **Phase 1** : Code Domain schema + CRUD E2E test (task #95)
