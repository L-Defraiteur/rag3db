# 06 — Cahier des charges : Code Domain

Roadmap d'implémentation du domaine Code dans rag3weaver, organisée en phases. Chaque phase est livrable et testable indépendamment. L'ordre suit la logique bottom-up : on construit les fondations catalog d'abord, puis on remonte vers les sources.

Réf : doc 04 (vision entités/KB/relations), doc 05 (pipeline queues).

---

## Phase 0 — Pré-requis rag3weaver

Avant de toucher au domaine Code, quelques extensions nécessaires dans rag3weaver core.

### 0a. Cross-entity KB search

**Actuellement :** `Catalog::search()` ne query que la `title.entity` d'une KB. Si la KB a `entities: {Directory, File}`, seul Directory (le title entity) est cherché.

**Nécessaire :** quand `kb.entities.len() > 1`, lancer une recherche par entité et fusionner les résultats (option B du doc 04 Q11). Même pattern que cross-KB search du doc 03.

**Livrable :** `catalog.search("TreeKB", "auth tests")` retourne des Directory ET des File.

### 0b. Priorités float dans la catalog queue

**Actuellement :** `OperationItem.priority` est un `u8`. EmbedOp est toujours prio 3.

**Nécessaire :** passer `priority: u8 → f32` pour permettre des nuances de priorité sans inventer de nouveaux niveaux entiers. Le tri devient `(priority: f32, insertion_order: u64)` — backward compatible (les prios existantes passent de 0/1/2/3 à 0.0/1.0/2.0/3.0).

**Use cases immédiats :**
- `3.0` EmbedOp full ingest (prioritaire)
- `3.5` EmbedOp touched source (passe après)

**Use cases futurs :**
- `3.2` re-ingest incrémental
- `3.8` pre-fetch spéculatif

**Livrable :** `catalog.create("File", data, { embed_priority: 3.5 })` enqueue l'embedding en prio 3.5. Dans la même prio, FIFO : le premier enqueued passe d'abord.

### 0c. PipelineQueue (optionnel pour Phase 1-3)

**Actuellement :** `OperationQueue` est typé sur `CatalogOp` (enum hardcodé).

**Nécessaire à terme :** factoriser en `Queue<Op>` générique pour réutiliser la mécanique (priorité, persistence, retry, events) avec des ops domaine. Cf doc 05 section 11.

**Pas bloquant pour commencer** — les premières phases peuvent être implémentées comme des fonctions séquentielles appelées manuellement, sans pipeline queue. La queue viendra quand on aura besoin de persistence/crash recovery/observabilité.

---

## Phase 1 — Schema + Bridge catalog (correspond aux ops P3)

Objectif : définir le schema YAML, créer les entités et relations dans rag3db via `catalog.create()` / `catalog.link()`, vérifier que le search fonctionne.

### 1a. Schema YAML Code Domain

Définir la config rag3weaver complète :
- 4 entités : Directory, File, Scope, Library
- 5+ relations : CONTAINS, HAS_FILE, DEFINED_IN, PARENT_OF, CONSUMES, INHERITS_FROM, IMPLEMENTS, DECORATES, USES_LIBRARY
- 4 KBs : TreeKB (cross-entity Directory+File), FileContentKB, ScopeKB (hybrid 3-way), LibraryKB
- Filter fields, chunking config par KB

**Livrable :** fichier `code_domain_schema.yaml` validé par `Catalog::new()`.

### 1b. Création manuelle d'entités de test

Script/test Rust qui :
1. Crée un Catalog avec le schema Code Domain
2. Crée manuellement quelques entités (2 Directory, 5 File, 10 Scope, 3 Library)
3. Crée les relations entre elles
4. Appelle `catalog.drain()`
5. Vérifie que le search fonctionne sur chaque KB

C'est l'équivalent du "Phase 0 CRUD" qu'on a fait pour rag3weaver — valider que le schema marche avant d'automatiser.

**Livrable :** test E2E qui passe, search fonctionnel sur les 4 KBs.

### 1c. Scope synthétique "file" pour oversized

Valider que le pattern fichier-entier-comme-scope marche :
1. Créer un File avec un gros content (ex: 10000 lignes)
2. Créer un Scope de type `"file"` avec le même content
3. `DEFINED_IN` entre le Scope et le File
4. Vérifier que ScopeKB retourne des chunks de ce gros fichier

**Livrable :** test qui prouve que le fallback oversized fonctionne via ScopeKB.

---

