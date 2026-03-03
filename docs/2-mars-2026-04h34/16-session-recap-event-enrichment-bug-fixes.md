# Doc 16 — Session Recap: Event Enrichment + Bug Fixes (A/B/C)

**Date**: 3 mars 2026
**Branche**: `feature/kb-index-architecture`

---

## Résumé

Enrichissement des QueueEvents avec `OpSummary` pour le debug pipeline, puis correction de 3 bugs (B: vector index name, C: update contentFor-only, A: link() hashsafe lookup). Le bug A s'est avéré être un problème de résolution UUID dans `link()`, pas un problème Tantivy.

---

## 1. Enrichissement QueueEvents (OpSummary)

### Motivation
Les QueueEvents ne portaient que des IDs opaques (`opi_1`, `opi_2`). Pour débugger le pipeline, il fallait corréler manuellement les IDs avec les opérations. La user a demandé que les events donnent accès aux données qu'ils ont traités.

### Implémentation

**Nouveau struct `OpSummary`** (`ops.rs`) :
```rust
pub struct OpSummary {
    pub id: String,           // "opi_3"
    pub op_type: &'static str, // "insert", "aggregate", etc.
    pub priority: OrderedPriority,
    pub target: String,        // entity/KB/rel name
    pub detail: String,        // UUIDs, from→to, etc.
}
```

**Méthode `CatalogOp::summary(&self, id: &str) -> OpSummary`** — génère un résumé lisible par variant :
- Insert → `target=entity_name, detail=uuid`
- Link → `target=rel_name, detail="from_uuid → to_uuid"`
- Aggregate → `target=kb_name, detail="title_entity:source_uuid idx=index_uuid"`
- Embed/SparseEmbed/DualEmbed → `target=kb_name, detail="ref=temp_uuid texts=N"`

**QueueEvent modifié** (`queue.rs`) :
- `Enqueued { summary: OpSummary }` (avant: `id, op_type, priority`)
- `ProcessingBatch { items: Vec<OpSummary> }` (avant: `Vec<String>`)
- `BatchCompleted { items: Vec<OpSummary> }` (avant: `Vec<String>`)
- `BatchFailed { items: Vec<OpSummary> }` (avant: `Vec<String>`)
- `Injected { ops: Vec<OpSummary> }` (avant: `Vec<(&str, OrderedPriority)>`)

Les summaries sont construites au moment de l'émission (pas lazy).

### Fichiers modifiés
| Fichier | Changement |
|---------|-----------|
| `src/ops.rs` | +OpSummary struct, +CatalogOp::summary(), +Display impl |
| `src/queue.rs` | QueueEvent variants portent OpSummary, emit calls mis à jour |
| `src/lib.rs` | +export OpSummary |

---

## 2. Fix Bug B : Vector index name mismatch

### Problème
Schema crée `{kb_name}_Index_Chunk_vec` mais search construit `{entity}_{kb_name}_vec` où `entity` est déjà `{kb_name}_Index_Chunk`. Résultat : `TreeKB_Index_Chunk_TreeKB_vec` au lieu de `TreeKB_Index_Chunk_vec`.

### Fix (`search.rs`)
```rust
// AVANT (search_vector_hnsw + search_vector_hnsw_filtered)
let index_name = format!("{entity}_{kb_name}_vec");

// APRÈS
let index_name = format!("{entity}_vec");
```
Paramètre `kb_name` renommé en `_kb_name` (plus utilisé dans ces fonctions).

---

## 3. Fix Bug C : update() contentFor-only silent error

### Problème
`update()` utilisait `if let Ok(...)` pour la requête de résolution des title entities liées (contentFor-only). Si la query échouait, l'erreur était avalée silencieusement → 0 AggregateOps enqueués → `drain()` traite 0 ops.

### Fix (`catalog.rs:~732`)
```rust
// AVANT — erreur silencieuse
if let Ok(title_results) = self.conn.execute_with_params(&query, &params).await {

// APRÈS — propagation d'erreur (comme delete() fait déjà)
let title_results = self.conn.execute_with_params(&query, &params).await
    .map_err(|e| CatalogError::DbError(e.to_string()))?;
```

---

## 4. Fix Bug A : link() hashsafe lookup string → UUID incorrect

### Diagnostic (grâce aux OpSummary enrichis !)

Le test `phase0b_bm25_search_multi_entity` appelait :
```rust
catalog.link("HAS_FILE", "Directory:/repo/src/", file_ref, ...)
```

La string `"Directory:/repo/src/"` est un **hashsafe lookup string**, pas un UUID. Deux conséquences :

1. **LinkProcessor** : `MATCH (a {_uuid: "Directory:/repo/src/"})` ne matche rien car le vrai `_uuid` est `56cd25ce-...` → **le lien HAS_FILE n'est jamais créé en DB**
2. **AggregateOp** : `source_uuid = "Directory:/repo/src/"` produit un `index_entry_uuid` différent de celui créé par `create()` → **AggregateProcessor ne retrouve pas l'index entry**

