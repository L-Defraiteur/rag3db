# 05 — Plan : `create_batch()` — chunking parallèle via rayon

## Contexte

Quand on ingère un GitHub entier (10K+ fichiers), les appels séquentiels `create()` → `build_chunk_ops()` deviennent un bottleneck CPU. Le chunking est pur (`&self`, pas de I/O) et embarrassingly parallel par document. Plus on ajoute de couches d'abstraction, plus il sera difficile de paralléliser — c'est le bon moment.

## Approche

Ajouter une méthode `create_batch()` qui prend N documents et parallélise le travail CPU (UUID, content hash, chunking) via `rayon::par_iter`, puis enqueue séquentiellement.

### Architecture

```
create_batch(items: Vec<(&str, HashMap)>)
  │
  ├─ Phase 1 (séquentielle) : warm chunker cache
  │
  ├─ Phase 2 (parallèle, rayon) : pour chaque doc
  │     ├─ validate entity
  │     ├─ generate UUID + content hash      (CPU, pur)
  │     ├─ build full_data
  │     ├─ create EntityRef pair
  │     ├─ compute chunk ops via chunker     (CPU, pur)
  │     └─ return (InsertOp + chunk ops, EntityRef)
  │
  └─ Phase 3 (séquentielle) : enqueue all ops
```

### Pourquoi ça marche

- `Chunker::chunk(&self)` — pur, `Send + Sync` implicitement
- Tous les ops (`InsertOp`, `LinkOp`, `EmbedOp`, `SparseEmbedOp`) sont `Send + Sync`
- `EntityRef::new()` crée des tokio watch channels — thread-safe
- La queue (`enqueue_all`) prend `&mut self` → doit rester séquentielle
- En splitant les borrows (refs partagées aux champs read-only, puis `&mut` pour la queue), le borrow checker est content

### Gestion du borrow checker

Le point clé : `create_batch(&mut self)` ne peut pas passer `&self` aux closures rayon. On split :

```rust
pub fn create_batch(&mut self, items: Vec<...>) -> ... {
    // Phase 1: warm cache (&mut self)
    self.warm_chunker_cache();

    // Emprunts partagés vers les champs read-only
    let config = &self.config;
    let kb_metadata = &self.kb_metadata;
    let chunker_cache = &self.chunker_cache;
    let has_sparse = self.sparse_embedder.is_some();

    // Phase 2: parallèle (pas de &mut self)
    let results: Vec<_> = items.par_iter().map(|(entity_name, data)| {
        compute_document_ops(entity_name, data, config, kb_metadata, chunker_cache, has_sparse)
    }).collect();

    // Phase 3: séquentielle (&mut self pour la queue)
    for (ops, _) in &results {
        self.queue.enqueue_all(ops);
    }
}
```

## Changements fichier par fichier

### 1. `Cargo.toml` — rayon toujours disponible

```toml
rayon = "1.10"   # était optional
```

Retirer `"dep:rayon"` de la feature `wasm-emscripten`. Rayon fonctionne nativement sans config spéciale.

### 2. `catalog.rs` — refactoring + `create_batch()`

**a. Extraire `compute_chunk_ops_standalone()` (fonction libre)**

Extraire la logique de `build_chunk_ops()` dans une fonction libre qui prend des refs explicites au lieu de `&self` :

```rust
fn compute_chunk_ops_standalone(
    entity_name: &str,
    parent_uuid: &str,
    entity_ref: &EntityRef,
    data: &HashMap<String, CypherValue>,
    config: &CatalogConfig,
    kb_metadata: &HashMap<String, KBMetadata>,
    chunker_cache: &HashMap<ChunkerConfig, Chunker>,
    has_sparse: bool,
) -> Vec<CatalogOp>
```

Même corps que `build_chunk_ops()` actuel, mais les accès `self.xxx` deviennent des params. Le `chunker_cache.get(&key).expect(...)` au lieu de `.entry().or_insert_with()`.

**b. `build_chunk_ops()` devient un wrapper**

```rust
fn build_chunk_ops(&mut self, ...) -> Vec<CatalogOp> {
    // Warm cache si besoin
    self.ensure_chunker_cached(entity_name);
    compute_chunk_ops_standalone(..., &self.config, &self.kb_metadata, &self.chunker_cache, ...)
}
```

