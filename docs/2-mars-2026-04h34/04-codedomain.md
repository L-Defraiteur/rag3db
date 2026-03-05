# 04 — Vision : Code Domain pour rag3weaver

Architecture du domaine Code (ingestion GitHub / local) dans rag3weaver. Ce document est une vision, pas un plan d'implémentation.

---

## Entités

### File

Représente un fichier physique dans l'arborescence. Abstraction partagée avec Google Drive et tout autre source hiérarchique.

```
File {
    _uuid: HASHSAFE(absolute_path + content_hash)   // ou + last_modified_at, à définir
    absolute_path: String       // TOUJOURS absolu, jamais relatif
    name: String                // basename (ex: "auth.ts")
    extension: String           // ".ts", ".py", ".rs"
    language: String            // "typescript", "python", "rust"
    lines_of_code: Int64
    content: String             // texte brut intégral du fichier
    content_hash: String        // SHA-256 du content (incrémental)
}
```

**Chemins absolus — convention :**
- GitHub ingéré : `/virtual/<owner>/<repo>/src/auth.ts`
- Code local : `/home/luciedefraiteur/project/src/auth.ts`
- Google Drive : `/virtual/drive/<drive_id>/Documents/rapport.pdf`

Les chemins relatifs n'existent qu'au moment de l'affichage / formatage de résultats, jamais dans la DB.

**KBs :**
- **FileContentKB** — hybrid (BM25 + embeddings), pour chercher dans le contenu des fichiers.
  - `titleFor: name`
  - `contentFor: content` (texte brut intégral, chunked)
  - Embeddings sur le contenu → recherche sémantique ("fichiers qui gèrent l'authentification")
- **TreeKB** — KB partagée avec Directory (voir ci-dessous).
  - `contentFor: name` + `absolute_path`

### Directory

Représente un répertoire dans l'arborescence. Abstraction partagée avec Google Drive.

```
Directory {
    _uuid: HASHSAFE(absolute_path)
    absolute_path: String       // /virtual/owner/repo/src/
    name: String                // "src"
    depth: Int64                // profondeur dans l'arborescence
}
```

**KB :**
- **TreeKB** — KB **cross-entity** partagée avec File, pour naviguer l'arborescence par nom ou chemin.
  - `titleFor: name` (sur Directory — le titre de la KB vient de Directory)
  - `contentFor: absolute_path` (sur Directory)
  - `contentFor: name` + `absolute_path` (sur File)
  - Une recherche "auth middleware" sur TreeKB retourne **à la fois** des Directory et des File

### TreeKB — KB cross-entity (File + Directory)

C'est le cas d'usage concret d'une KB indépendante d'une seule entité. Rag3weaver supporte déjà `KBMetadata.entities: HashSet<String>` — une KB peut agréger des champs de plusieurs entités, à condition qu'une relation existe entre elles (ici : `HAS_FILE` entre Directory et File).

```yaml
# Dans la config schema :
entities:
  Directory:
    fields:
      name:          { type: text, title_for: TreeKB }
      absolute_path: { type: text, content_for: [TreeKB] }
  File:
    fields:
      name:          { type: text, content_for: [TreeKB], title_for: FileContentKB }
      absolute_path: { type: text, content_for: [TreeKB] }
      content:       { type: text, content_for: [FileContentKB], chunked: true }

relations:
  HAS_FILE: { from: Directory, to: File }
```

**Résultat :** TreeKB a `entities: {Directory, File}`. Un search("TreeKB", "tests auth") peut retourner :
- Le répertoire `/virtual/owner/repo/tests/auth/` (match sur path)
- Le fichier `/virtual/owner/repo/src/auth.test.ts` (match sur name)

**Limitation actuelle :** le search de rag3weaver ne query aujourd'hui que la `title.entity` (ici Directory). Pour un vrai cross-entity, il faudra étendre le search avec un UNION sur les chunk tables de toutes les entités de la KB. C'est une évolution à prévoir — l'architecture est prête, le search query builder ne l'implémente pas encore.