Résultat : TreeKB_Index ne contenait que les données du Directory (`_title="src"`, `_content="/repo/src/"`) sans aucun contenu File. Parse ET Contains retournaient 0 pour "auth" — le bug n'était pas Tantivy du tout.

Les events enrichis ont rendu ça évident :
```
opi_9:  link HAS_FILE "Directory:/repo/src/" → pending    ← lookup string, pas UUID !
opi_10: aggregate TreeKB Directory:Directory:/repo/src/    ← double préfixe = UUID erroné
```

### Fix (`catalog.rs`)

**Nouvelle méthode `resolve_ref_or_uuid()`** :
```rust
fn resolve_ref_or_uuid(&self, r: RefOrUuid, entity_name: &str) -> RefOrUuid {
    if let RefOrUuid::Uuid(ref s) = r {
        let prefix = format!("{entity_name}:");
        if s.starts_with(&prefix) {
            let values_str = &s[prefix.len()..];
            let entity_def = self.config.entities.get(entity_name)?;
            let hashsafe_fields = entity_def.hashsafe.as_ref()?;
            let field_values: Vec<&str> = if hashsafe_fields.len() == 1 {
                vec![values_str]
            } else {
                values_str.splitn(hashsafe_fields.len(), ':').collect()
            };
            return RefOrUuid::Uuid(hashsafe_uuid(entity_name, &field_values));
        }
    }
    r
}
```

Appelée dans `link()` :
```rust
let from_ref = self.resolve_ref_or_uuid(from.into(), &from_entity);
let to_ref = self.resolve_ref_or_uuid(to.into(), &to_entity);
```

### Résultat après fix
- `opi_9: link HAS_FILE "56cd25ce-..." → pending` ✓ (vrai UUID)
- `opi_10: aggregate TreeKB Directory:56cd25ce-... idx=19512fd6-...` ✓ (même index que opi_4)
- TreeKB_Index `_content` = `"/repo/src/\n/repo/src/auth.ts\nauth.ts"` ✓ (File content inclus)
- Tantivy Contains "auth" → **1 résultat** ✓
- Tantivy Parse "auth" → **1 résultat** ✓
- `processed=23` (avant: 17) — les 6 ops supplémentaires sont les chunks/links File

---

## 5. Test isolé ajouté

**`phase0b_tantivy_contains_vs_parse`** (test 14) :
- Souscrit aux queue events
- Crée Directory + File + link avec lookup string
- Dump TreeKB_Index et TreeKB_Index_Chunk
- Teste 8 variantes de queries Tantivy raw (Parse/Contains, single/multi field, distance 0/1)
- Diagnostic tool réutilisable pour isoler les problèmes Tantivy vs pipeline

---

## Concessions / points d'attention

1. **`resolve_ref_or_uuid` est heuristique** : il détecte les lookup strings par le format `"EntityName:..."`. Si un vrai UUID contenait `:` ça poserait problème — mais les UUIDs hashsafe sont en hex+tirets, jamais de `:`.

2. **Multi-field hashsafe avec `:` dans les valeurs** : pour `hashsafe: [a, b]` avec `"Entity:val1:val2"`, on split par `:` avec `splitn(N)`. Si `val1` contient `:`, le split est incorrect. Risque faible (les hashsafe fields sont typiquement des paths ou noms), mais à garder en tête.

3. **Le test `phase0b_bm25_search_multi_entity` n'a pas encore été relancé** après le fix — il devrait passer maintenant que le contenu File apparaît dans TreeKB_Index.

4. **Les 13 tests n'ont pas été relancés en bloc** — prochaine étape : `./run_e2e.sh --test e2e_phase0b` pour vérifier le score global.

---

## Fichiers modifiés (cette session)

| Fichier | Changement |
|---------|-----------|
| `src/ops.rs` | +OpSummary, +CatalogOp::summary(), +Display |
| `src/queue.rs` | QueueEvent enrichis avec OpSummary |
| `src/lib.rs` | +export OpSummary |
| `src/search.rs` | Fix vector index name: `{entity}_vec` (pas `{entity}_{kb}_vec`) |
| `src/catalog.rs` | Fix update() error propagation, +resolve_ref_or_uuid(), link() résout lookups |
| `tests/e2e_phase0b.rs` | +test 14 `phase0b_tantivy_contains_vs_parse` |

---

## Prochaines étapes

1. **Relancer les 13+1 tests** : `./run_e2e.sh --test e2e_phase0b` → objectif 14/14
2. **Vérifier tests existants** : `./run_e2e.sh` (e2e_search) pour non-régression
3. Si tout vert → Phase 1 (Code Domain schema + CRUD)
