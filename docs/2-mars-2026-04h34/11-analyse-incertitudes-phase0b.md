# 11 — Analyse des incertitudes Phase 0b (3 mars 2026)

## État actuel : 362 tests, 0 échecs, 4 warnings

---

## Question 1 : Delete d'une entité contentFor-only

### Scénario

File a `contentFor: [TreeKB]` mais c'est Directory qui a `titleFor: TreeKB`. On delete un File.

### Problème : NON GÉRÉ

Le code actuel dans `delete()` (catalog.rs:720-722) :

```rust
for (kb_name, mapping) in &entity_kbs {
    if mapping.title_field.is_none() {
        continue;  // ← SKIP les KBs où l'entité est juste contentFor !
    }
}
```

`resolve_entity_kbs(File)` retourne `{TreeKB: {title_field: None, content_fields: [...]}}`. Le `continue` fait que TreeKB est complètement ignoré → **aucun re-aggregate**, le `_content` de `TreeKB_Index` garde le contenu du File supprimé, et les chunks SOURCED pointent vers une entité qui n'existe plus.

### Même problème dans `update()`

catalog.rs:663-680 — le `TODO` est explicite :

```rust
// TODO: if entity is contentFor (not titleFor), find the title entity's
// index entries linked via relations and enqueue AggregateOps for them.
```

### Solution

Pour `delete()` et `update()`, quand `mapping.title_field.is_none()` (contentFor-only) :

1. Trouver le title entity de cette KB via `kb_metadata[kb_name].title.entity`
2. Trouver la relation qui relie cette entité au title entity (même logique que `find_relation_to_entity()` dans AggregateProcessor)
3. Query les title entities liées :
   ```cypher
   MATCH (content:File {_uuid: $uuid})<-[:HAS_FILE]-(title:Directory)
   RETURN title._uuid
   ```
4. Pour chaque title entity trouvée → enqueue `AggregateOp` (pour `update()`) ou supprimer les chunks SOURCED + re-aggregate (pour `delete()`)

**Priorité :** Moyenne. Le flow batch initial (create → link → drain) fonctionne correctement car l'AggregateOp est enqueué par `create()` du title entity. Le bug n'affecte que l'incrémental (update/delete d'un contentFor entity après le drain initial).

---

## Question 2 : Offsets start_char / end_char des chunks et résolution vers les entités sources

### Comment ça marche actuellement

Le chunker reçoit le texte **d'un seul champ source** (pas le concaténé) :

```rust
// AggregateProcessor.process_one()
for source in &sources {
    let chunks = chunker.chunk(&source.text);  // source.text = valeur du champ
}
```

Les offsets retournés par le chunker (`start_byte`, `end_byte`, `start_line`, `end_line`, `core_*`) sont **relatifs au texte d'entrée**, c'est-à-dire à la valeur du champ source.

### Stockage sur les chunks

```
{KB}_Index_Chunk:
  _source_field = "body"              ← quel champ du source entity
  _start_char = 0                     ← relatif à la valeur du champ
  _end_char = 1500                    ← relatif à la valeur du champ
```

### Résolution vers l'entité source

Pour remonter du chunk vers l'entité source, on a deux chemins :

**Chemin 1 — Via la relation SOURCED :**
```cypher
MATCH (entity)-[:File_SOURCED_TreeKB]->(chunk:{KB}_Index_Chunk {_uuid: $chunk_uuid})
RETURN entity._uuid, entity.name, entity.absolute_path
```
Donne directement l'entité source + ses données.

