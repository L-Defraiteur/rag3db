# Doc 18 — Audit des optimisations : branche `feature/kb-index-architecture`

**Date** : 3 mars 2026
**Branche** : `feature/kb-index-architecture` (3 commits, +7989/-544 lignes vs `master`)

---

## Contexte

Audit de l'ensemble des changements introduits par la branche pour identifier les optimisations perdues, les régressions de performance, et les pistes d'amélioration. La branche introduit :

- **Phase 0a** : `OrderedPriority(f32)` — priorités fines pour le pipeline
- **Phase 0b** : Architecture KB Index — tables `{KB}_Index` + `{KB}_Index_Chunk` partagées
- **Dernier commit** : Hashsafe struct, fix update(), optimisation build script

### Fichiers modifiés (hors docs/tests)

| Fichier | Lignes +/- | Nature |
|---------|-----------|--------|
| `catalog.rs` | +976/-? | AggregateProcessor, create/update/delete propagation, Hashsafe |
| `ops.rs` | +288 | OrderedPriority, AggregateOp, Hashsafe, OpSummary, priority_override |
| `schema.rs` | +795/-544 | Refonte complète : entity pure data, KB Index tables |
| `queue.rs` | +82/-? | QueueEvent enrichi avec OpSummary, BTreeMap<OrderedPriority> |
| `search.rs` | +44/-? | _content_offset, convention {entity}_vec |
| `config.rs` | +10 | Champs additionnels |
| `persistence.rs` | +2/-1 | priority f32 |
| `cypher_persistence.rs` | +33/-? | CypherValue::Double pour priority |

---

## 1. Optimisations perdues

### 1.1 Filter fields natifs Lucivy (IMPACT FAIBLE — fallback `allowed_ids` existant)

**Avant** : les champs non-texte des entités (INT64, DOUBLE, STRING, BOOLEAN) étaient indexés comme filter fields dans Lucivy directement sur la table entité. Exemple :

```
CREATE_LUCIVY_INDEX('Document', ['title', 'body'],
    filter_fields=['page_count', 'status', 'published'])
```

Le filtrage `WHERE page_count > 10` était résolu au niveau segment Lucivy (pré-filtre natif, pas de post-filter).

**Après** : le FTS est sur `{KB}_Index` avec uniquement `_source_entity` comme filter field :

```
CREATE_LUCIVY_INDEX('main_Index', ['_title', '_content'],
    filter_fields=['_source_entity'])
```

**Impact réel** : le `FilterCompiler::split()` dans `filter.rs` sépare déjà les filtres en deux chemins :
- **Lucivy-native** : ops simples (Eq, Lt, Gt, In, Between...) → `FilterClause` JSON
- **Kuzu fallback** : ops complexes, cross-entity, IS NULL → pré-résolution en `allowed_ids` via Cypher `RETURN OFFSET(id(n))`

Le chemin `allowed_ids` fonctionne correctement comme fallback pour tous les filtres. La perte est uniquement de performance : 2 round-trips (query Kuzu pour IDs + query Lucivy avec allowed_ids) au lieu de 1 (Lucivy natif).

**Attention** : le `FilterCompiler::split()` ne vérifie PAS si les champs existent dans l'index Lucivy. Si un filtre `page_count > 10` est classé "Lucivy-compatible" par `is_field_lucivy()`, il sera envoyé à Lucivy qui ne connaît pas ce champ sur `{KB}_Index`. Bug potentiel à corriger : soit ajouter une vérification des champs disponibles dans le split, soit forcer tous les filtres non-`_source_entity` vers le chemin `kuzu`.

**Priorité** : basse — pas de use case actuel avec filtres non-texte sur la nouvelle archi, et fix simple si nécessaire.

### 1.2 update() fait 2 queries au lieu de 1

**Avant** :
```cypher
MATCH (n:File {_uuid: $uuid})
WITH n, n._content_hash AS old_hash
SET n.name = $name, n._content_hash = $new_hash
RETURN old_hash
```
Un seul round-trip.

**Après** :
```rust
// Query 1: lire le old_hash
MATCH (n:File {_uuid: $uuid}) RETURN n._content_hash

// Query 2: appliquer le SET
MATCH (n:File {_uuid: $uuid}) SET n.name = $name, n._content_hash = $new_hash
```
Deux round-trips.