### Scope

Représente un élément structurel du code extrait par codeparsers : function, class, method, interface, enum, namespace, module, variable, lambda, constant, block.

```
Scope {
    _uuid: HASHSAFE(absolute_path + signature)    // indépendant du numéro de ligne
    absolute_path: String       // fichier source (absolu)
    name: String
    scope_type: String          // "function", "class", "method", ...
    signature: String           // "async function processData(input: string): Promise<Result>"
    content: String             // code source dédent (contentDedented)
    docstring: String           // commentaire/doc associé
    member_summary: String      // pour containers : liste des signatures enfants
    scope_start_line: Int64     // dans le fichier
    scope_end_line: Int64
    scope_start_char: Int64     // offset char dans le fichier
    scope_end_char: Int64
    complexity: Int64           // cyclomatique (codeparsers)
    lines_of_code: Int64
}
```

**Hash UUID :** `HASHSAFE(absolute_path + signature)` — insensible aux refactors qui déplacent du code (renumerotation de lignes) tant que la signature ne change pas. Si la signature change → nouvelle entité (l'ancienne est supprimée par diff incrémental).

**KB :** ScopeKB — hybrid 3-way (BM25 + vector + sparse).
- `titleFor: signature` — les chunks héritent automatiquement du titre parent
- `contentFor: content` (chunked) + `docstring` + `member_summary`
- Le `content` est chunké, chaque chunk porte `start_line`, `end_line`, `start_char`, `end_char` **relatifs au scope** (pas au fichier)
- Pour calculer la position réelle dans le fichier : `chunk.start_line + scope.scope_start_line`

**member_summary (option B) :** pour les containers (class, interface, enum, namespace, module, struct, trait), un champ séparé indexé dans ScopeKB :

```
Members:
  - constructor(opts: Options) (L16-20)
  - processData(input: string): Result (L22-45)
  - validate(): boolean (L47-60)
```

Avantage vs remplacement du content (option A) : le code source original reste intact et searchable, le résumé est un signal additionnel.

### Library

Représente une dépendance externe (package importé).

```
Library {
    _uuid: HASHSAFE(name)      // unique par nom de package
    name: String                // "express", "tokio", "react"
    import_path: String         // "@types/express", "tokio::sync"
}
```

**KB :** LibraryKB — fulltext only.
- `titleFor: name`
- `contentFor: import_path`

---

## Relations

| Relation | From → To | Hash / dédup | Signification |
|----------|-----------|--------------|---------------|
| `CONTAINS` | Directory → Directory | — | hiérarchie répertoires |
| `HAS_FILE` | Directory → File | — | fichier dans répertoire |
| `DEFINED_IN` | Scope → File | — | scope défini dans ce fichier |
| `PARENT_OF` | Scope → Scope | — | container → enfant (class → method) |
| `CONSUMES` | Scope → Scope | — | appel / utilisation |
| `INHERITS_FROM` | Scope → Scope | — | héritage (class extends) |
| `IMPLEMENTS` | Scope → Scope | — | implémentation (class implements interface) |
| `DECORATES` | Scope → Scope | — | décorateur appliqué |
| `USES_LIBRARY` | Scope → Library | — | import d'un package externe |

Relations inverses (`CONSUMED_BY`, `HAS_PARENT`, `DECORATED_BY`) : à voir si on les stocke ou si on les résout à la query. Le graphe rag3db supporte les traversals bidirectionnels, donc probablement pas besoin de les matérialiser.

---

## Coordonnées : scopes, chunks, highlights

### Architecture core/full du chunker rag3weaver

Le chunker utilise une architecture **core-first** : il découpe d'abord le texte en zones **core** non-chevauchantes, puis étend chaque core avec de l'overlap des deux côtés. Chaque chunk stocke **8 champs de coordonnées** :

```
Chunk {
    // FULL range (avec overlap context)
    start_byte, end_byte           // bytes dans le texte source
    start_line, end_line           // lignes dans le texte source

    // CORE range (zone "possédée", sans overlap)
    core_start_byte, core_end_byte
    core_start_line, core_end_line
}
```

**Propriétés clés :**
- Les zones core sont **contiguës et non-chevauchantes** : `chunk[i].core_end_byte == chunk[i+1].core_start_byte`
- Les zones core couvrent **100% du texte** sans trou
- Les lignes sont calculées via un index cumulatif de newlines (`build_line_index()`) — O(n) pour le build, O(1) par lookup
- **Les numéros de ligne sont réels**, pas relatifs — même avec overlap, `start_line` et `core_start_line` donnent la vraie ligne dans le texte source

### Système de coordonnées à 3 niveaux

```
Fichier (absolu)
│  line 0
│  ...
│  line 15   ← scope.scope_start_line
│    │
│    │  chunk 0:
│    │    start_line: 15      ← full range (inclut overlap gauche, ici = début scope)
│    │    core_start_line: 15 ← zone possédée
│    │    core_end_line: 24   ← fin de la zone possédée
│    │    end_line: 26        ← full range (inclut overlap droite)
│    │
│    │  chunk 1:
│    │    start_line: 23      ← full range (overlap avec chunk 0)
│    │    core_start_line: 25 ← zone possédée (commence juste après core chunk 0)
│    │    core_end_line: 34
│    │    end_line: 36
│    │
│  line 40   ← scope.scope_end_line
│  ...
```

**Règles :**
- `scope.scope_start_line` / `scope.scope_end_line` : position absolue dans le fichier
- `scope.scope_start_char` / `scope.scope_end_char` : offset char absolu dans le fichier
- `chunk.start_line` / `chunk.end_line` : **relatif au content du scope** (0-based), full range avec overlap
- `chunk.core_start_line` / `chunk.core_end_line` : **relatif au content du scope** (0-based), zone possédée sans overlap
- Les highlights BM25 (byte offsets) sont **relatifs au chunk.text**

**Calcul de la position réelle dans le fichier :**
```
file_line = scope.scope_start_line + chunk.core_start_line
file_char = scope.scope_start_char + chunk.core_start_byte
highlight_file_char = scope.scope_start_char + chunk.start_byte + highlight.start
```

**Pour l'affichage d'un résultat de search :** utiliser les coordonnées `core_*` pour identifier la zone pertinente, et les coordonnées `start_*/end_*` pour afficher le contexte étendu (avec overlap). Ça évite les doublons entre chunks adjacents — chaque ligne du fichier n'est "possédée" que par un seul chunk.

---

## Gestion des gros fichiers

### Problème

Les fichiers de code très gros (bundles minifiés, dumps SQL, fichiers générés, gros CSV) peuvent faire planter codeparsers (stack overflow tree-sitter, timeout, mémoire) et produisent des scopes de mauvaise qualité de toute façon. Il faut un mécanisme de triage.

### Configuration

```rust
struct CodeDomainConfig {
    /// Seuil au-delà duquel un fichier n'est pas parsé par codeparsers.
    /// Valeur en bytes. Default: 512_000 (500KB).
    max_parse_size_bytes: usize,

    /// Que faire des fichiers qui dépassent le seuil.
    oversized_strategy: OversizedStrategy,
}

enum OversizedStrategy {
    /// Ignorer complètement : pas de File entity, pas de Scope, invisible.
    /// Utile pour les fichiers qu'on sait inutiles (bundle.min.js, vendor/).
    Ignore,

    /// Créer le File entity et le chunker normalement dans FileContentKB,
    /// mais ne PAS parser avec codeparsers → pas de Scope, pas de relations code.
    /// Le fichier est searchable via FileContentKB (BM25 + vector) mais pas via ScopeKB.
    FallbackChunk,
}
```

### Comportement par stratégie

| Stratégie | File entity | ScopeKB | Relations code | read_file | grep |
|-----------|:-----------:|:-------:|:--------------:|:---------:|:----:|
| **Normal** (≤ seuil) | oui | oui (N scopes parsés par codeparsers) | oui (CONSUMES, PARENT_OF, etc.) | oui | oui |
| **FallbackChunk** (> seuil) | oui | oui (**1 seul scope = fichier entier**, chunké normalement) | **DEFINED_IN uniquement** (pas de parse AST → pas de CONSUMES, etc.) | oui | oui |
| **Ignore** (> seuil) | non | non | non | non | non |

**FallbackChunk en détail :** le fichier oversized est traité comme un unique Scope synthétique :
```
Scope {
    _uuid: HASHSAFE(absolute_path + "__file_scope__")
    absolute_path: absolute_path
    name: file.name
    scope_type: "file"              // type spécial = fichier entier non parsé
    signature: "file: auth.min.js"  // signature synthétique
    content: file.content           // contenu brut intégral
    docstring: ""
    member_summary: ""
    scope_start_line: 0
    scope_end_line: file.lines_of_code
    scope_start_char: 0
    scope_end_char: file.content.len()
}
```

Ce Scope est chunké normalement par le ChunkProcessor dans ScopeKB. Il passe par exactement le même pipeline que les scopes parsés — mêmes chunks, mêmes embeddings, même search. La seule différence : pas de relations code (CONSUMES, PARENT_OF, etc.) puisqu'on n'a pas parsé l'AST.

**Avantage :** zéro cas particulier dans le search. Un `catalog.search("ScopeKB", "database migration")` retourne à la fois des fonctions parsées ET des chunks de gros fichiers non parsés. L'agent ne voit pas la différence.

### Patterns typiques par extension

Certaines extensions méritent un traitement par défaut sans attendre le seuil :

| Pattern | Stratégie suggérée | Raison |
|---------|-------------------|--------|
| `*.min.js`, `*.min.css` | Ignore | Minifié, illisible, pas de valeur search |
| `*.bundle.js`, `*.chunk.js` | Ignore | Bundles webpack/rollup |
| `*.generated.*`, `*.g.dart` | Ignore | Code généré |
| `*.lock`, `package-lock.json` | Ignore | Lockfiles, pas de valeur sémantique |
| `*.sql` (gros) | FallbackChunk | Peut contenir des migrations utiles |
| `*.csv`, `*.json` (gros) | FallbackChunk | Données potentiellement searchables |
| `*.proto`, `*.thrift` (gros) | FallbackChunk | Définitions d'API, searchable |

La config permettra une liste d'overrides par pattern glob :

```rust
struct FileFilterConfig {
    /// Seuil global (default 500KB)
    max_parse_size_bytes: usize,

    /// Overrides par pattern glob
    overrides: Vec<FileFilterOverride>,
}

struct FileFilterOverride {
    /// Pattern glob (ex: "*.min.js", "**/*.generated.*")
    pattern: String,

    /// Stratégie forcée, indépendante du seuil de taille
    strategy: OversizedStrategy,
}
```

---

## Pipeline d'ingestion

### Principes

Le pipeline utilise le système de queue à priorités de rag3weaver. Tous les `catalog.create()` et `catalog.link()` sont **synchrones** et retournent immédiatement des `EntityRef` / `RelationRef`. Le traitement réel (chunking, embedding, stockage, linking) est différé au `catalog.drain()`.

```
create() → enqueue InsertOp (prio 1) + ChunkOp (prio 0) → retourne EntityRef
link()   → enqueue LinkOp (prio 2)                       → retourne RelationRef

drain() exécute dans l'ordre strict :
    Prio 0: ChunkProcessor     (parallel rayon, émet InsertOp/LinkOp/EmbedOp downstream)
    Prio 1: InsertProcessor     (batch 50, résout les EntityRef)
    Prio 2: LinkProcessor       (batch 50, attend résolution des refs from/to)
    Prio 3: EmbedProcessor      (GPU batch 32, UNWIND pour store)
            SparseEmbedProcessor (GPU batch 32)
            DualEmbedProcessor   (mega-batch 500, mini-batch GPU 32)
```

**Events émis** à chaque étape (via `catalog.subscribe()`) : `EntityPrepared`, `ChunksCreated`, `EmbeddingStarted/Completed`, `EntitiesStored`, `RelationsLinked`, `DrainStarted/Completed`. Le code domain peut souscrire pour le suivi de progrès (SSE, logging, UI).

### Flow concret

```
Source (git clone / filesystem)
    ↓
1. Scan arborescence → enqueue Directory + File + relations
    for dir in walk_tree(root):
        let dir_ref = catalog.create("Directory", { absolute_path, name, depth })
        if parent_dir:
            catalog.link("CONTAINS", parent_dir_ref, dir_ref)
        for file in dir.files():
            let file_ref = catalog.create("File", { absolute_path, name, content, ... })
            catalog.link("HAS_FILE", dir_ref, file_ref)
    ↓
2. Filtrer fichiers code + triage par taille
    let (parseable, oversized, ignored) = files.filter(is_code).triage(|f| {
        if f.size_bytes > config.max_file_size_bytes {  // ex: 500KB
            match config.oversized_strategy {
                Ignore => Ignored,       // pas ingéré du tout
                FallbackChunk => Oversized,  // chunké brut, pas de parse AST
            }
        } else { Parseable }
    });

    // Fichiers ignorés : ni File entity, ni Scope — comme s'ils n'existaient pas
    // → filtrer en step 1 avant de créer les File entities

    // Fichiers oversized : créer un Scope synthétique "fichier entier"
    for file in &oversized {
        let scope_ref = catalog.create("Scope", {
            _uuid: HASHSAFE(file.absolute_path + "__file_scope__"),
            absolute_path: file.absolute_path,
            name: file.name,
            scope_type: "file",  // type spécial
            signature: format!("file: {}", file.name),
            content: file.content,  // contenu brut intégral → chunké dans ScopeKB
            scope_start_line: 0,
            scope_end_line: file.lines_of_code,
            scope_start_char: 0,
            scope_end_char: file.content.len(),
        });
        catalog.link("DEFINED_IN", scope_ref, file_refs[file.absolute_path]);
        // Pas de parse codeparsers → pas de CONSUMES, PARENT_OF, etc.
        // Mais searchable via ScopeKB comme n'importe quel scope
    }

    // Fichiers parseable : parse normal
    content_map = parseable.map(|f| (f.absolute_path, f.content));
    parse_result = ProjectParser::parse_project({ root, files: parseable, content_map, resolve_relationships: true })
    ↓
3. Enqueue Scopes
    for scope in parse_result.scopes():
        uuid = HASHSAFE(scope.absolute_path + scope.signature)
        member_summary = if is_container(scope) { build_member_summary(scope, children) } else { "" }
        let scope_ref = catalog.create("Scope", {
            _uuid: uuid,
            absolute_path, name, scope_type, signature,
            content: scope.content_dedented,
            docstring, member_summary,
            scope_start_line, scope_end_line, scope_start_char, scope_end_char,
            complexity, lines_of_code,
        })
    ↓
4. Enqueue Libraries
    for lib in parse_result.external_libraries():
        let lib_ref = catalog.create("Library", { _uuid: HASHSAFE(lib.name), name, import_path })
    ↓
5. Enqueue relations (toutes synchrones, attendent les refs)
    for rel in parse_result.relationships():
        catalog.link(rel.type, scope_refs[rel.from], scope_refs[rel.to])
    for scope in scopes:
        catalog.link("DEFINED_IN", scope_refs[scope.uuid], file_refs[scope.file])
    ↓
6. catalog.drain()
    Le drain exécute tout en ordre de priorité :
    → ChunkProcessor : chunk les content des Scopes + content des Files (parallel rayon)
    → InsertProcessor : insère Directory, File, Scope, Library, chunks en DB
    → LinkProcessor : crée toutes les relations (attend résolution des refs)
    → EmbedProcessor : embed ScopeKB (dense+sparse), FileKB (dense), DirectoryKB (dense), LibraryKB (dense)
    → Events émis tout au long pour tracking progrès
```

### Abstraction pour le code domain

Le pipeline ci-dessus est spécifique au domaine Code. Mais l'interface avec rag3weaver est 100% générique :

```rust
// Le code domain n'a besoin que de ça :
catalog.create(entity_name, data)   → EntityRef
catalog.link(rel_name, from, to)    → RelationRef
catalog.drain()                     → FlushResult
catalog.subscribe()                 → Receiver<CatalogEvent>
```

N'importe quel autre domaine (Google Drive, Shopify, Gmail) utilise exactement la même interface. La logique de parsing/conversion est dans le "domain adapter" (ici : codeparsers), pas dans rag3weaver.

---

## Search patterns

### Recherche de code par intention

```
catalog.search("ScopeKB", "error handling middleware", {
    signals: BM25 | VECTOR | SPARSE,
    limit: 10,
})
```

Retourne des Scopes (fonctions, méthodes, classes) pertinents. Les chunks portent les coordonnées pour naviguer au bon endroit dans le fichier.

### Grep sémantique sur fichiers entiers

```
catalog.search("FileKB", "TODO: fix authentication", {
    signals: BM25,  // fulltext only
    limit: 20,
})
```

Retourne des File entiers. Pas de vector ici — BM25 suffit pour le pattern matching textuel.

### Explore : naviguer le graphe depuis un résultat

```
catalog.search_with_explore("ScopeKB", "database connection pool", {
    limit: 5,
    explore_depth: 2,
    relations: ["CONSUMES", "DEFINED_IN", "USES_LIBRARY"],
})
```

Résultats + graphe de dépendances : "cette fonction appelle X qui utilise la library Y et est définie dans le fichier Z".

### Recherche par arborescence

```cypher
MATCH (d:Directory {absolute_path: '/virtual/owner/repo/src/auth/'})
      -[:HAS_FILE]->(f:File)
      <-[:DEFINED_IN]-(s:Scope)
WHERE SEARCH(s.content, 'jwt token validation')
RETURN s.signature, SEARCH_SCORE() AS score
ORDER BY score DESC LIMIT 10
```

Combine la navigation graph (arborescence) avec le fulltext search.

### Accès fichier brut pour agents (read_file, grep)

**Problème :** les projets GitHub n'existent jamais sur disque — ils sont clonés en `/tmp/`, ingérés, puis supprimés. Mais un agent a besoin de :
1. **read_file** — lire le contenu intégral d'un fichier (pas des chunks)
2. **grep** — chercher un pattern exact (regex) dans le contenu brut de N fichiers

**Solution : `File.content` EST le raw content.** Pas besoin d'un champ `_rawContent` séparé — c'est déjà le cas dans notre schéma. Le contenu brut intégral est stocké comme propriété de l'entité File.

**read_file virtuel :**
```cypher
MATCH (f:File {absolute_path: '/virtual/owner/repo/src/auth.ts'})
RETURN f.content
```
Retourne le texte brut complet. L'agent ne sait pas (et n'a pas besoin de savoir) si le fichier est sur disque ou virtuel. L'absolute_path est la clé d'accès universelle.

