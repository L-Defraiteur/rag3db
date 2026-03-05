# Rag3Weaver — Progression L3 (15 fevrier 2026)

Date : 15 fevrier 2026
Statut : En cours

---

## Etat actuel : 159 tests, 12 modules

### Modules termines (L3a complet)

| Module | Tests | Source TS | Notes |
|--------|:-----:|-----------|-------|
| `filter.rs` | 28 | `l3/FilterParser.ts` (267 loc) | **NOUVEAU** — FilterParser, FilterOp (10 variantes), FilterValue, ParsedFilter, cross-entity, list ops Kuzu |
| `validator.rs` | 11 | `l3/SchemaValidator.ts` (180 loc) | **NOUVEAU** — validate_schema(), 2 passes (collect KBs + validate), erreurs/warnings |
| `events.rs` | 5 | `l3/EventEmitter.ts` | Fait (etape 0) |
| `config.rs` | 11 | `catalog/types.ts` | Fait (etape 0) |
| `embedder.rs` | 5 | — | Fait (etape 0) |
| `connection.rs` | 14 | — | Fait (etape 0) |
| `schema.rs` | 22 | `catalog/CatalogSchema.ts` | Fait (etape 1) |
| `query.rs` | 17 | — | Fait (etape 1) |
| `hash.rs` | 4 | `catalog/CatalogUtils.ts` | Fait (etape 1) |
| `uuid.rs` | 10 | `catalog/CatalogUtils.ts` | Fait (etape 1) |
| `chunker.rs` | 21 | `l3/SemanticChunker.ts` | Fait (etape 1 + text-splitter) |
| `fusion.rs` | 11 | `catalog/CatalogSearch.ts` | Fait (etape 1) |

### Bilan tests

```
cargo test → 159 passed, 0 failed
```

---

## L3a — Detail des deux nouveaux modules

### filter.rs (28 tests)

Port fidele de `l3/FilterParser.ts`. Types et API :

- `FilterOp` — 10 variantes : Eq, Neq, Lt, Lte, Gt, Gte, In, HasAny, HasAll, HasNone
- `FilterValue` — 3 variantes : Direct(CypherValue), List(Vec), Ops(Vec<FilterOp>)
  - From impls pour &str, i64, f64, bool, Vec<CypherValue>
- `ParsedFilter` — where_clauses, match_clauses, params, aliases, combine_where()
- `FilterParser<'a>` — new(relations), parse(filters, entity, alias)
- `FilterError` — InvalidIdentifier, NoRelation
- `is_valid_identifier()` — pub, reutilisable par catalog.rs

Fonctionnalites :
- Egalite directe, null (IS NULL), arrays (IN), 6 operateurs de comparaison
- Cross-entity via notation `"Entity.field"` → detection bidirectionnelle de relation → MATCH clause
- List ops Kuzu : `list_any_match`, `list_all`, `NOT list_any_match`
- Parametrage sequentiel `filter_p0..pN`, reset entre appels
- Validation d'identifiants (injection prevention)
- Reutilisation d'alias quand plusieurs filtres sur la meme entite cross

### validator.rs (11 tests)

Port fidele de `l3/SchemaValidator.ts`. Types et API :

- `KBFieldRef` — { entity, field }
- `KBValidation` — { title, content, entities }
- `ValidationResult` — { valid, errors, warnings, knowledge_bases }
- `validate_schema(config) -> ValidationResult`

Regles :
- Phase 1 : `collect_kbs()` scanne toutes entites/champs → map de KBs
- Phase 2 : `validate_kb()` par KB :
  - Exactement 1 titleFor par KB (erreur si 0 ou 2+)
  - Au moins 1 contentFor (warning si absent)
  - Multi-entite → relation bidirectionnelle requise (erreur sinon)

Difference avec `schema.rs::resolve_entity_kbs()` : celui-ci est per-entity (pour DDL), le validateur est cross-entity (pour validation globale).

---

## L3b — En cours (refs.rs + ops.rs + queue.rs)

### Cargo.toml modifie

Ajout de `tokio = { version = "1", features = ["sync"] }` en dependance principale (pas juste dev) pour `tokio::sync::watch` dans refs.rs. Lightweight, WASM-compatible (pas de runtime, juste primitives sync).

