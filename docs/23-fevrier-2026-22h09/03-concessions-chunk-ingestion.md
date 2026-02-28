# 03 — Concessions de l'implémentation chunk ingestion

## 1. Core offsets = copie des offsets normaux

**Problème** : `_core_start_char` / `_core_end_char` sont actuellement copiés depuis `start_byte` / `end_byte`. Le vrai calcul devrait donner la zone sans overlap — le "cœur" du chunk.

**Impact** : Le frontend highlight la zone entière du chunk au lieu de la zone originale sans contexte overlap. Pas d'impact sur le search, uniquement sur l'affichage.

**Fix prévu** : Retrouver l'implémentation existante dans les anciennes versions (rag3weaver-l1 à l4, test-l5) qui avait déjà ce calcul testé et approuvé. Le calcul est grosso modo :
- Chunk 0 : `core_start = start`, `core_end = start + (end - start) - overlap/2`
- Chunk milieu : `core_start = start + overlap/2`, `core_end = end - overlap/2`
- Dernier chunk : `core_start = start + overlap/2`, `core_end = end`

Mais le vrai overlap peut varier avec les boundaries sémantiques, donc il faut le calculer à partir des offsets réels des chunks voisins, pas de la config overlap.

**Où** : `build_chunk_ops()` dans `catalog.rs`, après le `chunker.chunk()`. On a accès à tous les chunks d'un coup donc on peut comparer les voisins.

## 2. Chunker instancié par KB × create()

**Problème** : `build_chunk_ops()` crée un `Chunker::new(ChunkerConfig{...})` à chaque appel pour chaque KB. Si 10 KBs utilisent la même config de chunking, c'est 10 instanciations identiques.

**Impact** : Négligeable (Chunker est 3 champs config), mais c'est pas propre.

**Fix prévu** : Un registre `HashMap<u64, Chunker>` sur le Catalog, keyed par hash de la ChunkerConfig. Au `build_chunk_ops`, on lookup par hash au lieu d'instancier. La config est petite (max_size, overlap, strategy) donc le hash est trivial.

**Alternative plus simple** : Un seul `Chunker` sur le Catalog si tous les KBs partagent la même config (ce qui est le cas courant). Lazy init au premier `build_chunk_ops`.

## 3. Processors : vérifier le batching des embeddings

**Problème** : Les ops sont individuels (1 EmbedOp par chunk), mais les processors sont censés les traiter en batch. C'est le design de la queue — on n'a pas à batcher à la génération d'ops.

**Mais à vérifier** : Est-ce que `EmbedProcessor::process()` regroupe bien les embeddings en un seul appel à l'embedder ? Ou est-ce qu'il fait 1 appel embedder par op ?

Si l'embedder reçoit 50 chunks un par un au lieu d'un batch de 50, c'est 50 round-trips HTTP (ou 50 inferences) au lieu d'1. C'est le point le plus critique côté performance.

Les anti-patterns Cypher du plan V2 (batch UNWIND dans EmbedProcessor et SparseEmbedProcessor) s'appliqueront aussi aux chunks — les chunks vont même amplifier le problème (N chunks par document au lieu de 1 embedding par document).

## 4. Clones de textes

**Problème** : `chunk.text.clone()` pour InsertOp + `embed_text.clone()` pour EmbedOp + potentiellement SparseEmbedOp = 3 copies du même texte de ~1500 chars par chunk.

**Impact** : ~4.5 KB par chunk. Pour un document de 100 chunks, ça fait ~450 KB de copies inutiles. Pas bloquant mais ça s'accumule sur des ingestions massives.

**Fix prévu** (plan V2, concession #8) : `Arc<String>` pour partager le texte entre les ops. Un seul clone d'Arc (8 bytes) au lieu de clone du String.

## 5. `_text_hash` par chunk vs content hash au niveau document

**Problème initial** : On fait `content_hash(&chunk.text)` par chunk pour stocker `_text_hash`. Blake3 est rapide, pas un problème de perf.

**Mais la vraie question est l'invalidation** : le `_content_hash` est au niveau du **document entier**, et c'est lui qui décide dans `update()` si on re-indexe. C'est correct : si le body change, le hash document change, on delete tous les chunks et on re-chunk tout.

Le `_text_hash` sur chaque chunk est un hash du texte du chunk individuel. Il pourrait servir pour :
- **Invalidation fine** : ne re-embedder que les chunks dont le texte a changé (au lieu de tout re-faire)
- **Déduplication** : détecter des chunks identiques entre documents

Pour l'instant c'est stocké mais pas utilisé. L'invalidation fine nécessiterait de comparer les anciens chunks avec les nouveaux (match par index + hash), ce qui est plus complexe que le delete-all + re-create actuel. À envisager quand l'ingestion incrémentale sera un bottleneck.

**Flow actuel** :
```
update() → content_hash document changé ?
  OUI → delete ALL chunks → re-chunk body → re-insert ALL + re-embed ALL
  NON → rien
```

**Flow optimisé (futur)** :
```
update() → content_hash document changé ?
  OUI → re-chunk body → comparer text_hash ancien vs nouveau par index
       → delete chunks changés → insert nouveaux → embed seulement les changés
  NON → rien
```

## 6. Pas de parallélisme du chunking multi-champ

**Problème** : Si une entité a 2 champs chunked (`body` et `code`), on chunk séquentiellement dans la boucle.

**Impact** : Le Chunker est CPU-only et rapide (<1ms pour 10KB de texte). Pas un bottleneck.

**Fix futur** : Plutôt que paralléliser le chunking lui-même, prévoir un parallélisme au niveau de la queue d'ops — envoyer les ops de chunking de N documents en parallèle. Le chunking est embarrassingly parallel par document.

## Résumé priorités

| # | Concession | Priorité | Effort |
|---|---|---|---|
| 1 | Core offsets | Moyenne | Faible (code existant à retrouver) |
| 2 | Cache Chunker | Basse | Trivial |
| 3 | Vérifier batch embed | **Haute** | Lecture code |
| 4 | Arc textes | Basse | Moyen (plan V2) |
| 5 | Invalidation fine chunks | Basse | Élevé (futur) |
| 6 | Parallélisme chunking | Basse | Moyen (futur) |
