# 05 — Vision : Pipeline Queues — du chaos ragforge-core vers un DAG d'opérations

Ce document part du constat que les pipelines d'ingestion deviennent vite chaotiques, et propose une architecture de queues hiérarchiques qui étend le pattern de rag3weaver au-delà du catalog.

---

## 1. Leçons de ragforge-core : pourquoi tout devient bordelique

### Le problème fondamental

ragforge-core avait une pipeline linéaire (discovered → parsing → parsed → relations → linked → entities → embedding → embedded) implémentée à travers ~6000 lignes réparties dans :

| Composant | Lignes | Responsabilité |
|-----------|--------|----------------|
| UnifiedProcessor | ~2500 | Orchestrateur principal, 3 méthodes process*() |
| FileProcessor | ~1700 | Parse fichiers, crée scopes, résout imports |
| FileStateMachine | ~730 | Machine à états fichiers (8 états + error) |
| FileWatcher | ~440 | Chokidar + batching 500ms |
| TouchedFilesWatcher | ~510 | Orphan files = pseudo-projet "touched-files" |
| IncrementalIngestionManager | ~400+ | Hash detection, diff, recovery |

### Ce qui dérapait

**1. Course aux conditions et re-checks constants**
```typescript
// FileProcessor.processFile() — ligne 1
const currentState = await neo4jClient.run(`MATCH (f:File {uuid: $uuid}) RETURN f._state`);
if (currentState === 'parsing' || currentState === 'linked') {
    return { status: 'skipped' }; // Déjà traité par un autre chemin!
}
```
Le read_file de l'agent pouvait déclencher un updateMediaContent en parallèle de la pipeline normale → race condition. Solution ragforge : re-checker l'état au début de chaque étape.

**2. Deux systèmes de processing pour la même chose**
- Fichiers projet → UnifiedProcessor → FileProcessor → batch parse
- Fichiers orphelins → TouchedFilesWatcher → FileProcessor → même parse
- Le FileProcessor ne sait pas "qui" l'appelle → duplique la logique de transition

**3. Incrémental = diff ad hoc à chaque étape**
```typescript
// FileProcessor.processBatchFiles()
const storedHash = await getStoredHash(filePath);
if (newHash === storedHash) {
    await stateMachine.transition(uuid, 'linked'); // Skip direct
}
// ...mais plus tard dans processLinked():
// "est-ce que les embeddings sont à jour?" → re-check content hash vs embedded hash
```
Chaque étape re-check indépendamment si le travail a été fait. Pas de source de vérité unique.

**4. Recovery = heuristiques fragiles**
```typescript
// FileStateMachine.resetStuckFiles()
// Resets files stuck in 'parsing'/'relations'/'entities'/'embedding' for > 5 minutes
```
Si un crash survient pendant le parsing → fichier bloqué en "parsing" → timeout de 5 min → reset → re-parse complet. Pas de checkpoint granulaire.

**5. Virtual files : stockage monolithique**
```typescript
// File node avec _rawContent dans Neo4j → potentiellement des MB de code source
// Dans la même base qui sert pour la search!
```
Le contenu brut est dans Neo4j, les chunks aussi, les embeddings aussi. Pas de séparation de concerns.

**6. Le pseudo-projet "touched-files"**
```typescript
const ORPHAN_PROJECT_ID = 'touched-files';
// Un fichier ouvert par l'agent → ajouté au projet orphelin
// Parsing individuel, pas de résolution inter-fichier
// Plus tard: "ah ce fichier fait partie d'un vrai projet" → migration manuelle
```
L'orphelin ne sait pas qu'il fait partie d'un projet. Pas de détection automatique de package.json/Cargo.toml/go.mod pour l'associer.

### Ce qui marchait bien

- **Batch UNWIND Neo4j** : `createNodesBatchGlobal()` avec MERGE → 100x plus rapide que le one-by-one
- **Un seul adapter.parse()** pour N fichiers → codeparsers en mode batch
- **Chokidar batching** (500ms) : évite le flood de FS events
- **Content hash** pour skip les fichiers inchangés

### La conclusion

Le problème n'est pas le code — c'est l'**absence de queue persistée**. Chaque composant gère son propre état, sa propre recovery, ses propres re-checks. Il n'y a pas de source de vérité centralisée qui dit "voilà ce qu'il reste à faire, dans quel ordre, avec quel état".

---

## 2. Architecture proposée : Pipeline Queue + Catalog Queue

### Deux niveaux de queue

```
┌──────────────────────────────────────────────────────────────┐
│  PIPELINE QUEUE (nouveau)                                     │
│  Opérations de haut niveau, spécifiques au domaine            │
│  "Que faut-il ingérer et comment?"                            │
│                                                                │
│  Ex: DownloadGithubOp → ScanTreeOp → ParseProjectOp          │
│                                                                │
│  Persistence: oui (crash recovery, reprise après reboot)      │
│  Priorité: configurable par domaine                           │
│  Processeur: DomainProcessor (trait, implémenté par domaine)  │
└───────────────────────────┬──────────────────────────────────┘
                            │ émet catalog.create() / catalog.link()
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  CATALOG QUEUE (existant rag3weaver)                          │
│  Opérations internes au catalog                               │
│  "Comment stocker, chunker, embedder les entités?"            │
│                                                                │
│  ChunkOp → InsertOp → LinkOp → EmbedOp                       │
│                                                                │
│  Persistence: via OperationPersistence (existant)             │
│  Priorité: fixe (0→3)                                         │
│  Processeurs: built-in (ChunkProcessor, InsertProcessor, etc.)│
└──────────────────────────────────────────────────────────────┘
```