### refs.rs — Design decide, code pas encore ecrit

Source TS : `l3/Ref.ts` (292 lignes)

**Pattern** : `(EntityRef, EntityRefResolver)` — consumer/producer split via `tokio::sync::watch`.

- `EntityRef` : Clone (watch::Receiver), entity name, temp_uuid
  - `uuid()` sync → Result<String, RefError>
  - `ready()` async → attend resolution via watch channel
  - `is_ready()` sync → bool
  - `temp_uuid()` → UUID temporaire (blake3 + atomic counter, pas de crate uuid)
- `EntityRefResolver` : consomme sur resolve/fail (watch::Sender)
  - `resolve(uuid)` → envoie Ready(uuid)
  - `fail(error)` → envoie Failed(error)
- `RelationRef` + `RelationRefResolver` : meme pattern, resout en `RelResolved { from_uuid, to_uuid }`
- `RefError` : Pending, Failed
- Temp UUID : blake3 hash d'un compteur atomique, formate en UUID

~12 tests prevus.

### ops.rs — Design decide, code pas encore ecrit

Source TS : `catalog/CatalogQueueItems.ts` (247 lignes)

- `CatalogOp` enum : Insert(InsertOp), Embed(EmbedOp), Link(LinkOp)
- `InsertOp` : entity_name, data (HashMap), resolver (EntityRefResolver)
- `EmbedOp` : entity_ref (EntityRef clone), kb_name, texts (rempli par pipeline)
- `LinkOp` : rel_name, from (RefOrUuid), to (RefOrUuid), properties, resolver (RelationRefResolver)
- `RefOrUuid` : Ref(EntityRef) | Uuid(String), avec try_resolve() sync et resolve() async
- Priorites : INSERT=1, LINK=2, EMBED=3
- From impls : EntityRef → RefOrUuid, String → RefOrUuid, &str → RefOrUuid

~5 tests prevus.

### queue.rs — Design decide, code pas encore ecrit

Source TS : `queue/GenericOperationQueue.ts` (452 lignes) — v1 simplifiee

- `OperationQueue` : Vec<CatalogOp> + FlushConfig + QueueStats
- `enqueue(op)` → push + increment counter
- `drain_sorted()` → sort by priority, drain all
- `drain_up_to(max_priority)` → drain seulement priority <= max, reste en queue
- `pending_count()`, `is_empty()`, `clear()`
- `QueueStats` : total_enqueued, total_drained

**Pas en v1** : auto-flush (timers), persistence (_Operation table), state machine des items, crash recovery, processor registration. Tout ca vient incrementalement.

~8 tests prevus.

---

## Sources TS lues et validees

Les fichiers TS sources sont dans :
- `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/src/lib/l3/` (5 modules, ~1200 loc)
- `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/src/lib/catalog/modules/` (6 modules, ~3300 loc)
- `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/src/queue/` (5 modules utiles, ~2000 loc)

**IMPORTANT** : Le fichier `rag3weaver-l3.ts` (ancien rename d'un JS) a ete deplace par l'utilisateur et ne doit PAS etre utilise comme reference.

---

## Suite apres L3b

| Sous-etape | Modules | Deps | Tests estimes |
|------------|---------|------|:---:|
| **L3b** (en cours) | `refs.rs`, `ops.rs`, `queue.rs` | tokio sync | ~25 |
| **L3c** | `pipeline.rs`, `catalog.rs`, `persistence.rs` | MockConnection + MockEmbedder | ~33 |
| **L3d** | `search.rs`, `explore.rs` | DB + embedder + lucivy (via Cypher) | ~18 |

Ordre strict : L3b → L3c → L3d (chaque couche depend de la precedente).

Total estime apres L3 complet : **~260 tests**.

---

## Fichiers modifies dans cette session

| Fichier | Action |
|---------|--------|
| `src/filter.rs` | Cree (28 tests) |
| `src/validator.rs` | Cree (11 tests) |
| `src/lib.rs` | Modifie (ajout modules + re-exports) |
| `Cargo.toml` | Modifie (ajout tokio sync en dep) |
