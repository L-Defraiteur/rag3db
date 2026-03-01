# 05 — Session : SearchSignals bitflags + EmbedOp conditionnel + design fusion per-signal

## Ce qui a été fait

### SearchSignals — système de flags binaires

Remplacement du système `SearchMode` enum + `sparse: bool` par des flags composables :

```rust
pub struct SearchSignals(u8);

const BM25    = 0b001;
const VECTOR  = 0b010;
const SPARSE  = 0b100;

// Combinaison libre
let signals = SearchSignals::BM25 | SearchSignals::SPARSE;
signals.bm25()   // true
signals.vector() // false
signals.sparse() // true
```

Aliases de commodité : `FULLTEXT = BM25`, `SEMANTIC = VECTOR`, `HYBRID = BM25|VECTOR`.

Serde : serialize → `["bm25","vector","sparse"]`, deserialize ← `["bm25","sparse"]` (accepte aussi `"fulltext"`, `"semantic"`, `"dense"` comme synonymes).

### Fichiers modifiés

| Fichier | Changement |
|---------|-----------|
| `src/search.rs` | `SearchSignals` struct + `Serialize`/`Deserialize` + `Debug` + `BitOr`/`BitOrAssign`. `SearchType` enum supprimé. `SearchMeta.search_type` → `SearchMeta.signals`. `SearchOptions.signals: Option<SearchSignals>` ajouté (override per-query). |
| `src/config.rs` | `KBConfig.signals: Option<SearchSignals>` ajouté (JSON: `"signals": ["bm25","vector"]`). `KBConfig::signals()` méthode : explicit `signals` field > legacy `search` + `sparse`. `SearchMode` conservé pour rétrocompat serde. |
| `src/catalog.rs` | `Catalog::search()` : remplacé `match SearchType` 3 branches par `if signals.bm25()` / `if signals.vector()` / `if signals.sparse()`. `embed_query` conditionnel sur `signals.vector()`. `compute_chunk_ops` : `EmbedOp` conditionnel sur `kb_signals.vector()` (chunks créés sans embeddings en BM25-only). Sparse index creation : `kb_config.signals().sparse()` au lieu de `kb_config.sparse`. |
| `src/lib.rs` | Export `SearchSignals` au lieu de `SearchType`. |
| `tests/e2e_search.rs` | Import `SearchSignals`. 7 tests phase4 ajoutés. |

### EmbedOp conditionnel dans compute_chunk_ops

Avant : `EmbedOp` toujours créé pour chaque chunk (même en BM25-only → embeddings inutiles avec MockEmbedder).

Après : `EmbedOp` seulement si `kb_signals.vector()`. Les chunks (InsertOp + LinkOp) sont toujours créés — BM25 les utilise pour résoudre les highlights vers des positions de chunk.

### 7 tests phase4

Setup partagé : `setup_phase4_catalog(signals)` — crée config avec `KBConfig { signals: Some(signals) }`, BGE-M3 comme embedder/sparse_embedder selon les signaux, mêmes 3 docs, timing du drain.

| Test | Signals | Drain (GPU) |
|------|---------|:-----------:|
| `phase4_bm25_only` | `BM25` | **22ms** |
| `phase4_dense_only` | `VECTOR` | **210ms** |
| `phase4_sparse_only` | `SPARSE` | **314ms** |
| `phase4_bm25_vector` | `BM25\|VECTOR` | **371ms** |
| `phase4_bm25_sparse` | `BM25\|SPARSE` | **260ms** |
| `phase4_vector_sparse` | `VECTOR\|SPARSE` | **465ms** |
| `phase4_all_three` | `BM25\|VECTOR\|SPARSE` | **555ms** |

### État des tests

- `cargo check` ✓
- `cargo test --lib` ✓ 345 passed
- `phase4_bm25_only` ✓
- `phase4_dense_only` ✓
- `phase4_bm25_vector` ✓
- **4 tests sparse FAIL** : `No sparse vector index found on table 'Document_Chunk'`

### Bug en cours : schema.rs vérifie `kb_config.sparse` (legacy)

