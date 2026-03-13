# Doc 07 — Design : registration idempotente (entities + KBs)

Date : 12 mars 2026

## 1. Problème

### 1.1 Simple entities

`register_entity("Product", config)` :
- **Rejette** si l'entité existe déjà (erreur "already registered")
- **Pas de persistance** : entity_configs en mémoire seulement, perdu à la réouverture du Catalog
- **Pas de migration** : impossible d'ajouter un champ sans tout recréer

### 1.2 KBs

Les KBs sont config-driven (YAML → `CatalogConfig` → `initialize()`). Pas de `register_kb()` dynamique.

- `initialize()` est déjà quasi-idempotent (tout est `IF NOT EXISTS`)
- Mais si on change la config entre deux `initialize()`, rien ne migre — les tables existent avec l'ancien schéma

### 1.3 Impact concret

| Scénario | Aujourd'hui | Attendu |
|----------|------------|---------|
| Dev itère sur son schéma entity | Doit détruire la DB et recréer | Ajout de champ transparent |
| Dev ajoute un champ contentFor à une entité KB | Doit détruire la DB | ALTER TABLE + rebuild FTS |
| Catalog fermé puis réouvert | entity_configs perdus | Configs rechargées auto |
| Même code re-lancé sans changement | Simple entities : erreur. KBs : OK | Tout : no-op |

## 2. État actuel du code

### 2.1 Simple entities (`register_entity()` — catalog.rs:301)

```
check_initialized()
→ entity déjà registered ? ERREUR
→ CREATE NODE TABLE IF NOT EXISTS {Entity}
→ CREATE NODE TABLE IF NOT EXISTS {Entity}_Chunk
→ CREATE REL TABLE IF NOT EXISTS {Entity}_CHUNKED_FROM
→ CREATE_LUCIVY_INDEX (ignore erreur)
→ CREATE HNSW INDEX (ignore erreur)
→ store in-memory : entity_configs + config.entities
```

### 2.2 KBs (`initialize()` — catalog.rs:182)

```
validate_schema(config)
→ generate_full_schema(config) → DDL + indexes
→ execute DDL (tout IF NOT EXISTS)
→ execute indexes (skip_if_exists / ignore erreur)
→ build kb_metadata from validation
→ init checkpoint store
```

Tables KB : `{KB}_Index`, `{KB}_Index_Chunk`, `{KB}_Index_HAS_CHUNK`, `{TitleEntity}_IN_{KB}`, `{Entity}_SOURCED_{KB}`

### 2.3 Infra existante

- `_catalog_meta` (key-value STRING/STRING) — créée dans le schéma mais **jamais utilisée**
- `ALTER TABLE {table} ADD {col} {type} DEFAULT {val}` — supporté par rag3db (Kuzu fork)
- `generate_rel_table_ddl()` — existe, produit `CREATE REL TABLE IF NOT EXISTS`

## 3. Direction retenue : API unifiée register_*

### 3.1 Trois méthodes publiques, toutes idempotentes

```rust
// 1. Enregistrer une entité (simple ou participant à une KB)
catalog.register_entity("Document", config).await?;

// 2. Déclarer une relation entre entités (pour KBs multi-entités)
catalog.register_relation("HAS_SCOPE", "Document", "Scope").await?;

// 3. Configurer une KB (signals, chunking, fusion, boosts)
catalog.register_kb("main", kb_config).await?;
```

Chaque méthode est idempotente : re-appeler = no-op ou migration additive.

### 3.2 SimpleFieldDef — rétrocompatible + extensible

```rust
pub struct SimpleFieldDef {
    pub field_type: FieldType,

    // ── Raccourcis "self" (pipeline simple, cas 90%) ───────────
    pub is_title: bool,       // alias pour title_for: "self"
    pub is_content: bool,     // alias pour content_for: ["self"]

    // ── KB explicite (pipeline KB, power users) ────────────────
    pub title_for: Option<String>,        // "main" → titre de KB main
    pub content_for: Option<Vec<String>>, // ["main"] → contenu de KB main
}
```

**Règles** :
- `is_title: true` et `title_for: Some(...)` sont mutuellement exclusifs (validation)
- `is_content: true` et `content_for: Some(...)` sont mutuellement exclusifs
- `is_title` = sucre pour pipeline simple, pas de KB créée
- `title_for: "main"` = cette entité fournit le titre de la KB "main"

