# 07 — Inspirations et Directions Futures

## Deux pistes qui convergent

### Piste 1 : Qdrant et les sparse embeddings

Pour les **documents** (markdown, PDF, FAQ...), les solutions standard marchent très bien : chunking + dense embedding + BM25 hybride. C'est exactement ce qu'on a déjà dans rag3weaver avec Tantivy (FTS) + cosine similarity (vecteurs denses).

Mais il manque une pièce : les **sparse embeddings** (SPLADE, etc.). Qdrant les supporte nativement et c'est une référence à étudier pour comprendre :
- Comment ils stockent et indexent les vecteurs sparse (inverted index sur tokens pondérés)
- Comment ils fusionnent sparse + dense dans leur scoring
- Si on pourrait implémenter une extension "sparse embedding" dans rag3db, ou si c'est plus malin de s'en passer
- Quelles autres idées de Qdrant pourraient inspirer notre architecture

**Intérêt concret** : les sparse embeddings capturent mieux les termes exacts (noms de fonctions, identifiants) que le BM25 classique, tout en restant dans l'espace vectoriel. C'est un middle ground entre fulltext et semantic.

### Piste 2 : Code RAG avec graphe de relations

Pour le **code**, les solutions documents-style (chunking + embedding) ne suffisent pas. Le code a une structure que les documents n'ont pas : des fonctions qui appellent d'autres fonctions, des classes qui héritent, des imports qui lient des fichiers. **Le graphe de dépendances EST le contexte.**

C'est ici que le travail exploratoire dans `kuzu-wasm-exp` a posé les fondations.

---

## Le pipeline code : codeparsers -> graph -> search

### @luciformresearch/codeparsers

Le parser de code maison (`ProjectParser` + `RelationshipResolver`) transforme du code source en un graphe structuré :

**ProjectParser** analyse les fichiers et extrait les scopes :
```
File → [ScopeInfo, ScopeInfo, ...]
```
Chaque `ScopeInfo` contient : name, type (class/function/method/interface...), signature, content, docstring, startLine/endLine, parameters, returnType, modifiers, decorators, children, members, complexity...

**RelationshipResolver** croise les fichiers et résout les liens :
```
CONSUMES     : Scope A utilise Scope B (appel de fonction, référence)
INHERITS_FROM : Scope A extends Scope B
IMPLEMENTS   : Scope A implements Scope B
PARENT_OF    : Scope A contient Scope B (classe → méthode)
DEFINED_IN   : Scope → File (où est-ce physiquement)
DECORATES    : Scope A décore Scope B
USES_LIBRARY : Scope → Library (imports externes)
```

### Le schema dans kuzu-wasm-exp

Deux entités principales + une entité Library :

```javascript
entities: {
  File: {
    fields: {
      path:         { type: 'string', titleFor: 'FileKB' },
      absolutePath: { type: 'string' },
    }
  },
  Scope: {
    fields: {
      name:      { type: 'string' },
      scopeType: { type: 'choice' },  // class, function, method...
      signature: { type: 'text', titleFor: 'ScopeKB', boost: 2.5 },
      content:   { type: 'text', contentFor: 'ScopeKB', chunked: true },
      docstring: { type: 'text', contentFor: 'ScopeKB', boost: 0.5 },
      startLine: { type: 'number' },
      endLine:   { type: 'number' },
      parent:    { type: 'string' },
    }
  },
  Library: {
    fields: {
      name:       { type: 'string', titleFor: 'LibraryKB' },
      importPath: { type: 'string' },
    }
  }
}
```

Deux KBs avec des stratégies différentes :
- **FileKB** : fulltext uniquement, jamais chunké, grep-able
- **ScopeKB** : hybrid (BM25 + semantic), chunking activé, keyword_weight 0.3

### Le flow d'ingestion

```
codeparsers parse → entities (File, Scope, Library)
                   → relationships (DEFINED_IN, CONSUMES, PARENT_OF...)
                   ↓
catalog.create('File', {...})     → handle
catalog.create('Scope', {...})    → handle
catalog.relate('DEFINED_IN', scopeHandle, fileHandle)
catalog.relate('CONSUMES', scopeA, scopeB)
                   ↓
catalog.drain()   → batch embed + batch insert + link
```

L'ingestion est queue-based : `create()` et `relate()` sont synchrones (retournent un handle), `drain()` fait le vrai travail (embeddings par batch, insertions DB, liens).

---

## Découvertes clés

### 1. Enrichissement "Members:" pour les classes

Quand une classe est trop grosse pour être affichée en entier, on veut quand même voir sa structure. L'idée : enrichir le `content` des classes avec un résumé de leurs membres **au moment de l'ingestion**.

```
class SearchService

Members:
  - constructor(config: SearchServiceConfig) (L241-245)
  - canDoSemanticSearch(): boolean (L250-252)
  - async search(options): Promise<ResultSet> (L257-351)
  ...
```

Cet enrichissement est fait par `enrichClassContent()` (user code, pas lib) qui :
1. Détecte les container types (class, interface, enum, namespace, module)
2. Trouve les children scopes (via parent + lignes)
3. Construit la section "Members:" avec signature + lignes
4. Append au content avant stockage

Le content enrichi est ce qui est embedé et cherché. Quand on cherche "search service", on matche aussi les noms des méthodes.

### 2. Class boosting