## Phase 2 — Codeparsers integration (correspond aux ops P2)

Objectif : parser du vrai code avec codeparsers Rust et convertir le résultat en entités rag3weaver.

### 2a. Adaptateur codeparsers → entités

Fonction `parse_and_ingest(catalog, files, config)` qui :
1. Appelle `ProjectParser::parse_project()` avec les fichiers
2. Convertit les `ScopeFileAnalysis` en entités Scope rag3weaver
3. Convertit les `ResolvedRelationship` en relations rag3weaver
4. Gère le triage oversized/ignore (config `max_parse_size_bytes`)
5. Crée les Scope synthétiques "file" pour les oversized
6. Construit les `member_summary` pour les containers
7. Extrait les Library depuis les imports externes
8. Appelle `catalog.drain()`

**Inputs :** liste de `(absolute_path, content)` + config.
**Output :** `DrainResult` (stats entités/relations/chunks créés).

### 2b. Mapping codeparsers → rag3weaver

| codeparsers | rag3weaver | Notes |
|-------------|-----------|-------|
| `ScopeFileAnalysis.scopes[]` | Scope entity | Un par scope extrait |
| `scope.scope_type` | `Scope.scope_type` | "function", "class", "method", etc. |
| `scope.content_dedented` | `Scope.content` | Dedented pour chunking/embedding |
| `scope.signature` | `Scope.signature` | titleFor ScopeKB |
| `scope.docstring` | `Scope.docstring` | contentFor ScopeKB |
| `scope.children` → parent | `PARENT_OF` relation | Container → enfant |
| `ResolvedRelationship::Consumes` | `CONSUMES` relation | Appel/utilisation |
| `ResolvedRelationship::InheritsFrom` | `INHERITS_FROM` relation | Héritage |
| `ResolvedRelationship::Implements` | `IMPLEMENTS` relation | Interface impl |
| `ResolvedRelationship::Decorates` | `DECORATES` relation | Décorateurs |
| `scope.imports[].module` (externe) | Library entity + `USES_LIBRARY` | Packages npm/pip/cargo |

### 2c. UUID et hash

- Scope UUID : `HASHSAFE(absolute_path + signature)` — stable si code reformaté
- File UUID : `HASHSAFE(absolute_path)` — stable tant que le fichier ne bouge pas
- Library UUID : `HASHSAFE(name)` — unique par package
- Directory UUID : `HASHSAFE(absolute_path)`

### 2d. member_summary pour containers

Pour chaque Scope de type container (class, interface, enum, namespace, module, struct, trait) :
- Lister les enfants directs (PARENT_OF)
- Formater : `"Members:\n  - signature1 (L16-20)\n  - signature2 (L22-45)\n  ..."`
- Stocker dans `Scope.member_summary` (contentFor ScopeKB)

### 2e. Tests

- Parser un petit projet TypeScript (5-10 fichiers) → vérifier entités/relations créées
- Parser un projet Rust (Cargo workspace) → vérifier scopes/relations
- Fichier oversized → vérifier scope synthétique "file"
- Fichier ignoré → vérifier absence d'entité
- Search sur ScopeKB → retrouver une fonction par intention
- Search sur TreeKB → retrouver un fichier par path
- Explore depuis un Scope → naviguer CONSUMES, DEFINED_IN

---

## Phase 3 — Arborescence + scan (correspond aux ops P1)

Objectif : scanner une arborescence de fichiers (locale ou téléchargée) et créer les entités Directory/File.

### 3a. Tree scanner

Fonction `scan_and_ingest_tree(catalog, root_path, config)` qui :
1. Walk récursif de `root_path`
2. Filtre selon config (ignore patterns : `node_modules`, `.git`, `dist`, `*.min.js`, etc.)
3. Crée les Directory entities avec relations CONTAINS
4. Crée les File entities avec relations HAS_FILE
5. Lit le contenu de chaque fichier
6. Retourne la liste des fichiers (pour passer à Phase 2)

### 3b. Détection de projet

Fonction `detect_projects(root_path)` qui :
1. Cherche les manifests (package.json, Cargo.toml, go.mod, pyproject.toml, etc.)
2. Identifie les workspace members (monorepo)
3. Retourne une liste de `ProjectInfo { root, manifest_type, files }`

Utile pour grouper les fichiers par projet avant l'appel codeparsers (qui a besoin du project root pour résoudre les imports).

### 3c. FileFilterConfig