**Cause** : Kuzu retourne la valeur POST-SET dans `RETURN old_hash` (le `WITH` capture une référence au noeud, pas un snapshot de la valeur). Comportement différent de Neo4j.

**Impact** : +1 round-trip par update. Négligeable pour des updates isolés, mais significatif si on batch des milliers d'updates.

**Pistes** :
1. Attendre un fix Kuzu (support des snapshots dans WITH)
2. Stored procedure / extension C++ qui fait les deux opérations atomiquement
3. Colonne `_prev_content_hash` : le SET écrit le nouveau hash ET garde l'ancien dans une colonne dédiée → 1 seule query, compare après coup

### 1.3 build_content_text partial hash (faux positifs)

**Problème** : à l'`update()`, le hash est calculé uniquement sur les champs présents dans `data`, pas sur tous les champs texte de l'entité.

Exemple : si on update `lines_of_code` (INT64) sur un File qui a `name` et `content` comme champs texte :
- Le hash original a été calculé sur `name + content`
- Le nouveau hash est calculé sur `""` (aucun champ texte dans `data`)
- `content_changed = true` → re-aggregate inutile (faux positif)

**Impact** : re-aggregate + re-chunk + re-embed pour rien. Coûteux en GPU si embeddings activés.

**Pistes** :
1. **Lire l'entité complète** avant de hasher : `MATCH (n:File {_uuid: $uuid}) RETURN n` → extraire tous les champs texte, merger avec `data`, puis hasher. +1 query mais élimine tous les faux positifs.
2. **Stocker des hashes par-champ** : `_content_hash_name`, `_content_hash_content`, etc. Comparer champ par champ. Plus précis mais plus de colonnes système.
3. **Hash uniquement si au moins un champ texte/content_for est dans `data`** : skip le re-hash si aucun champ contribuant à un KB n'a changé. Simple, couvre 90% des cas.

---

## 2. Régressions de performance structurelles

### 2.1 AggregateProcessor : N+2 queries par rebuild

Pour un rebuild d'une entry `TreeKB_Index` avec 2 entities (Directory, File) :

| Step | Query | Round-trips |
|------|-------|-------------|
| 1. Lire _content_hash de l'index entry | `MATCH (idx:TreeKB_Index {_uuid:...}) RETURN idx._content_hash` | 1 |
| 2. Lire le title text du Directory | `MATCH (d:Directory {_uuid:...}) RETURN d.name` | 1 |
| 3. Lire les content fields du Directory | `MATCH (d:Directory)-[:Directory_IN_TreeKB]->(idx) RETURN d._uuid, d.name, d.absolute_path` | 1 |
| 4. Lire les content fields du File | `MATCH (f:File)-[:File_SOURCED_TreeKB]->(c) WHERE c._parent_uuid = $idx_uuid ...` (ou via rel) | 1 |
| 5. UPDATE l'index entry | `SET idx._title, idx._content, idx._content_hash` | 1 |
| 6. DELETE old chunks | `MATCH (c:TreeKB_Index_Chunk {_parent_uuid:...}) DETACH DELETE c` | 1 |
| **Total avant downstream** | | **6** |
| 7. Downstream ops (inserts + links + embeds) | Via injection dans la queue | N |

**Pistes** :
1. **Batch les content queries** : une seule query UNION ALL pour toutes les entities contribuantes, au lieu de 1 query par entity
2. **Cache le hash en mémoire pendant le drain** : évite le step 1 (lire `_content_hash`). Le AggregateProcessor peut maintenir un `HashMap<String, String>` des hashes déjà lus pendant un drain
3. **Fusionner steps 5+6** : `MATCH (idx:...) OPTIONAL MATCH (idx)-[:HAS_CHUNK]->(c) DETACH DELETE c SET idx._title = ...` — potentiellement 1 query au lieu de 2
4. **Paralléliser les rebuilds** : les AggregateOps indépendants (index entries différents) peuvent être traités en parallèle via `tokio::join!`

### 2.2 Pas de vector index sur `{KB}_Index` (document-level)

**Schéma actuel** : HNSW index uniquement sur `{KB}_Index_Chunk`, pas sur `{KB}_Index`.