**Chemin 2 — Via l'index entry :**
```cypher
MATCH (idx:{KB}_Index)-[:TreeKB_Index_HAS_CHUNK]->(chunk {_uuid: $chunk_uuid})
RETURN idx._source_entity, idx._source_uuid
```
Donne le type et UUID du title entity (pas directement l'entité source du chunk).

### Verdict : OK

Les offsets sont **cohérents** — relatifs au champ source, pas au texte concaténé. La résolution vers l'entité source se fait via `{Entity}_SOURCED_{KB}` (traversable, efficace).

**Limitation connue :** pour une recherche qui retourne des chunks de File ET de Directory (multi-entity), il faut traverser les SOURCED rels pour savoir quelle entité a produit quel chunk. Le `_source_field` dit quel champ, mais pas quelle entité. Les SOURCED rels comblent ce manque.

---

## Question 3 : Le titre est-il ajouté aux chunks ? Offsets impactés ?

### Réponse : Le titre est ajouté à l'EMBED TEXT, PAS au chunk text

Le code dans `AggregateProcessor.process_one()` :

```rust
let embed_text = if !title_text.is_empty() {
    format!("{title_text}\n---\n{}", chunk.text)
} else {
    chunk.text.clone()
};

// chunk._text = chunk.text (PAS le embed_text)
chunk_data.insert("_text".to_string(), CypherValue::String(chunk.text.clone()));

// chunk._start_char = relatif au champ source (PAS au embed_text)
chunk_data.insert("_start_char".to_string(), CypherValue::Int(chunk.start_byte as i64));
```

### Séparation des données

| Donnée | Contenu | Offsets relatifs à |
|--------|---------|-------------------|
| `chunk._text` | Texte brut du chunk | — |
| `chunk._start_char` / `_end_char` | Offsets dans le champ source | Valeur du champ source |
| `embed_text` | `"{title}\n---\n{chunk}"` | Pas stocké, seulement passé à l'embedder |
| `{KB}_embedding` | Vecteur dense | — |

### Verdict : OK, pas de confusion

Le même pattern est utilisé dans l'ancien `compute_chunk_ops()` — c'est une convention établie. Le titre enrichit l'embedding (meilleure qualité sémantique) sans polluer les offsets.

---

## Question 4 : Résolution BM25 highlights vers chunks d'une KB multi-entité

### Problème : NON GÉRÉ (bug de mapping field name + offset)

C'est le problème le plus complexe. Deux bugs combinés :

### Bug A — Mauvais nom de clé

`search_bm25_chunked()` (search.rs) fait :

```rust
if let Some(offsets) = highlights.get(&chunk.parent_field) {
```

- `highlights` a les clés `"_content"` et `"_title"` (noms des champs de l'Index table)
- `chunk.parent_field` = `"body"`, `"absolute_path"`, etc. (noms des champs source)
- `highlights.get("body")` retourne `None` → aucun chunk ne matche

### Bug B — Système de coordonnées incompatible

Même si le nom de clé était correct :
- Les offsets de highlight sont relatifs au `_content` **concaténé** (toutes les sources jointes par `\n`)
- Les offsets des chunks (`_start_char`, `_end_char`) sont relatifs au **champ source individuel**
- Comparer directement donne des résultats absurdes

### Exemple concret

```
Sources (triées) :
  Directory.absolute_path = "/src/"       (5 chars)
  File.absolute_path      = "/src/auth.ts" (12 chars)
  File.name               = "auth.ts"      (7 chars)

_content = "/src/\n/src/auth.ts\nauth.ts"
           [0-4]  [6-17]       [19-25]

Chunks créés :
  chunk_1: source_field="absolute_path", start_char=0, end_char=5   (texte: "/src/")
  chunk_2: source_field="absolute_path", start_char=0, end_char=12  (texte: "/src/auth.ts")
  chunk_3: source_field="name",          start_char=0, end_char=7   (texte: "auth.ts")
```

Recherche BM25 pour "auth" :
- Tantivy retourne : `{"_content": [[6, 10], [19, 23]]}`
- Offset [6,10] pointe vers "auth" dans "/src/auth.ts" (position globale dans _content)
- Offset [19,23] pointe vers "auth" dans "auth.ts"
- Mais chunk_2 a `start_char=0, end_char=12` (relatif à son champ source)
- Comparaison impossible sans translation

### Solutions possibles

#### Option A : `_content_offset` sur chaque chunk (recommandée)

Stocker la position de début du champ source dans le `_content` concaténé sur chaque chunk :

```
chunk._content_offset = 6    (pour les chunks de File.absolute_path)
chunk._content_offset = 19   (pour les chunks de File.name)
```

Translation au search time :
```
highlight_global_offset - chunk._content_offset = offset_dans_le_champ_source
```

**Avantages :** simple, une seule colonne INT64 en plus, translation O(1).

**Inconvénient :** si `_content` est reconstruit (re-aggregate), les offsets changent → il faut recalculer les `_content_offset` des chunks existants. Mais puisque le re-aggregate delete+recreate les chunks, c'est automatique.

#### Option B : `_content_boundaries` JSON sur l'index entry

Stocker un mapping `field → (start, end)` sur `{KB}_Index` :

```json
{"Directory.absolute_path": [0, 5], "File.absolute_path": [6, 18], "File.name": [19, 26]}
```

Translation au search time : trouver quel champ contient l'offset global, puis soustraire le start.

**Avantage :** pas de colonne en plus sur les chunks.
**Inconvénient :** JSON parsing au search time, plus complexe.

#### Option C : Chercher les highlights par champ source dans Tantivy

Indexer dans Tantivy un document avec des champs dynamiques par source (`Directory_absolute_path`, `File_name`, etc.) au lieu d'un seul `_content`.

**Avantage :** highlights déjà par champ source, pas de translation nécessaire.
**Inconvénient :** restructuration majeure du FTS index, et problème d'IDF (les champs seraient séparés, pas un seul corpus).

### Recommandation : Option A (`_content_offset`)

1. Ajouter `_content_offset INT64` au schema de `{KB}_Index_Chunk`
2. Dans `AggregateProcessor`, tracker l'offset cumulé en itérant les sources :
   ```rust
   let mut content_offset = 0usize;
   for source in &sources {
       // ... chunks pour ce source ...
       // chunk._content_offset = content_offset + chunk.start_byte
       // NON en fait: content_offset est le début du champ source dans _content
       chunk_data.insert("_content_offset", content_offset as i64);
       content_offset += source.text.len() + 1; // +1 pour le \n
   }
   ```
3. Dans `search_bm25_chunked()`, pour chaque highlight sur `_content` :
   ```rust
   let chunk_start_in_content = chunk.content_offset + chunk.start_char;
   let chunk_end_in_content = chunk.content_offset + chunk.end_char;
   let overlap = h_end.min(chunk_end_in_content).saturating_sub(h_start.max(chunk_start_in_content));
   ```

---

## Résumé des actions

| # | Problème | Gravité | Action |
|---|----------|---------|--------|
| 1 | delete/update contentFor-only | Moyenne | Implémenter la traversée relations → AggregateOp pour les title entities liées |
| 2 | Offsets chunks | OK | Rien à faire, déjà relatifs au champ source |
| 3 | Titre dans chunks | OK | Déjà séparé (titre dans embed_text, pas dans _text ni offsets) |
| 4 | BM25 highlight → chunks | Haute | Ajouter `_content_offset` sur chunks + fix `search_bm25_chunked()` mapping |

### Ordre recommandé

1. **Q4 (BM25 highlights)** en premier — c'est un bug qui casse le search chunked pour toutes les KBs, même single-entity (le bug de field name `"body"` vs `"_content"` affecte tout le monde)
2. **Q1 (delete/update contentFor)** ensuite — n'affecte que l'incrémental
3. Q2 et Q3 sont déjà OK

---

## Fichiers concernés

| Fichier | Changement |
|---------|-----------|
| `schema.rs` | Ajouter `_content_offset INT64` dans `generate_index_chunk_table_ddl()` |
| `catalog.rs` | AggregateProcessor : calculer et stocker `_content_offset` par chunk |
| `catalog.rs` | `delete()` et `update()` : gérer les entités contentFor-only |
| `search.rs` | `search_bm25_chunked()` : utiliser `_content` comme clé + translation offsets via `_content_offset` |
| `search.rs` | `resolve_bm25_to_chunks()` : idem |