Les colonnes sparse (`{kb}_sparse_indices INT64[]`, `{kb}_sparse_weights DOUBLE[]`) ne sont créées dans le DDL que si `kb_config.sparse == true` (le champ legacy bool). Quand on utilise le nouveau `signals: Some(SearchSignals::SPARSE)`, le champ `sparse` reste `false` par défaut → pas de colonnes → pas d'index.

**Fix nécessaire** : `schema.rs` lignes 149 et 206 — remplacer `c.sparse` par `c.signals().sparse()`.

Même pattern que le fix fait dans `catalog.rs` pour la création d'index sparse.

## Ce qui reste à faire

### 1. Virer le comportement legacy complètement

Supprimer `SearchMode` enum, `KBConfig.search`, et `KBConfig.sparse` bool. Ne garder que `KBConfig.signals: SearchSignals` (plus optionnel → champ requis avec default `HYBRID`).

Fichiers impactés :
- `config.rs` : supprimer `SearchMode`, simplifier `KBConfig`
- `schema.rs` : utiliser `signals().sparse()` / `signals().vector()` partout
- `catalog.rs` : `KBMetadata.search` → remplacer ou supprimer
- `tests/` : adapter configs existantes (phases 0-3)
- Configs JSON externes : migration `"search": "hybrid"` → `"signals": ["bm25","vector"]`

### 2. Fix schema.rs pour sparse columns

`generate_node_table_ddl()` et `generate_chunk_table_ddl()` : `c.sparse` → `c.signals().sparse()`.

Idem pour la colonne embedding : pourrait devenir conditionnel sur `signals().vector()` (pas de colonne embedding inutile en BM25-only pur). Mais attention, ça casse le DDL si on passe de BM25-only à Hybrid plus tard (migration de schema).

### 3. Vérifier phases 0-3

Après le retrait du legacy, s'assurer que les tests existants (phase0 CRUD, phase1 BM25, phase2 vector, phase3 sparse hybrid) passent toujours.

---

## Design : Stratégies de fusion per-signal

### Problème avec le système actuel

`HybridStrategy` est un enum global appliqué à tous les signaux :

```rust
pub enum HybridStrategy {
    Boost,    // vector × (1 + bm25_norm × factor) — hardcodé vector=primary, BM25=booster
    RRF,      // rank-based, chaque liste pèse pareil
    Weighted, // (1-kw) × vector + kw × bm25 (+ sw × sparse en 3-way)
}
```

Limites :
- **Rôles hardcodés** : Boost suppose vector=primary, BM25=booster. Impossible d'inverser ou d'utiliser sparse comme booster.
- **RRF sans pondération** : chaque liste contribue `1/(k + rank)` — pas de moyen de donner plus de poids à un signal.
- **Boost + sparse = fallback RRF** : le code actuel tombe en RRF quand sparse est présent, car le boost 2-way n'est pas généralisé.
- **Poids implicites** : `keyword_weight` + `sparse_weight` + vector implicite `(1 - kw - sw)`. Pas de poids explicit par signal.

### Nouveau modèle : rôle per-signal

Chaque signal a un **rôle** :

| Rôle | Comportement |
|------|-------------|
| **`fuse`** (défaut) | Participe à la fusion principale. Combine les résultats avec les autres signaux `fuse` via la stratégie globale (RRF ou weighted). |
| **`boost`** | Appliqué **après** la fusion. Ne contribue pas de nouveaux candidats — re-ranke seulement les résultats existants. |

Un signal `boost` a un **boost_type** :

| Type | Formule | Quand l'utiliser |
|------|---------|-----------------|
| **`additive`** | `score += weight × normalized_signal_score` | Le signal apporte un bonus linéaire. Bon pour des signaux complémentaires (sparse booste un hybrid BM25+vector). |
| **`multiplicative`** | `score *= (1 + weight × normalized_signal_score)` | Le signal amplifie les bons résultats. Bon quand un résultat pertinent selon le booster devrait être beaucoup plus haut. |

La **stratégie de fusion globale** s'applique aux signaux `fuse` :

| Stratégie | Formule | Caractéristiques |
|-----------|---------|-----------------|
| **`rrf`** (défaut) | `Σ weight_i / (k + rank_i)` | Rank-based, robuste aux différences d'échelle. Les poids pondèrent la contribution RRF de chaque signal. |
| **`weighted`** | `Σ weight_i × normalized_score_i` | Score-based. Nécessite une bonne normalisation. Plus sensible aux scores réels. |