### 3.3 Chemins internes

**Entité "self" (simple)** — quand au moins un champ a `is_title` ou `is_content` :
```
→ CREATE Entity + Entity_Chunk + Entity_CHUNKED_FROM
→ FTS index sur les content fields
→ Vector/sparse index si signals le demandent
→ Pipeline simple (InsertRecordNode → ChunkRecordNode → EmbedNode → FlushNode)
```
Pas de KB créée. C'est le code actuel.

**Entité KB** — quand au moins un champ a `title_for` ou `content_for` :
```
→ CREATE Entity (table entité)
→ Tables KB créées quand register_kb() est appelé :
  {KB}_Index, {KB}_Index_Chunk, rels, FTS, indexes
→ Pipeline KB (KBGatherNode → KBUpdateNode → KBChunkNode → EmbedNode → FlushNode)
```

### 3.4 register_relation()

```rust
pub async fn register_relation(
    &mut self,
    rel_name: &str,
    from_entity: &str,
    to_entity: &str,
) -> Result<(), CatalogError>
```

Génère `CREATE REL TABLE IF NOT EXISTS {rel}(FROM {from} TO {to})`. Idempotent.

Plus tard on pourra ajouter des propriétés sur la relation si besoin (un 4ème arg optionnel).

### 3.5 register_kb()

```rust
pub async fn register_kb(
    &mut self,
    kb_name: &str,
    kb_config: KBConfig,  // signals, chunking, fusion, boosts
) -> Result<(), CatalogError>
```

Détecte quelles entités ont des champs `title_for`/`content_for` pointant vers cette KB. Crée les tables d'index, chunks, rels. Si la KB existe déjà, diff et migre (ALTER TABLE ADD, rebuild FTS si content fields changés).