Implémenter la config de filtrage :
- `max_parse_size_bytes` (défaut 500KB)
- `OversizedStrategy` (Ignore / FallbackChunk)
- Overrides par pattern glob
- Ignore patterns (`.gitignore`-style)

### 3d. Tests

- Scanner un répertoire de test → vérifier Directory/File créés
- Ignorer node_modules, .git → vérifier absence
- Détecter package.json → ProjectInfo correct
- Monorepo (Cargo workspace) → détecter les workspace members

---

## Phase 4 — Sources (correspond aux ops P0)

Objectif : acquérir du code depuis des sources externes.

### 4a. GitHub download

Fonction `download_github(url, branch, config)` qui :
1. Clone shallow (`--depth 1`) ou télécharge ZIP via API GitHub
2. Stocke dans un répertoire temporaire ou cache dédié
3. Attribue les chemins virtuels : `/virtual/<owner>/<repo>/...`
4. Retourne le root path pour passer à Phase 3

### 4b. Local source

Fonction `ingest_local(root_path, config)` qui :
1. Utilise directement le filesystem (pas de copie)
2. Les chemins absolus sont les vrais chemins
3. Passe directement à Phase 3

### 4c. Pipeline complète

Orchestration `ingest_code(source_config)` qui chaîne tout :
```
source_config → download/local → scan_tree → detect_projects → parse_and_ingest → drain
```

À ce stade c'est encore une fonction séquentielle. La PipelineQueue (doc 05) viendra dans une phase ultérieure pour ajouter persistence, crash recovery, observabilité.

### 4d. Tests

- Ingest d'un petit repo GitHub public (ex: unjs/defu)
- Ingest local d'un répertoire de test
- Pipeline complète : download → scan → parse → search fonctionnel

---

## Phase 5 — TouchedSources

Objectif : permettre à un agent de chercher dans les fichiers qu'il a explorés, avant le full ingest.

### 5a. TouchSource API

Fonction `touch_source(catalog, file_path, content, trigger)` qui :
1. Crée/upsert File avec `_touched: true`, `_touched_at: now()`
2. BM25 indexation synchrone (flush_insertions)
3. Embedding enqueued en basse prio (prio 3.5)
4. Optionnel : parse léger codeparsers single-file → Scope synthétiques

### 5b. Détection projet depuis touch

Quand un fichier est touché :
1. Remonter l'arborescence → chercher un manifest
2. Si trouvé et projet pas encore ingéré → déclencher full ingest en background
3. Le fichier touché est immédiatement searchable, le full ingest arrive après

### 5c. Absorption par full ingest

Quand le full ingest traite un fichier déjà touché (même `_uuid = HASHSAFE(absolute_path)`) :
- Si `content_hash` identique → skip re-index BM25, mais ajouter embeddings/scopes/relations
- Si `content_hash` différent → full re-index
- Mettre `_touched: false`

### 5d. Garbage collection

- TTL configurable (défaut 24h) sur les fichiers `_touched: true` non absorbés
- Quota (défaut 1000 fichiers touchés max)
- Cleanup : supprime File + Scope synthétiques + chunks + embeddings

### 5e. Tests

- Touch fichier → BM25 search immédiat
- Touch fichier → embedding arrive en background → vector search fonctionne
- Full ingest absorbe le touché → `_touched: false`, scopes/relations ajoutés
- Touch pendant full ingest → embedding prio 3.5 passe après prio 3.0

---

## Phase 6 — File watching

Objectif : surveiller un répertoire local et re-ingérer automatiquement les fichiers modifiés.

### 6a. Watcher

Composant qui :
1. Surveille un répertoire (inotify/FSEvents)
2. Debounce les events (500ms)
3. Émet des FileChangedOp batché

### 6b. Incrémental

Sur file change :
- Comparer `content_hash` → skip si inchangé
- Re-parse codeparsers le fichier modifié
- Diff les scopes (nouveau/modifié/supprimé) par UUID
- Update `member_summary` des containers parents impactés
- `catalog.update()` / `catalog.create()` / `catalog.delete()` selon le diff

Sur file delete :
- Cascade delete : File + Scopes associés + chunks + relations

### 6c. Tests

- Modifier un fichier → scopes mis à jour
- Ajouter un fichier → nouveaux scopes créés
- Supprimer un fichier → cascade delete
- Renommer un fichier → ancien supprimé, nouveau créé (UUID basé sur path)

---

## Phase 7 — PipelineQueue + observabilité

Objectif : remplacer les appels séquentiels par une vraie pipeline queue avec persistence et events.