**grep virtuel (regex sur contenu brut) :**
```cypher
MATCH (f:File)
WHERE f.absolute_path STARTS WITH '/virtual/owner/repo/'
  AND f.content =~ '.*TODO.*fix.*'
RETURN f.absolute_path, f.name
```
Ou via BM25 pour une recherche plus rapide (Lucivy, pas regex line-by-line) :
```
catalog.search("FileContentKB", "TODO fix", { signals: BM25, limit: 50 })
```

**grep avec numéros de lignes :**
Pour un vrai grep (pattern + ligne + contexte), il faut extraire les lignes matchantes du `content`. Deux options :
- **Côté client** : fetch `f.content`, split par `\n`, filtrer en mémoire. Simple, mais transfère tout le fichier.
- **Côté Lucivy** : BM25 search sur FileContentKB retourne des highlights avec byte offsets → convertir en numéros de lignes via les coordonnées des chunks. Plus efficace pour des gros volumes.

**Alternative : reconstruction depuis les chunks ?**

On pourrait ne PAS stocker `File.content` et reconstruire le fichier en concaténant les zones `core` des chunks FileContentKB :
```
full_content = chunks.sort_by(core_start_byte).map(|c| c.text[core_range]).join("")
```
Pros : pas de duplication (le contenu n'est stocké que dans les chunks).
Cons :
- Les chunks sont potentiellement transformés (overlap découpé, etc.)
- La reconstruction demande N lectures de chunks + assemblage
- Un simple `read_file` devient une opération multi-query
- Les chunks d'un fichier de 5000 lignes = ~50 chunks → 50 lookups

**Verdict : stocker `File.content` directement.** Le coût storage est acceptable (un repo de 500 fichiers × 200 lignes moyen = ~5MB de texte brut, négligeable vs les embeddings qui prennent bien plus). La simplicité d'accès (`RETURN f.content`) justifie la duplication avec les chunks.

Pour les très gros fichiers (> 100KB), on pourrait envisager un seuil : au-delà, ne pas stocker `content` et forcer la reconstruction depuis les chunks. Mais c'est une optimisation future — commencer simple.

---

## Incrémental

**Détection de changement :** `content_hash` (SHA-256) sur chaque File et Scope.

**Re-ingestion :**
1. Re-scan l'arborescence
2. Pour chaque fichier : comparer `content_hash`
   - Inchangé → skip
   - Changé → re-parse avec codeparsers, diff les scopes
   - Supprimé → cascade delete (File + Scopes + chunks + relations)
3. Pour chaque scope : comparer `HASHSAFE(absolute_path + signature)`
   - Même UUID + même content_hash → skip
   - Même UUID + content changé → `catalog.update()` (re-chunk, re-embed)
   - UUID absent → nouveau scope → `catalog.create()`
   - UUID dans DB mais plus dans parse result → supprimé → `catalog.delete()`

**Avantage du hash sur absolute_path + signature (pas sur line numbers) :** si un fichier est reformaté (ajout de lignes blanches, reindentation), les signatures ne changent pas → les UUIDs restent stables → pas de re-ingestion inutile. Seul un changement de contenu ou de signature déclenche un update.

---

## Questions ouvertes

1. **Hash File UUID** : `HASHSAFE(absolute_path + content_hash)` ou `HASHSAFE(absolute_path)` seul ? Si on hash avec content_hash, un fichier modifié = nouvel UUID = perte des relations. Mieux vaut `HASHSAFE(absolute_path)` seul et update le content.

2. **Scopes orphelins** : un scope dont le fichier source est supprimé doit être cascade-deleted. Ça passe par la relation `DEFINED_IN` : `DELETE FROM Scope WHERE _uuid IN (SELECT s._uuid FROM Scope s JOIN ... WHERE file not in new_files)`. Ou plus simple : delete File → DETACH DELETE → cascade.

3. **Profondeur Directory** : pour un repo GitHub avec 500+ répertoires, est-ce qu'on crée une entité Directory par sous-dossier ? Ou seulement les répertoires qui contiennent des fichiers code ? Probablement le 2e — un Directory vide n'a pas de valeur.

4. **member_summary : quand recalculer ?** Si un enfant est ajouté/supprimé/modifié, le member_summary du parent doit être recalculé. En incrémental, ça veut dire : après le diff des scopes, identifier les parents impactés et re-générer leur member_summary.

5. **Scope blocks** : codeparsers extrait des scopes de type `block` (if/for/try). Les inclure dans les entités Scope ou les ignorer ? L5 JS les excluait du member_summary mais les gardait comme entités. Ils ajoutent du bruit en search — probablement mieux de les filtrer complètement.

6. **Relations inverses** : `CONSUMED_BY`, `HAS_PARENT`, `DECORATED_BY` — les matérialiser ou compter sur les traversals bidirectionnels ? Rag3db supporte `MATCH (a)<-[:CONSUMES]-(b)`, donc probablement pas besoin de dupliquer.

7. **FileKB chunking config** : un fichier de 5000 lignes → chunks pour le search hybride. Quelle taille de chunk pour du code brut ? Les scopes sont déjà découpés sémantiquement par codeparsers, donc le chunking FileKB sert surtout pour les fichiers non-code (markdown, config, etc.) et comme fallback grep. Taille plus grande (2000-4000 chars) que ScopeKB (1000) probablement.

8. **Multiple files même signature** : si deux fichiers ont une fonction `main()` avec la même signature, `HASHSAFE(absolute_path + signature)` reste unique grâce à l'absolute_path. OK.

9. **Sparse embeddings sur code** : est-ce que BGE-M3 sparse est pertinent pour du code source ? Les tokens de code (noms de variables, noms de fonctions) sont souvent très discriminants — sparse pourrait être très efficace. À valider empiriquement.

10. **Scope content : dedented ou original ?** codeparsers fournit `content` (avec indentation originale) et `content_dedented` (indentation normalisée). Pour le chunking et l'embedding, dedented est probablement mieux (moins de bruit d'indentation). Mais pour l'affichage, on veut l'original. Stocker les deux ? Ou stocker dedented + indent_level pour reconstruire ?