Les classes sont des unités logiques importantes. Quand une classe a un score >= 0.7, on la booste de 10% (multiplicateur 1.1) pour éviter que ses méthodes individuelles la "noient" dans les résultats.

```javascript
boostIf: {
  "scopeType = 'class'": 1.1,
  "scopeType = 'interface'": 1.05,
},
boostMinScore: 0.7,
```

C'est implémenté comme argument de recherche (pas dans le schema) via un mini-parser d'expressions. Le preset `CODE_SEARCH_PRESET` regroupe ces valeurs par défaut pour le cas code.

### 3. searchWithExplore : le graphe au service de la recherche

La killer feature du code RAG : après avoir trouvé les scopes pertinents, **explorer le graphe** pour récupérer le contexte.

```javascript
const result = await catalog.searchWithExplore('ScopeKB', 'create entity', {
  limit: 3,
  exploreDepth: 2,
  exploreTopK: 15,
  consumesRelations: ['CONSUMES', 'USES_LIBRARY'],
  consumedByRelations: ['CONSUMED_BY', 'PARENT_OF'],
});
```

Le résultat contient :
- `results` : les scopes trouvés par la recherche
- `graph` : les nodes et edges du graphe exploré (dépendances, parents, consumers)

Formaté en ASCII tree :
```
extractScopes (function) ★0.76 @ src/GenericCodeParser.ts:203-285
└── [CONSUMES]
    ├── findScopeStarts (function) ★0.70 @ src/GenericCodeParser.ts:317-350
    └── buildScopeTree (function) @ src/GenericCodeParser.ts:380-420
```

C'est exactement ce qu'un LLM ou un développeur veut voir : pas juste "cette fonction est pertinente", mais "cette fonction utilise telles autres fonctions, fait partie de telle classe, et est définie dans tel fichier".

### 4. MatchedRange et container types

Quand le search matche un chunk d'une fonction, le `matchedRange` pointe vers les lignes exactes du chunk. Mais quand c'est un chunk d'une classe (dont le content est le résumé "Members:"), les lignes du chunk sont relatives au résumé, pas au fichier source.

Solution : pour les container types, utiliser les lignes du parent (la classe entière) au lieu des lignes du chunk.

---

## Tensions de design (non résolues)

### Le paradoxe du chunk

**File** veut être grep-able (contenu brut, complet, pas découpé).
**Scope** veut être chunké (pour embedding, pour localisation dans un gros scope).

Décision prise : File = jamais chunké. Scope = chunké pour embedding. Le chunk est une entité cachée (l'utilisateur ne la voit pas dans son schema), gérée en interne par le système.

### Score multi-champs

Un Scope a plusieurs champs qui contribuent à ScopeKB :
- `signature` (identité, poids 2.5) — "comment ça s'appelle"
- `content` (comportement, poids 1.0, chunké) — "ce que ça fait"
- `docstring` (documentation, poids 0.5) — "comment l'utiliser"

Quand le même Scope matche dans plusieurs champs, comment combiner ? La stratégie retenue : `max_with_boost` — le meilleur score domine, avec un léger boost logarithmique pour chaque match supplémentaire.

### Fulltext sur les chunks

Décidé par Lucie : fulltext sur les chunks aussi (pas juste sur le content complet). Raison : pour pouvoir récupérer uniquement les chunks pertinents et ne pas surcharger le contexte d'un LLM avec un scope entier de 5000 lignes.

---

## Vision unifiée : le schema universel

L'architecture n'est PAS code-specific. C'est un **catalogue universel** dont le code RAG est une instanciation :

| Concept universel | Code RAG |
|---|---|
| Entité | File, Scope, Library |
| `titleFor: KB` | signature → ScopeKB, path → FileKB |
| `contentFor: KB` | content, docstring → ScopeKB |
| `boost` par champ | signature 2.5, docstring 0.5 |
| `chunked: true` | content de Scope |
| Relations | CONSUMES, DEFINED_IN, PARENT_OF... |
| Knowledge Base | FileKB (fulltext), ScopeKB (hybrid), LibraryKB |
| `special_ops` | grep + read sur FileKB |

Le même système peut servir pour :
- **Documents** : title + content chunké, hybrid search → comme qdrant
- **Code** : signature + content chunké + docstring, relations codeparsers, explore
- **FAQ** : question (titre, poids 3.0) + answer (content, chunké)
- **Produits** : nom + description + avis, filtres par catégorie

---

## Prochaines étapes

### Court terme
1. **Cloner qdrant** comme référence locale pour étudier les sparse embeddings et l'architecture de scoring
2. **Documenter les APIs Weaver** manquantes pour le code RAG (searchWithExplore, grep, readFile)

### Moyen terme
3. **Implémenter searchWithExplore** dans rag3weaver WASM (Cypher graph traversal après search)
4. **Extension sparse embedding** : évaluer si c'est faisable comme extension rag3db (comme tantivy_fts) ou si ça nécessite un redesign
5. **Hooks d'enrichissement** : `onResultEnrich` callback pour class boosting, Members enrichment, matchedRange

### Long terme
6. **Pipeline complet** : codeparsers → rag3weaver → search + explore → formatted output pour LLM
7. **ONNX embedder WASM** : remplacer MockEmbedder par un vrai embedder local (TEI ou ONNX Runtime)