### 7a. Queue générique

Factoriser `OperationQueue` en `Queue<Op>` générique. Implémenter `PipelineQueue` avec des ops string-typed enregistrables par domaine.

### 7b. Ops Code Domain

Enregistrer les processeurs pour chaque op (cf doc 05 section 3) :
- P0 : DownloadGithubOp, WatchLocalOp, FileChangedOp
- P1 : ScanTreeOp, DetectProjectOp
- P2 : ParseProjectOp, ReparseFileOp, TouchSourceOp
- P3 : EnqueueDirectoryOp, EnqueueFileOp, IngestScopeOp, IngestLibraryOp, IngestRelationOp, DrainCatalogOp

### 7c. Persistence `_PipelineOp`

Table dans rag3db pour persister les ops en cours. Recovery au restart.

### 7d. Events + SSE

PipelineEvent → EventBus → endpoint SSE pour suivi de progrès côté UI/agent.

### 7e. Tests

- Pipeline GitHub complète via queue
- Crash mid-pipeline → recovery au restart
- Events émis correctement à chaque étape

---

## Phase 8 — DomainPipeline trait + abstraction

Objectif : abstraire le pattern pour que d'autres domaines (Drive, Shopify, Mail) puissent réutiliser la même mécanique.

### 8a. Trait DomainPipeline

```rust
trait DomainPipeline: Send + Sync {
    fn name(&self) -> &str;
    fn register_processors(&self, queue: &mut PipelineQueue);
    fn create_initial_ops(&self, source: SourceConfig) -> Vec<PipelineOp>;
}
```

### 8b. Ops bridge réutilisables

Extraire les ops communes (EnqueueDirectoryOp, EnqueueFileOp, DrainCatalogOp) pour qu'elles soient partagées entre domaines.

### 8c. CodeDomainPipeline

Wrapper qui implémente DomainPipeline pour le domaine Code, encapsulant tout ce qui a été fait en Phases 1-7.

---

## Résumé des phases

| Phase | Quoi | Dépend de | Testable seul | Effort estimé |
|-------|------|-----------|:-------------:|:---:|
| **0** | Pré-requis rag3weaver (cross-entity KB, embed prio) | — | oui | moyen |
| **1** | Schema + bridge catalog (CRUD manuel) | 0 | oui | léger |
| **2** | Codeparsers → entités rag3weaver | 1 | oui | lourd |
| **3** | Tree scan + détection projet | 1 | oui | moyen |
| **4** | Sources (GitHub, local) + pipeline séquentielle | 2, 3 | oui | moyen |
| **5** | TouchedSources (cache live agent) | 1, 0b | oui | moyen |
| **6** | File watching + incrémental | 2, 3 | oui | lourd |
| **7** | PipelineQueue + persistence + events | 4, 5, 6 | oui | lourd |
| **8** | DomainPipeline trait + abstraction | 7 | oui | léger |

**Chemin critique :** 0 → 1 → 2 → 4 (pipeline séquentielle fonctionnelle).
**Parallélisable :** Phase 3 en parallèle de Phase 2 (pas de dépendance directe). Phase 5 en parallèle de Phase 3-4.

---

## Ce qui N'EST PAS dans ce cahier des charges

- **Autres domaines** (Drive, Shopify, Mail, Notion) — doc 02/03 pour la vision, implémentation ultérieure
- **Cross-KB search** (doc 03 section 2) — utile mais pas critique pour le domaine Code seul
- **Graph-aware reranking** (doc 03 section 6) — optimisation future
- **Schema inference automatique** (doc 03 section 4) — le schema Code est défini manuellement
- **SEARCH() dans WHERE** (plan existant) — orthogonal, peut avancer en parallèle
- **WASM** — build WASM de rag3weaver, phase séparée

---

## Questions à trancher avant de commencer

Reprises des questions ouvertes du doc 04 qui impactent l'implémentation :

1. **File UUID** : `HASHSAFE(absolute_path)` seul — confirmé ? (doc 04 Q1)
2. **Scope blocks** (if/for/try) : filtrer complètement ou garder ? (doc 04 Q5)
3. **Scope content** : dedented ou original ? (doc 04 Q10)
4. **Seuil gros fichiers** : 500KB défaut ok ? (doc 04 Q13)
5. **Ignore list** : où configurer ? .rag3ignore ? (doc 04 Q15)
6. **FileContentKB chunking** : taille de chunk pour du code brut ? (doc 04 Q7)