**Pourquoi deux queues ?**
- La catalog queue est **interne** — elle gère le chunking/embedding/storage. Son API est `create() + link() + drain()`. Les domaines ne la manipulent pas directement.
- La pipeline queue est **externe** — elle gère l'orchestration de haut niveau (download, parse, scan, etc.). Chaque domaine définit ses propres ops et processeurs.
- Séparer les deux évite de polluer la catalog queue avec des ops à sémantique très différente (un DownloadGithubOp peut prendre 30 secondes, un InsertOp 1ms).

### Interface commune

Les deux queues partagent le même pattern fondamental :

```rust
trait Queue<Op> {
    fn enqueue(&self, op: Op) -> OpRef;
    fn drain(&self) -> DrainResult;
    fn subscribe(&self) -> Receiver<QueueEvent>;
    fn set_persistence(&mut self, persistence: Box<dyn OperationPersistence>);
}
```

La pipeline queue peut réutiliser **le même code** que `OperationQueue` de rag3weaver, paramétré avec un enum d'ops différent.

---

## 3. Operations pipeline par domaine

### Code domain

```
Priorité 0 — Acquisition
├── DownloadGithubOp { url, branch, shallow: bool }
│     Processeur: git clone --depth 1 ou ZIP API
│     Émet: ScanTreeOp
│
├── WatchLocalOp { root_path, patterns: ["**/*.ts"], ignore: ["node_modules"] }
│     Processeur: setup chokidar/inotify watcher
│     Émet: ScanTreeOp (initial) + FileChangedOp (en continu)
│
└── FileChangedOp { path, event: Add|Change|Delete }
      Processeur: debounce 500ms, batch par projet
      Émet: ScanTreeOp (si nouveau fichier) ou DeleteEntityOp (si supprimé) ou ReparseFileOp

Priorité 1 — Scan
├── ScanTreeOp { root_path, source_type: Github|Local }
│     Processeur: walk tree, détecte projets (package.json, Cargo.toml, etc.)
│     Émet: N × EnqueueDirectoryOp + N × EnqueueFileOp + N × DetectProjectOp
│
└── DetectProjectOp { manifest_path, manifest_type: PackageJson|CargoToml|GoMod|... }
      Processeur: parse manifest, identifie le projet et ses dépendances
      Émet: N × ParseProjectOp (un par workspace member si monorepo)

Priorité 2 — Parse
├── ParseProjectOp { project_root, files: [path], language_hint }
│     Processeur: codeparsers ProjectParser::parse_project()
│     Émet: N × IngestScopeOp + N × IngestLibraryOp + N × IngestRelationOp
│
├── ReparseFileOp { file_path, content_hash }
│     Processeur: codeparsers single file parse, diff avec scopes existants
│     Émet: create/update/delete scope ops selon diff
│
└── TouchSourceOp { file_path, content, trigger: AgentRead|AgentWrite|AgentList }
      Processeur: indexation rapide pour recherche immédiate (voir section 4)
      Émet: EnqueueFileOp (BM25 only, pas d'embedding coûteux)
      Peut aussi émettre: DetectProjectOp (si manifest trouvé en remontant)

Priorité 3 — Catalog bridge (émission vers catalog queue)
├── EnqueueDirectoryOp { absolute_path, name, depth, parent_ref }
│     Processeur: catalog.create("Directory", ...) + catalog.link("CONTAINS", parent, self)
│
├── EnqueueFileOp { absolute_path, name, content, content_hash, dir_ref }
│     Processeur: catalog.create("File", ...) + catalog.link("HAS_FILE", dir, self)
│
├── IngestScopeOp { scope_data, file_ref }
│     Processeur: catalog.create("Scope", ...) + catalog.link("DEFINED_IN", scope, file)
│
├── IngestLibraryOp { name, import_path }
│     Processeur: catalog.create("Library", ...)
│
├── IngestRelationOp { rel_type, from_ref, to_ref }
│     Processeur: catalog.link(rel_type, from, to)
│
└── DrainCatalogOp { }
      Processeur: catalog.drain() — flush tout le catalog
      Émet rien, c'est le terminal
```

### Pourquoi cette granularité ?

**1. Crash recovery.** Si un crash survient pendant ParseProjectOp, au restart :
- Les DownloadGithubOp et ScanTreeOp sont déjà completed → pas de re-download
- ParseProjectOp est marqué "processing" → `reset_processing_items()` → re-parse uniquement ce projet
- Les EnqueueFile/DirectoryOp déjà completed restent → pas de dupliqué