**c. Ajouter `warm_chunker_cache(&mut self)`**

Pré-remplit le cache pour **toutes** les ChunkingConfig connues dans `kb_metadata` :

```rust
fn warm_chunker_cache(&mut self) {
    for kb in self.kb_metadata.values() {
        let key = ChunkerConfig {
            max_size: kb.chunking.max_size,
            overlap: kb.chunking.overlap,
            strategy: kb.chunking.strategy.clone(),
        };
        self.chunker_cache.entry(key.clone()).or_insert_with(|| Chunker::new(key));
    }
}
```

**d. Ajouter `create_batch()`**

```rust
pub fn create_batch(
    &mut self,
    items: Vec<(&str, HashMap<String, CypherValue>)>,
) -> Result<Vec<EntityRef>, CatalogError> {
    self.check_initialized()?;
    self.warm_chunker_cache();

    // Shared refs for parallel phase
    let config = &self.config;
    let kb_metadata = &self.kb_metadata;
    let chunker_cache = &self.chunker_cache;
    let has_sparse = self.sparse_embedder.is_some();

    // Phase parallèle : validation + UUID + hash + chunking
    let results: Vec<Result<(Vec<CatalogOp>, EntityRef), CatalogError>> =
        items.par_iter().map(|(entity_name, data)| {
            // validate entity
            let entity_def = config.entities.get(*entity_name)
                .ok_or_else(|| CatalogError::UnknownEntity(entity_name.to_string()))?;

            // UUID
            let uuid = if let Some(ref hashsafe_fields) = entity_def.hashsafe {
                let field_values: Vec<&str> = hashsafe_fields.iter()
                    .map(|f| data.get(f).and_then(|v| v.as_str()).unwrap_or(""))
                    .collect();
                hashsafe_uuid(entity_name, &field_values)
            } else {
                crate::refs::generate_temp_uuid()
            };

            // Content hash
            let content_text = build_content_text_standalone(entity_name, data, config);
            let hash = content_hash(&content_text);

            // Build full data
            let mut full_data = data.clone();
            full_data.insert("_uuid".into(), CypherValue::String(uuid.clone()));
            full_data.insert("_content_hash".into(), CypherValue::String(hash));

            // EntityRef
            let (entity_ref, resolver) = EntityRef::new(entity_name);
            let insert_op = CatalogOp::Insert(InsertOp::new(
                entity_name.to_string(), full_data, resolver, entity_ref.clone(),
            ));

            // Chunk ops
            let chunk_ops = compute_chunk_ops_standalone(
                entity_name, &uuid, &entity_ref, data,
                config, kb_metadata, chunker_cache, has_sparse,
            );

            let mut ops = vec![insert_op];
            ops.extend(chunk_ops);
            Ok((ops, entity_ref))
        }).collect();

    // Phase séquentielle : enqueue + collect refs
    let mut refs = Vec::with_capacity(items.len());
    for result in results {
        let (ops, entity_ref) = result?;
        self.queue.enqueue_all(ops);
        refs.push(entity_ref);
    }
    Ok(refs)
}
```

**e. Extraire `build_content_text_standalone()`**

Même pattern : fonction libre qui prend `&CatalogConfig` au lieu de `&self`.

### 3. Pas de changement à `create()`

`create()` reste tel quel pour le chemin single-doc. Il appelle toujours `build_chunk_ops(&mut self)` avec le cache lazy.

## Vérification

```bash
# Tous les tests existants doivent passer
cd packages/rag3db/extension/rag3weaver && cargo test --lib

# Test spécifique create_batch
cargo test --lib -- create_batch
```

Tests à ajouter :
- `create_batch_basic` — 3 docs, vérifie que les ops sont créés (count)
- `create_batch_empty` — 0 docs, retourne Ok(vec![])
- `create_batch_unknown_entity` — 1 doc avec entity inconnue → erreur
- `create_batch_multiple_entities` — mix de types d'entités

## Fichiers impactés

| Fichier | Changement |
|---|---|
| `Cargo.toml` | rayon non-optional |
| `catalog.rs` | `create_batch()`, `warm_chunker_cache()`, fonctions standalone extraites |