### Format de config JSON — 3 niveaux de complexité

Le champ `signals` accepte 3 formats, du plus simple au plus complet :

#### Niveau 1 : Array simple (compatibilité actuelle)

```json
{
  "signals": ["bm25", "vector", "sparse"]
}
```

Tous les signaux en rôle `fuse`, poids égaux, stratégie RRF. Equivalent au comportement actuel.

#### Niveau 2 : Map de poids

```json
{
  "signals": {
    "bm25": 0.3,
    "vector": 0.5,
    "sparse": 0.2
  }
}
```

Un nombre = rôle `fuse` avec ce poids. La stratégie globale est contrôlée par `fusion_strategy` (défaut `"rrf"`).

#### Niveau 3 : Config complète per-signal

```json
{
  "signals": {
    "bm25":   { "weight": 0.3 },
    "vector": { "weight": 0.5 },
    "sparse": { "weight": 0.2, "role": "boost", "boost_type": "multiplicative" }
  },
  "fusion_strategy": "rrf"
}
```

Chaque signal peut être un nombre (sucre pour `{ "weight": N }`) ou un objet complet.

### Champs per-signal

| Champ | Type | Défaut | Description |
|-------|------|--------|-------------|
| `weight` | `f64` | `1.0` | Poids du signal dans la fusion ou l'amplitude du boost |
| `role` | `"fuse"` \| `"boost"` | `"fuse"` | Participe à la fusion ou re-ranke après |
| `boost_type` | `"additive"` \| `"multiplicative"` | `"multiplicative"` | Type de boost (ignoré si role=fuse) |
| `normalize` | `"min-max"` \| `"none"` \| `"rank"` | auto | Stratégie de normalisation des scores avant fusion |
| `top_k` | `usize` | illimité | Limite le nombre de candidats du signal avant fusion |

### Normalisation des scores

Les signaux ont des échelles très différentes :
- **Vector (cosine)** : [0, 1] — borné
- **BM25** : [0, +∞) — non borné, dépend de la longueur du corpus
- **Sparse** : [0, +∞) — non borné

Pour que la fusion weighted ait du sens, il faut normaliser. Défauts intelligents :

| Signal | Normalisation par défaut |
|--------|------------------------|
| `vector` | `"none"` (déjà [0,1]) |
| `bm25` | `"min-max"` (normalize per-query vers [0,1]) |
| `sparse` | `"min-max"` |

Modes disponibles :
- **`min-max`** : `(score - min) / (max - min)` per-query → [0, 1]
- **`none`** : score brut
- **`rank`** : utilise le rang (position) au lieu du score — similaire à RRF mais dans un weighted context

Pour RRF la normalisation est sans effet (rank-based par nature).

### Top-K per signal

Limiter les candidats par signal avant fusion :

```json
{
  "signals": {
    "bm25":   { "weight": 0.3, "top_k": 100 },
    "vector": { "weight": 0.5, "top_k": 50 },
    "sparse": { "weight": 0.2, "top_k": 30 }
  }
}
```

Réduit le bruit : BM25 peut retourner des centaines de résultats faibles qui polluent la fusion.

### Exemples de configurations typiques

#### RAG classique — BM25 + vector, sparse en boost

```json
{
  "signals": {
    "bm25":   { "weight": 0.3 },
    "vector": { "weight": 0.7 },
    "sparse": { "weight": 0.15, "role": "boost", "boost_type": "multiplicative" }
  },
  "fusion_strategy": "rrf"
}
```

BM25 et vector fusionnés en RRF pondéré. Sparse amplifie les résultats qui matchent les termes domain-specific.

#### Code search — BM25 primary, vector en boost

```json
{
  "signals": {
    "bm25":   { "weight": 0.8 },
    "vector": { "weight": 0.3, "role": "boost", "boost_type": "additive" }
  },
  "fusion_strategy": "weighted"
}
```

BM25 fait le gros du travail (noms de fonctions, identifiants exacts). Vector ajoute un bonus sémantique aux résultats existants.