11. **Cross-entity search (TreeKB) : implémentation UNION.** Aujourd'hui le search ne query que la `title.entity`. Pour TreeKB (Directory + File), il faut un UNION sur `Directory_Chunk` et `File_Chunk` (pour BM25 via Lucivy) et sur les embeddings des deux entités (pour vector). Options :
    - **Option A** : UNION Cypher explicite dans le search query builder quand `kb.entities.len() > 1`
    - **Option B** : lancer N recherches parallèles (une par entité) puis fusionner avec RRF
    - **Option C** : une seule table de chunks partagée `TreeKB_Chunk` au lieu de `Directory_Chunk` / `File_Chunk`
    - L'option B est probablement la plus simple et la plus flexible — c'est le même pattern que cross-KB search du doc 03.

12. **TreeKB : embedding pertinent sur des paths ?** Les paths sont courts et structurés. Est-ce que "le dossier des tests d'authentification" → embedding sur `/virtual/.../tests/auth/` a de la valeur sémantique ? Probablement oui si le modèle a vu des paths pendant l'entraînement (e5, BGE-M3 l'ont). Mais BM25 seul pourrait suffire pour la navigation arborescence. À valider empiriquement.

13. **Seuil gros fichiers : défaut ?** 500KB est un bon défaut pour du code (un fichier de 500KB ≈ 15000 lignes, rare en code normal). Mais pour des fichiers data (JSON, CSV, SQL), 500KB est courant et légitime. Faut-il des seuils différents par extension ? Ou un seul seuil global + overrides par pattern glob ?

14. **Gros fichiers oversized : duplication content.** Le Scope synthétique "file" a `content = file.content` (le contenu brut intégral). Ce même contenu est aussi sur `File.content`. Duplication assumée (cf. section "Accès fichier brut") — le File.content sert au read_file/grep, le Scope.content sert au chunking/embedding ScopeKB. Si un fichier fait 5MB, ça fait 10MB total (File + Scope). À surveiller si beaucoup de gros fichiers dans un repo.

15. **Ignore list : où la configurer ?** Au niveau du domaine (CodeDomainConfig), au niveau du schema YAML, ou au niveau de la pipeline (ScanTreeOp) ? Probablement au niveau du domaine avec possibilité de surcharge par projet (ex: un .rag3ignore à la racine du repo, comme .gitignore).