**Impact** : pour les KBs sans champs `chunked` (TreeKB, LibraryKB), le vector search passe par les chunks qui sont des "mini-chunks" d'un seul field court. Fonctionne mais :
- Pas de document-level vector search (sans chunk resolution)
- Pour des contenus courts (nom + path), chunker produit 1 chunk = 1 entry, donc pas de gain par rapport à indexer directement sur `{KB}_Index`

**Piste** : ajouter un HNSW index sur `{KB}_Index` pour le cas document-level. Router automatiquement : contenu court → Index direct, contenu long → Chunks. Le `search()` peut décider selon `kb_meta.chunking` et la taille du content.

---

## 3. Gains obtenus

### 3.1 Entity tables allégées

Les tables entités n'ont plus de colonnes embedding (`FLOAT[384]`, `sparse_indices`, `sparse_weights`). Gains :
- Scans plus rapides pour les queries non-search (CRUD, traversals)
- Moins de colonnes NULL pour les entités sans certains KBs
- Structure plus propre (séparation données / index de recherche)

### 3.2 Cross-entity search unifié (IDF cohérent)

Avant : impossible d'avoir un BM25 cohérent entre Directory et File (index séparés = IDF incomparables).
Après : `TreeKB_Index` contient les deux → un seul index FTS, un seul IDF, scores comparables.

### 3.3 Pipeline idempotent et observable

- `AggregateProcessor` est idempotent : query le graph pour l'état actuel, rebuild from scratch
- `OpSummary` sur chaque `QueueEvent` → debug complet du pipeline sans accès aux `CatalogOp`
- Déduplication des AggregateOps par `index_entry_uuid` : 100 links vers le même Directory = 1 seul rebuild

### 3.4 Priorités fines

`OrderedPriority(f32)` permet : chunk(0.0) → insert(1.0) → link(2.0) → aggregate(2.5) → post-agg insert(2.6) → post-agg link(2.7) → embed(3.0). Plus de collision entre étapes du pipeline.

---

## 4. Résumé des pistes d'optimisation

| # | Optimisation | Complexité | Gain estimé | Priorité |
|---|---|---|---|---|
| 1 | **build_content_text : skip si pas de champ texte dans data** | Faible | Élimine ~90% des faux positifs re-aggregate | Haute (quick win) |
| 2 | **Fix FilterCompiler : forcer champs non-index vers kuzu** | Faible | Évite crash si filtre sur champs entité | Haute (bug potentiel) |
| 3 | **Batch content queries dans AggregateProcessor** | Moyenne | -50% round-trips par rebuild multi-entity | Moyenne |
| 4 | **Cache hash en mémoire pendant drain** | Faible | -1 query par AggregateOp | Moyenne |
| 5 | **Vector index dual (Index + Chunk)** | Moyenne | Meilleur vector search document-level | Moyenne |
| 6 | **AggregateProcessor parallèle** (tokio::join) | Faible | Proportionnel au nombre de KBs × entries | Basse |
| 7 | **update() atomic** (extension Kuzu / stored proc) | Haute | -1 round-trip par update | Basse (gain marginal) |
| 8 | **Fusionner UPDATE + DELETE chunks** en 1 query | Faible | -1 query par rebuild | Basse |

---

## 5. Recommandations

**Court terme** (avant Phase 1 Code Domain) :
- Implémenter **#1** (skip re-hash si pas de champ texte dans data) — 10 lignes, élimine les re-aggregates parasites
- Implémenter **#2** (fix FilterCompiler) — ajouter la liste des champs disponibles dans l'index au `split()`, forcer les champs inconnus vers le chemin `kuzu` (allowed_ids). Évite un crash si un utilisateur filtre par champ entité.
- Implémenter **#4** (cache hash mémoire) — simple HashMap dans AggregateProcessor

**Moyen terme** (pendant/après Phase 1) :
- **#5** (vector index dual) quand on implémente ScopeKB avec `content: chunked`
- **#3** (batch content queries) si les rebuilds multi-entity deviennent un goulot

**Long terme** :
- **#6** devient important à l'échelle (milliers d'entités par KB)
- **#7** dépend de l'évolution de Kuzu (WITH snapshot semantics)
- Propager filter fields vers `{KB}_Index` si les perf de filtrage deviennent un problème (colonnes mirror + re-indexation Lucivy). Pas prioritaire tant que `allowed_ids` suffit.