#### Semantic-first — vector seul, sparse en re-rank

```json
{
  "signals": {
    "vector": { "weight": 1.0 },
    "sparse": { "weight": 0.2, "role": "boost", "boost_type": "additive" }
  }
}
```

Vector retrieves, sparse booste les résultats qui matchent des termes rares (domain-specific vocabulary).

#### All-in RRF égal — simple et robuste

```json
{
  "signals": ["bm25", "vector", "sparse"]
}
```

Ou équivalent : `{ "bm25": 1.0, "vector": 1.0, "sparse": 1.0 }` en RRF.

### Algorithme de fusion — pseudo-code

```
fn fuse(results_per_signal, config):
    // 1. Séparer fuse vs boost
    fuse_signals  = [s for s in config.signals if s.role == "fuse"]
    boost_signals = [s for s in config.signals if s.role == "boost"]

    // 2. Appliquer top_k per signal
    for signal in all_signals:
        if signal.top_k:
            results[signal] = results[signal][:top_k]

    // 3. Fusionner les signaux "fuse"
    match config.fusion_strategy:
        RRF =>
            for each (signal, results) in fuse_signals:
                for (rank, result) in results.enumerate():
                    scores[result.id] += signal.weight / (k + rank + 1)

        Weighted =>
            for each (signal, results) in fuse_signals:
                normalized = normalize(results, signal.normalize)
                for result in normalized:
                    scores[result.id] += signal.weight * result.score

    // 4. Appliquer les boosts
    for (signal, results) in boost_signals:
        normalized = normalize(results, signal.normalize)
        for result in fused_results:
            boost_score = normalized.get(result.id, 0.0)
            match signal.boost_type:
                Additive      => result.score += signal.weight * boost_score
                Multiplicative => result.score *= (1 + signal.weight * boost_score)

    // 5. Re-trier par score final
    fused_results.sort_by_score_desc()
```

### Structures Rust proposées

```rust
/// Per-signal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    pub weight: f64,
    #[serde(default)]
    pub role: SignalRole,
    #[serde(default)]
    pub boost_type: BoostType,
    #[serde(default)]
    pub normalize: Option<NormalizeMode>,
    #[serde(default)]
    pub top_k: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalRole {
    #[default]
    Fuse,
    Boost,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoostType {
    Additive,
    #[default]
    Multiplicative,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizeMode {
    MinMax,
    None,
    Rank,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FusionStrategy {
    #[default]
    Rrf,
    Weighted,
}
```

Le `Deserialize` de `signals` dans `KBConfig` doit gérer les 3 formats :
- `["bm25", "vector"]` → `SearchSignals` + `SignalConfig` par défaut pour chaque
- `{ "bm25": 0.3, "vector": 0.7 }` → `SearchSignals` + `SignalConfig { weight: N }` pour chaque
- `{ "bm25": { "weight": 0.3 }, ... }` → parsing complet

### Impact sur HybridStrategy

`HybridStrategy` actuel (`Boost`, `RRF`, `Weighted`) sera remplacé par `FusionStrategy` (`Rrf`, `Weighted`). Le mode `Boost` disparait en tant que stratégie globale — il devient un `role` per-signal.

Le `fuse_results()` actuel avec ses branches hardcodées sera remplacé par l'algorithme générique décrit ci-dessus.

### Future : Pipeline 2-stage explicite

Pour des cas avancés, un format pipeline explicite pourrait être ajouté :

```json
{
  "pipeline": [
    {
      "stage": "retrieve",
      "signals": { "bm25": 0.4, "vector": 0.6 },
      "strategy": "rrf",
      "top_k": 100
    },
    {
      "stage": "rerank",
      "signal": "sparse",
      "boost_type": "multiplicative",
      "weight": 0.3
    }
  ]
}
```

Sémantiquement plus clair pour des configurations complexes, mais pas nécessaire dans l'immédiat — le modèle `fuse`/`boost` per-signal couvre les mêmes cas.

## Build & Tests

```
cargo check                    ✓
cargo test --lib               ✓ 345 passed
run_e2e.sh phase4              3 passed, 4 failed (sparse schema bug)
```