**2. Parallélisme.** Plusieurs ParseProjectOp (différents projets d'un monorepo) peuvent s'exécuter en parallèle.

**3. Observabilité.** Chaque op émet des events → progress bar précise ("Downloaded 3/3 repos, Parsed 12/47 files, Embedded 230/890 chunks").

**4. Incrémental natif.** Un `FileChangedOp` re-entre dans la pipeline au bon endroit, sans re-scanner tout l'arbre.

---

## 4. TouchedSources : cache de recherche live pour l'agent

### Le vrai problème

Un agent de code travaille : il ouvre des fichiers, explore du code, navigue dans l'arborescence. À un moment il cherche "jwt validation" dans ce qu'il a déjà vu. Le full project ingest n'est peut-être même pas lancé, ou il est en cours mais pas terminé. L'agent a besoin de chercher **maintenant** dans ce qu'il a touché.

Ce n'est pas un problème d'orphelin — c'est un problème de **latence entre exploration et recherche**.

### Ce que ragforge-core faisait (et pourquoi c'était bancal)

```typescript
const ORPHAN_PROJECT_ID = 'touched-files';
// Fichier touché → pseudo-projet → parse individuel → embedding
// Problèmes:
// - Parse individuel = pas de résolution inter-fichier (CONSUMES, etc.)
// - Embedding = lent (secondes), l'agent attend
// - Pseudo-projet = jamais rattaché au vrai projet automatiquement
// - Si le full ingest passe après → doublons, migration manuelle
```

### TouchedSources : la vision

**Principe :** quand l'agent touche un fichier (read, write, list), on l'indexe **immédiatement** — BM25 d'abord (synchrone, < 10ms), puis embedding en basse priorité. L'agent peut chercher dedans dans la milliseconde via BM25, et les embeddings arrivent en arrière-plan. Si un full project ingest est en cours, ses embeddings sont **prioritaires** — les embeddings touched passent après.

```
Agent: read_file("src/auth.ts")
    ↓ immédiat (< 10ms)
TouchSourceOp { path: "src/auth.ts", content, trigger: AgentRead }
    ↓ TouchSourceProcessor:
    1. catalog.create("File", { absolute_path, name, content, _touched: true })
       → BM25 indexation synchrone → searchable immédiatement
       → Embedding enqueued en basse priorité (prio 3.5 touched, vs prio 3.0 full ingest)
       → Pas de parse codeparsers (économise le temps de parse AST)
    2. catalog.flush_insertions()  → synchrone, immédiat
    ↓
Agent peut maintenant: catalog.search("FileContentKB", "jwt validation", { signals: BM25 })
    → retrouve src/auth.ts via BM25

    ...en arrière-plan, dans l'order de priorité de la catalog queue...

    Si full ingest en cours:
        Prio 3: EmbedOp (full ingest) — traité d'abord
        Prio 4: EmbedOp (touched)     — traité après
    Si pas de full ingest en cours:
        Prio 4: EmbedOp (touched)     — traité immédiatement

    → File "src/auth.ts" reçoit ses embeddings → searchable via vector aussi
    → L'agent ne voit pas la différence, la qualité des résultats s'améliore en continu

    ...plus tard, le full ingest arrive...

Full ingest (DownloadGithubOp → ScanTreeOp → ParseProjectOp → ...)
    → crée File + Scope + Library + relations
    → embeddings dense + sparse (prio 3, prioritaire)
    → le File "src/auth.ts" est upsert (même _uuid = HASHSAFE(absolute_path))
    → embedding déjà fait par touch? skip si content_hash identique
    → ScopeKB, TreeKB, LibraryKB deviennent searchables
```

### Priorité d'embedding : touched vs full ingest

Le point clé : les embeddings sont **le goulot** (GPU/CPU coûteux). Un full ingest de 500 fichiers ne doit pas être bloqué par 20 fichiers touchés qui veulent leurs embeddings.

```
Catalog Queue — priorités d'embedding (float) :
    Prio 3.0  EmbedOp (full ingest)     ← projet complet, prioritaire
    Prio 3.5  EmbedOp (touched source)  ← fichiers touchés, passe après

Concrètement:
- Agent touche 5 fichiers → 5 EmbedOp prio 3.5 en queue
- Full ingest lance → 890 EmbedOp prio 3.0 en queue
- La catalog queue traite d'abord les 890 prio 3.0 (full ingest)
- Puis les 5 prio 3.5 (touched) — SAUF si l'agent en touche d'autres entre-temps
- Si un fichier touché est ensuite ingéré par le full ingest (même _uuid):
  → l'EmbedOp prio 3.5 pending est annulé (content_hash identique, embedding redondant)

Tri de la queue: (priority: f32, insertion_order: u64)
- Entre prios différentes : la plus basse passe d'abord (3.0 avant 3.5)
- Dans la même prio : FIFO — le premier enqueued passe d'abord
- Backward compatible : les prios existantes passent de u8 à f32 (0→0.0, 1→1.0, etc.)
```

Le changement dans rag3weaver est minimal : `priority: u8 → f32` sur `OperationItem` et `OperationConfig`. Le reste (tri, batch, persistence) fonctionne pareil. Et ça ouvre des nuances futures sans ajouter de complexité :
- `3.0` full ingest en cours
- `3.2` re-ingest incrémental (moins urgent qu'un premier ingest)
- `3.5` touched source
- `3.8` pre-fetch spéculatif (embedding de fichiers proches dans le graph mais pas encore ouverts)

### Qualité de recherche progressive

| Phase | KB disponibles | Signaux | Latence | Qualité |
|-------|---------------|---------|---------|---------|
| **Touched (immédiat)** | FileContentKB | BM25 | < 10ms | Recherche textuelle dans le contenu brut |
| **Touched (après embed)** | FileContentKB | BM25 + vector + sparse | ~100ms par fichier (background) | Recherche sémantique sur contenu brut |
| **Full ingest** | FileContentKB + ScopeKB + TreeKB + LibraryKB | BM25 + vector + sparse | Secondes-minutes total | Recherche sémantique + structurelle + relationnelle |

L'agent commence avec du BM25 brut, obtient les embeddings progressivement en background, et quand le full ingest complète, il a accès au graphe complet avec scopes parsés et relations.

### TouchSourceOp dans la pipeline

```
TouchSourceOp { file_path, content, trigger: AgentRead|AgentWrite|AgentList }
    ↓ TouchSourceProcessor:
    1. INDEX RAPIDE — BM25 synchrone + embedding basse prio
       catalog.create("File", {
           _uuid: HASHSAFE(absolute_path),
           absolute_path, name, extension,
           content,
           content_hash: SHA256(content),
           _touched: true,         // marqueur "pas encore full-ingested"
           _touched_at: now(),
       }, { embed_priority: 3.5 })  // EmbedOp à prio 3.5, pas 3.0
       catalog.flush_insertions()  // synchrone, < 10ms → BM25 searchable
       // EmbedOp prio 3.5 reste en queue → traité après les EmbedOp prio 3.0 (full ingest)

    2. DÉTECTION PROJET — optionnel, asynchrone
       Si c'est le premier fichier touché dans ce répertoire :
       → remonter vers package.json / Cargo.toml / go.mod / .git
       → Si trouvé ET projet pas encore ingéré :
           sender.emit(DetectProjectOp { manifest_path, triggered_by: "touch" })
       → Le full ingest se lance en background
       → L'agent n'attend pas

    3. PARSE LÉGER — optionnel, si temps le permet
       Si le fichier est du code (extension connue) :
       → parse codeparsers en mode single-file (pas de résolution cross-file)
       → enqueue IngestScopeOp pour les scopes trouvés (embed_priority: 3.5)
       → Donne accès à ScopeKB (BM25 immédiat, vector en background)
       → Priorité basse, ne bloque pas le retour au step 1
```

### Cycle de vie d'un fichier touché

```
                    ┌──────────────────┐
    agent touch ──→ │  TOUCHED          │ ← BM25 immédiat, searchable
                    │  _touched: true   │
                    └────────┬─────────┘
                             │ embed prio 3.5 (background)
                             ▼
                    ┌──────────────────┐
                    │  TOUCHED+EMBEDDED │ ← BM25 + vector + sparse
                    │  _touched: true   │   (mais pas de scopes/relations)
                    └────────┬─────────┘
                             │ full ingest arrive (même _uuid)
                             │ catalog.update() ou upsert
                             ▼
                    ┌──────────────────┐
                    │  INGESTED         │ ← BM25 + vector + sparse + relations
                    │  _touched: false  │
                    │  Scopes créés     │
                    │  Relations créées │
                    └──────────────────┘
```

**Pas de migration manuelle.** Le full ingest fait un upsert sur le même `_uuid = HASHSAFE(absolute_path)`. Les données touchées sont enrichies in-place. Si le content_hash n'a pas changé entre le touch et le full ingest, le contenu n'est pas re-indexé (skip BM25 re-index, skip re-chunk).

### Deduplication agent → full ingest

```rust
// Pendant le full ingest (EnqueueFileOp processor):
let uuid = HASHSAFE(absolute_path);
if catalog.entity_exists("File", uuid) {
    let existing = catalog.get("File", uuid);
    if existing.content_hash == new_content_hash {
        // Fichier touché, contenu inchangé → skip re-index BM25
        // Mais on continue : ajouter embeddings, créer Scopes, relations
        catalog.update("File", uuid, { _touched: false, /* + champs enrichis */ });
    } else {
        // Contenu changé depuis le touch → full re-index
        catalog.update("File", uuid, { content: new_content, _touched: false, ... });
    }
} else {
    // Fichier pas touché → create normal
    catalog.create("File", { ... });
}
```

### Garbage collection des touched sources

Les fichiers touchés mais jamais ingérés par un full ingest (vrais fichiers isolés, pas dans un projet) restent dans la DB avec `_touched: true`. Options :
- **TTL** : supprimer après N heures/jours sans re-touch → `_touched_at` permet ça
- **Quota** : garder les N derniers touchés, supprimer les plus anciens
- **Explicite** : l'agent signale "j'ai fini de travailler sur ce contexte" → cleanup
- **Jamais** : les garder — coût storage modéré (BM25 index + embeddings)

Probablement un mix TTL + quota : garder les 1000 derniers touchés, supprimer ceux > 24h sans re-touch. La suppression inclut le cleanup des embeddings et de l'index BM25.

---

## 5. File watching : du polling au event-driven

### Pattern chokidar (ragforge-core)

```
chokidar.watch(paths, { ignoreInitial: true, awaitWriteFinish: { stabilityThreshold: 300ms } })
    → 'add'    : pendingChanges.set(path, 'add')
    → 'change' : pendingChanges.set(path, 'change')
    → 'unlink' : pendingChanges.set(path, 'unlink')

    batchTimer (500ms) → flushBatch()
        → additions/changes → fileStateMachine.markDiscoveredBatch()
        → deletions → deleteFiles()
```

### Transposition en pipeline queue

```
                    ┌──────────────────────────────┐
                    │  OS : inotify / FSEvents      │
                    └──────────────┬───────────────┘
                                   │ raw events
                    ┌──────────────▼───────────────┐
                    │  Debouncer (500ms batch)       │
                    │  Deduplique par path           │
                    │  Dernière action gagne         │
                    └──────────────┬───────────────┘
                                   │ batched events
                ┌──────────────────▼──────────────────┐
                │  FileChangedOp (un par fichier)      │
                │  { path, event, batch_id }           │
                └──────────────────┬──────────────────┘
                                   │
                   ┌───────────────┼───────────────┐
                   │               │               │
              event=Add      event=Change     event=Delete
                   │               │               │
            ScanTreeOp      ReparseFileOp    DeleteEntityOp
            (si nouveau     (diff + update)  (cascade delete)
             répertoire)
```

**Avantage queue :** si l'agent sauvegarde 50 fichiers d'un coup (reformatage, git checkout), le debouncer batch → 50 FileChangedOp → le processeur les groupe par projet → un seul ParseProjectOp batch. Sans queue, 50 parse individuels en parallèle → race conditions.

**Persistence :** le debouncer est in-memory (pas besoin de persister des raw FS events). Mais les FileChangedOp résultants sont persistés dans la pipeline queue → crash-safe.

---

## 6. Pipeline GitHub complète : du endpoint au search

```
POST /api/ingest/github { url: "https://github.com/unjs/defu" }
    │
    ▼
┌─ Pipeline Queue ──────────────────────────────────────────────────┐
│                                                                    │
│  P0: DownloadGithubOp { url, branch: "main", shallow: true }      │
│      → git clone --depth 1 → /tmp/github-ingest-abc123/defu/     │
│      → émet: ScanTreeOp { root: "/tmp/.../defu" }                 │
│                                                                    │
│  P1: ScanTreeOp { root: "/tmp/.../defu" }                         │
│      → walk tree (ignore node_modules, .git, dist)               │
│      → détecte package.json → émet DetectProjectOp               │
│      → émet: 12 × EnqueueDirectoryOp, 47 × EnqueueFileOp        │
│                                                                    │
│  P1: DetectProjectOp { manifest: "package.json", type: npm }      │
│      → parse package.json → dependencies, devDependencies         │
│      → émet: ParseProjectOp { root, files: [47 .ts files] }      │
│                                                                    │
│  P2: ParseProjectOp { root, files: [...], lang: typescript }      │
│      → codeparsers::parse_project(root, files)                    │
│      → 47 fichiers → 230 scopes, 45 relations, 12 libraries      │
│      → émet: 230 × IngestScopeOp, 12 × IngestLibraryOp           │
│              45 × IngestRelationOp                                 │
│                                                                    │
│  P3: EnqueueDirectoryOp × 12                                      │
│      → catalog.create("Directory", ...) × 12                      │
│      → catalog.link("CONTAINS", parent, child) × 11               │
│                                                                    │
│  P3: EnqueueFileOp × 47                                           │
│      → catalog.create("File", ...) × 47                           │
│      → catalog.link("HAS_FILE", dir, file) × 47                   │
│                                                                    │
│  P3: IngestScopeOp × 230                                          │
│      → catalog.create("Scope", ...) × 230                         │
│      → catalog.link("DEFINED_IN", scope, file) × 230              │
│                                                                    │
│  P3: IngestLibraryOp × 12                                         │
│      → catalog.create("Library", ...) × 12                        │
│                                                                    │
│  P3: IngestRelationOp × 45                                        │
│      → catalog.link("CONSUMES"/"INHERITS"/..., from, to) × 45     │
│                                                                    │
│  P3: DrainCatalogOp { }                                            │
│      → catalog.drain()                                             │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
    │
    │ catalog.drain() déclenche la catalog queue :
    ▼
┌─ Catalog Queue (rag3weaver interne) ──────────────────────────────┐
│                                                                    │
│  Prio 0: ChunkProcessor                                           │
│      → 47 File contents → ~200 chunks FileContentKB               │
│      → 230 Scope contents → ~890 chunks ScopeKB                   │
│      → émet: InsertOp (chunks) + EmbedOp                          │
│                                                                    │
│  Prio 1: InsertProcessor                                           │
│      → INSERT 12 Directory + 47 File + 230 Scope + 12 Library     │
│      → INSERT ~1090 chunks                                         │
│                                                                    │
│  Prio 2: LinkProcessor                                             │
│      → CREATE 11 CONTAINS + 47 HAS_FILE + 230 DEFINED_IN          │
│        + 45 CONSUMES/INHERITS/... + 12 USES_LIBRARY               │
│                                                                    │
│  Prio 3: EmbedProcessor                                            │
│      → embed TreeKB (12+47 paths, dense)                           │
│      → embed FileContentKB (~200 chunks, dense+sparse)             │
│      → embed ScopeKB (~890 chunks, dense+sparse)                   │
│      → embed LibraryKB (12, dense)                                 │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
    │
    ▼
Searchable: 4 KBs, 301 entités, 345 relations, ~1090 chunks, ~2180 embeddings
```

---

## 7. Events et observabilité

### Pipeline Events

```rust
enum PipelineEvent {
    // Lifecycle
    OpEnqueued { id, op_type, priority },
    OpStarted { id, op_type },
    OpCompleted { id, op_type, duration_ms, emitted_count },
    OpFailed { id, op_type, error, will_retry: bool },

    // Progress
    BatchProgress { op_type, completed: usize, total: usize },

    // Milestones
    DownloadCompleted { source, files_count, bytes },
    ScanCompleted { directories: usize, files: usize, projects: usize },
    ParseCompleted { project, scopes: usize, relations: usize },
    IngestCompleted { entities: usize, relations: usize, chunks: usize },

    // Pipeline-level
    PipelineStarted { source_description },
    PipelineCompleted { stats: PipelineStats },
    PipelineFailed { error, completed_ops: usize, pending_ops: usize },
}
```

### Utilisation côté UI

```
POST /api/ingest/github { url: "..." }
    → 202 Accepted, { pipeline_id: "pip_abc123" }

GET /api/pipelines/pip_abc123/events (SSE)
    → data: { type: "DownloadCompleted", files_count: 47 }
    → data: { type: "BatchProgress", op_type: "ParseProjectOp", completed: 12, total: 47 }
    → data: { type: "ParseCompleted", scopes: 230 }
    → data: { type: "BatchProgress", op_type: "EmbedProcessor", completed: 450, total: 1090 }
    → data: { type: "PipelineCompleted", stats: { duration_ms: 12450, entities: 301, ... } }
```

---

## 8. Persistence et crash recovery

### Quoi persister

| Donnée | Où | Pourquoi |
|--------|-----|---------|
| Pipeline ops (pending/processing) | table `_PipelineOp` dans rag3db | Reprise après crash |
| Catalog ops (pending/processing) | table `_Operation` (existant) | Reprise après crash |
| Fichiers téléchargés | `/tmp/` ou storage dédié | Re-download évitable via hash |
| Résultat parse codeparsers | pas persisté, re-calculable | Rapide, déterministe |
| Résultat embedding | table entité (existant) | Coûteux, persisté immédiatement |

### Table `_PipelineOp`

```sql
CREATE NODE TABLE _PipelineOp (
    uuid STRING PRIMARY KEY,
    pipeline_id STRING,       -- groupe d'ops (ex: "pip_abc123" pour un ingest github)
    op_type STRING,           -- "DownloadGithubOp", "ParseProjectOp", etc.
    priority DOUBLE,            -- f32 en Rust, DOUBLE en Cypher
    state STRING,             -- "pending", "processing", "completed", "failed"
    payload STRING,           -- JSON sérialisé de l'op
    depends_on STRING[],      -- UUIDs des ops dont celle-ci dépend
    emitted_by STRING,        -- UUID de l'op parente qui a émis celle-ci
    created_at INT64,
    started_at INT64,
    completed_at INT64,
    error STRING,
    retry_count INT64
)
```

### Recovery flow

```
Au démarrage :
1. Charger _PipelineOp WHERE state IN ('pending', 'processing', 'persisted')
2. Les ops 'processing' → reset à 'pending' (crash mid-execution)
3. Les ops 'pending' → re-enqueue dans la pipeline queue
4. Les ops 'completed' → ignorer (déjà fait)
5. Les ops 'failed' avec retry_count < max → re-enqueue

Invariant: une op est idempotente ou protégée par un guard :
  - DownloadGithubOp : si /tmp/.../ existe déjà et hash match → skip download
  - EnqueueFileOp : catalog.create() avec _uuid = HASHSAFE → upsert
  - ParseProjectOp : déterministe sur les mêmes inputs
```

---

## 9. Abstraction DomainPipeline

### Trait pour les domaines

```rust
/// Un domaine (Code, Drive, Shopify, Mail, etc.) implémente ce trait
/// pour définir ses propres opérations et processeurs de pipeline.
trait DomainPipeline: Send + Sync {
    /// Nom du domaine
    fn name(&self) -> &str;

    /// Enregistre les processeurs pour chaque type d'op
    fn register_processors(&self, queue: &mut PipelineQueue);

    /// Point d'entrée : crée les ops initiales
    fn create_initial_ops(&self, source: SourceConfig) -> Vec<PipelineOp>;

    /// Référence au catalog pour les ops bridge (prio 3)
    fn catalog(&self) -> &Catalog;
}
```

### Implémentation Code domain

```rust
struct CodeDomainPipeline {
    catalog: Catalog,
    parser_config: CodeparserConfig,
}

impl DomainPipeline for CodeDomainPipeline {
    fn name(&self) -> &str { "code" }

    fn register_processors(&self, queue: &mut PipelineQueue) {
        queue.register("DownloadGithubOp", Box::new(GithubDownloader::new()));
        queue.register("WatchLocalOp", Box::new(LocalWatcher::new()));
        queue.register("ScanTreeOp", Box::new(TreeScanner::new()));
        queue.register("DetectProjectOp", Box::new(ProjectDetector::new()));
        queue.register("ParseProjectOp", Box::new(ProjectParser::new(self.parser_config)));
        queue.register("TouchSourceOp", Box::new(TouchSourceHandler::new()));
        queue.register("ReparseFileOp", Box::new(FileReparser::new()));
        // Bridge ops (prio 3) → catalog.create/link
        queue.register("EnqueueDirectoryOp", Box::new(CatalogBridge::new(&self.catalog)));
        queue.register("EnqueueFileOp", Box::new(CatalogBridge::new(&self.catalog)));
        queue.register("IngestScopeOp", Box::new(CatalogBridge::new(&self.catalog)));
        queue.register("IngestLibraryOp", Box::new(CatalogBridge::new(&self.catalog)));
        queue.register("IngestRelationOp", Box::new(CatalogBridge::new(&self.catalog)));
        queue.register("DrainCatalogOp", Box::new(CatalogDrainer::new(&self.catalog)));
    }

    fn create_initial_ops(&self, source: SourceConfig) -> Vec<PipelineOp> {
        match source {
            SourceConfig::GitHub { url, branch } => {
                vec![PipelineOp::new("DownloadGithubOp", 0, json!({ "url": url, "branch": branch }))]
            }
            SourceConfig::Local { root, watch } => {
                let mut ops = vec![PipelineOp::new("ScanTreeOp", 1, json!({ "root": root }))];
                if watch {
                    ops.push(PipelineOp::new("WatchLocalOp", 0, json!({ "root": root })));
                }
                ops
            }
            SourceConfig::SingleFile { path } => {
                vec![PipelineOp::new("TouchSourceOp", 2, json!({ "path": path, "trigger": "AgentRead" }))]
            }
        }
    }

    fn catalog(&self) -> &Catalog { &self.catalog }
}
```

### Utilisation

```rust
let catalog = Catalog::new(db, schema, embedder);
let pipeline = CodeDomainPipeline::new(catalog, parser_config);

let mut queue = PipelineQueue::new();
pipeline.register_processors(&mut queue);

// Ingest GitHub
let ops = pipeline.create_initial_ops(SourceConfig::GitHub {
    url: "https://github.com/unjs/defu".into(),
    branch: "main".into(),
});
for op in ops {
    queue.enqueue(op);
}
queue.drain().await?;

// Ingest local avec watch
let ops = pipeline.create_initial_ops(SourceConfig::Local {
    root: "/home/user/project".into(),
    watch: true,
});
// ...
```

---

## 10. Autres domaines : même pattern

### Google Drive domain

```
DownloadDriveOp { folder_id, recursive }
    → ScanTreeOp (même qu'en code! réutilisé)
        → EnqueueDirectoryOp (même! réutilisé)
        → EnqueueFileOp (même! réutilisé)
    → DetectDocTypeOp { file_id, mime_type }
        → OCRDocumentOp { file_id, provider: tesseract|gemini }
        → ParseMarkdownOp { file_id }
        → ParseSpreadsheetOp { file_id }
```

### Shopify domain

```
FetchProductsOp { shop_url, cursor }
    → EnqueueProductOp × N
        → catalog.create("Product", ...)
    → FetchCollectionsOp { shop_url }
        → EnqueueCollectionOp × N
            → catalog.create("Collection", ...)
            → catalog.link("IN_COLLECTION", product, collection)
    → DrainCatalogOp
```

### Gmail domain

```
SyncMailboxOp { email, since_history_id }
    → FetchThreadsOp { thread_ids: [...] }
        → EnqueueThreadOp × N
            → EnqueueMailOp × N (mails du thread)
                → catalog.create("Mail", ...)
                → catalog.link("IN_THREAD", mail, thread)
                → DetectAttachmentOp × N
                    → OCRDocumentOp (si PDF/image)
    → DrainCatalogOp
```

**Pattern récurrent :** Acquisition (P0) → Scan/Detect (P1) → Parse/Transform (P2) → Bridge Catalog (P3). Les ops bridge (EnqueueDirectoryOp, EnqueueFileOp) sont **réutilisables** entre domaines.

---

## 11. PipelineQueue vs OperationQueue : quoi réutiliser ?

### Ce qui est identique

| Feature | OperationQueue (catalog) | PipelineQueue |
|---------|--------------------------|---------------|
| Priority ordering | Oui (0.0→3.0, f32) | Oui (0.0→3.0+, f32) |
| Batch processing | Oui (batch_size par type) | Oui |
| Processor trait | `Processor { process(items, sender) }` | Même trait |
| Event emission | QueueEvent | PipelineEvent (similaire) |
| Persistence trait | OperationPersistence | Même trait |
| Expansion (emit downstream) | Oui, prio > source | Oui, prio > source |
| Failed items + retry | Oui, max_retries par type | Oui |

### Ce qui diffère

| Feature | OperationQueue | PipelineQueue |
|---------|----------------|---------------|
| Op enum | CatalogOp (6 variantes, f32 prio) | PipelineOp (dynamique, string-typed, f32 prio) |
| Processeurs | Built-in (5 processors) | Registrés par domaine |
| Durée des ops | Ms (insert, embed) | Secondes-minutes (download, parse) |
| Concurrence | Sequential dans une prio | Parallel possible dans une prio |
| Pipeline ID | N/A | Chaque pipeline a un ID trackable |
| Dépendances | Implicites (prio) | Explicites possibles (depends_on) |

### Proposition

**Factoriser** `OperationQueue` en un générique `Queue<Op>` :

```rust
struct Queue<Op: QueueOp> {
    items: Vec<QueueItem<Op>>,
    processors: HashMap<String, Box<dyn Processor<Op>>>,
    persistence: Option<Box<dyn Persistence<Op>>>,
    event_bus: EventBus<QueueEvent>,
}

trait QueueOp: Send + Sync {
    fn op_type(&self) -> &str;
    fn priority(&self) -> f32;
    fn config(&self) -> OperationConfig;
}
```

Puis :
- `type CatalogQueue = Queue<CatalogOp>;` (existant, inchangé)
- `type PipelineQueue = Queue<PipelineOp>;` (nouveau, string-typed)

---

## 12. Questions ouvertes

1. **Quand appeler catalog.drain() ?** Après chaque op bridge ? Ou une seule fois à la fin (DrainCatalogOp) ? Le drain batch est plus efficace (un seul pass embed), mais les entités ne sont searchables qu'après drain. Pour un ingest github one-shot → drain final. Pour un watcher continu → drain périodique (toutes les N ops ou tous les M secondes).

2. **Concurrence des pipeline ops.** Plusieurs ParseProjectOp (monorepo) peuvent-ils s'exécuter en parallèle ? Oui si les scopes qu'ils créent n'ont pas de relations croisées. En pratique, un ParseProjectOp par workspace member est safe. Mais les IngestRelationOp cross-projet doivent attendre la fin des deux ParseProjectOp → dépendances explicites.

3. **Où stocker les fichiers téléchargés ?** `/tmp/` est volatile (reboot = perdu). Mieux vaut un répertoire dédié dans le data dir de rag3db : `{db_path}/../pipeline_cache/github/{owner}/{repo}/`. Avec cleanup après ingestion réussie (ou rétention configurable).

4. **WatchLocalOp : processus long.** C'est un daemon, pas une op one-shot. Comment le modéliser dans une queue d'ops ? Options :
    - Op spéciale "long-running" qui n'est jamais "completed" et émet des FileChangedOp en continu
    - Hors queue : le watcher est un composant séparé qui injecte des FileChangedOp dans la queue
    - Probablement le 2e — le watcher est un **producteur** externe à la queue, pas un consommateur

5. **Granularité des ops bridge.** 230 × IngestScopeOp = 230 items dans la queue. Trop granulaire ? Alternative : un seul `IngestBatchScopesOp { scopes: Vec<ScopeData> }` qui appelle catalog.create() 230 fois en boucle. Moins d'overhead queue, mais perd la recovery granulaire. Probablement un batch op avec un batch_size large (1000+) est le bon compromis.

6. **Pipeline inter-domaines.** Un mail contient un lien GitHub → on veut ingérer le repo. Le MailDomainPipeline détecte l'URL → émet un DownloadGithubOp dans la CodeDomainPipeline. Comment cross-pipeline ? Probablement un event `CrossDomainTrigger { target_domain, source_config }` que l'orchestrateur route.

7. **Déduplication cross-source.** Le même fichier ingéré localement ET via GitHub (paths différents : `/home/user/defu/src/index.ts` et `/virtual/unjs/defu/src/index.ts`). Deux entités File distinctes ? Ou déduplication par content_hash ? Probablement deux entités — l'absolute_path est la clé, pas le contenu. Mais une relation `SAME_CONTENT_AS` pourrait lier les deux.

8. **TouchedSources : annulation d'EmbedOp redondants.** Quand le full ingest arrive et upsert un fichier déjà touché (même _uuid, même content_hash), l'EmbedOp prio 3.5 (touched) en queue est redondant — le full ingest va produire le même embedding en prio 3.0. Comment annuler les EmbedOp touched pendants ? Options : (a) le processeur EmbedOp vérifie si l'entité a déjà un embedding à jour avant de le recalculer, (b) un mécanisme d'annulation explicite dans la queue (`queue.cancel(op_id)`), (c) on laisse le doublon passer — l'upsert embedding est idempotent, juste du CPU gaspillé.

9. **TouchedSources : parse léger scope.** Le step 3 optionnel (parse codeparsers single-file) donne accès à ScopeKB avant le full ingest. Mais le parse single-file ne résout pas les relations cross-file (CONSUMES, etc.). Les scopes touchés sont "isolés" — pas de graph navigable. Acceptable ? Ou confusant pour l'agent qui voit des scopes sans relations ?

10. **TouchedSources : TTL vs absorption.** Quand le full ingest passe et absorbe un fichier touché (upsert sur même _uuid), les champs `_touched` / `_touched_at` sont écrasés. Mais si le full ingest ne couvre pas ce fichier (fichier hors projet), combien de temps le garder ? Le TTL doit-il être par fichier ou global ?

11. **TouchedSources : quels triggers ?** `AgentRead` (l'agent lit un fichier), `AgentWrite` (l'agent modifie), `AgentList` (l'agent liste un répertoire → toucher tous les fichiers listés ?). Lister un répertoire de 500 fichiers ne devrait probablement pas tous les indexer — seulement ceux que l'agent ouvre ensuite. Le trigger `AgentList` est peut-être de trop.