**Ordre d'appel** : `register_entity()` d'abord (déclare les entités et leurs champs), `register_relation()` si multi-entité, puis `register_kb()` (crée l'infra de recherche).

### 3.6 Exemples d'usage

**Simple entity (90% des cas)** :
```rust
catalog.register_entity("Product", EntityConfig {
    fields: hashmap! {
        "name"  => SimpleFieldDef { type: Text, is_title: true, .. },
        "desc"  => SimpleFieldDef { type: Text, is_content: true, .. },
        "price" => SimpleFieldDef { type: Double, .. },
    },
    signals: HYBRID,
    chunking: Default::default(),
}).await?;

// Prêt à ingest + search. Pas de register_kb() nécessaire.
```

**KB mono-entité** :
```rust
catalog.register_entity("Document", EntityConfig {
    fields: hashmap! {
        "title"   => SimpleFieldDef { type: Text, title_for: Some("docs"), .. },
        "content" => SimpleFieldDef { type: Text, content_for: Some(vec!["docs"]), .. },
        "path"    => SimpleFieldDef { type: String, .. },
    },
    ..Default::default()
}).await?;

catalog.register_kb("docs", KBConfig {
    signals: HYBRID,
    chunking: ChunkingConfig::default(),
    ..Default::default()
}).await?;
```

**KB multi-entités** :
```rust
catalog.register_entity("Document", EntityConfig {
    fields: hashmap! {
        "title"   => SimpleFieldDef { type: Text, title_for: Some("codebase"), .. },
        "content" => SimpleFieldDef { type: Text, content_for: Some(vec!["codebase"]), .. },
    },
    ..Default::default()
}).await?;

catalog.register_entity("Scope", EntityConfig {
    fields: hashmap! {
        "name"    => SimpleFieldDef { type: String, .. },
        "summary" => SimpleFieldDef { type: Text, content_for: Some(vec!["codebase"]), .. },
    },
    ..Default::default()
}).await?;

catalog.register_relation("HAS_SCOPE", "Document", "Scope").await?;

catalog.register_kb("codebase", KBConfig {
    signals: SearchSignals::BM25 | SearchSignals::VECTOR | SearchSignals::SPARSE,
    ..Default::default()
}).await?;
```

**Ajout de champ (migration)** :
```rust
// Deuxième appel, avec un champ en plus
catalog.register_entity("Product", EntityConfig {
    fields: hashmap! {
        "name"    => SimpleFieldDef { type: Text, is_title: true, .. },
        "desc"    => SimpleFieldDef { type: Text, is_content: true, .. },
        "price"   => SimpleFieldDef { type: Double, .. },
        "summary" => SimpleFieldDef { type: Text, is_content: true, .. },  // NOUVEAU
    },
    signals: HYBRID,
    ..
}).await?;
// → ALTER TABLE Product ADD summary STRING DEFAULT ''
// → Rebuild FTS index (content fields changés)
```

## 4. Migration additive — détail

### 4.1 Ce qui est migré automatiquement

| Changement | Action |
|-----------|--------|
| Nouveau champ (non-content) | `ALTER TABLE ADD {field} {type} DEFAULT {default}` |
| Nouveau champ content | ALTER TABLE ADD + rebuild FTS |
| Nouveau signal (vector/sparse) | Crée l'index manquant |
| Même config re-appelée | No-op |

### 4.2 Ce qui est refusé (erreur explicite)

| Changement | Raison |
|-----------|--------|
| Champ supprimé | Destructif |
| Type de champ changé | Destructif |
| Embedding dim changé | Nécessiterait recréer chunk tables + re-embed |

### 4.3 Détection needs_reindex

Si un champ `is_content`, `is_title`, `content_for` ou `title_for` est ajouté ou modifié, on :

1. Rebuild FTS index (DROP + CREATE avec les nouveaux champs)
2. Flag `needs_reindex:{entity}` = `true` dans `_catalog_meta`
3. Log warning : "Entity {name} needs reindex after schema change — run catalog.reindex('{name}')"

**Tant que le reindex n'est pas fait** :
- BM25 : index FTS recréé vide, anciennes données ne remontent plus en BM25 (résultats incomplets, pas faux)
- Vector/Sparse : chunks existants ont les anciens embeddings, search marche mais contenu partiel
- Données : nouvelle colonne vide pour les anciennes lignes, queries OK

Rien ne casse — c'est juste dégradé.

### 4.4 reindex()

```rust
pub async fn reindex(&mut self, entity_name: &str) -> Result<(), CatalogError>
```

- Query tous les UUIDs existants de l'entité
- Enqueue comme updates dans PendingWork
- Drain → re-chunk, re-embed, re-index FTS
- Clear flag `needs_reindex:{entity}` dans `_catalog_meta`

### 4.4 Defaults par type pour ALTER TABLE ADD

| Type | Default Kuzu |
|------|-------------|
| STRING / Text | `''` |
| INT64 / Integer | `0` |
| DOUBLE / Number | `0.0` |
| BOOLEAN | `false` |
| TIMESTAMP | `'1970-01-01 00:00:00'` |

## 5. Persistance

Tout est stocké dans `_catalog_meta` (table key-value existante, jamais utilisée jusqu'ici).

| Clé | Valeur |
|-----|--------|
| `entity_config:{name}` | JSON de EntityConfig |
| `kb_config:{name}` | JSON de KBConfig |
| `relation:{name}` | JSON `{ from, to }` |

Au `initialize()` : charger depuis `_catalog_meta` → restaurer entity_configs, kb_metadata, relations.

## 6. Compatibilité avec l'existant

### 6.1 CatalogConfig (YAML) reste fonctionnel

`initialize()` continue de lire `CatalogConfig` et créer les tables. Les KBs config-driven marchent comme avant. La seule différence : on persiste aussi dans `_catalog_meta` pour détecter les changements au prochain init.

### 6.2 Les deux chemins coexistent

- Entités déclarées dans `CatalogConfig.entities` → créées par `initialize()` (code existant)
- Entités déclarées via `register_entity()` → créées dynamiquement (code nouveau)
- Les deux finissent dans `entity_configs` + `_catalog_meta`

À terme on pourrait déprécier le chemin config-driven et tout passer par `register_*()`, mais c'est pas obligatoire.

## 7. Questions ouvertes

1. **Faut-il un `reindex()`** pour forcer le re-drain/re-chunk/re-embed après ajout d'un content field ?
2. **Propriétés sur relations** : `register_relation()` prend juste from/to pour l'instant. Ajouter un arg optionnel `properties: Option<HashMap<String, FieldDef>>` plus tard ?
3. **Magie future** : auto-détection des relations entre entités d'une même KB ? Auto-inférence de la KB config depuis la première entité ? Sucre syntaxique, pas urgent.
4. **Ordre d'implémentation** : register_entity idempotent + persistance d'abord (le plus utile), puis register_relation, puis register_kb ?
