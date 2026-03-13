# Doc 02 — Priorités : vers le multi-backend

Date : 13 mars 2026

## Vision

rag3weaver devient un framework RAG multi-backend :

| Backend | Rôle | Priorité |
|---------|------|----------|
| **Supabase + pgvector** | Production cloud, priorité #1 | 🔴 Haute |
| **rag3db** | Graph DB maison, déjà fonctionnel | ✅ Fait (continuer à améliorer) |
| **Neo4j** | Graph DB enterprise | 🟡 Moyenne |
| **Qdrant** | Vector DB spécialisé | 🟡 Moyenne |

## Pourquoi Supabase + pgvector en premier

- Postgres = standard, hébergé partout, écosystème mature
- pgvector = vector search intégré, pas de service séparé
- Supabase = auth, storage, realtime, edge functions — stack complète
- Adoption développeurs >> rag3db pour le moment

## Briques manquantes AVANT le multi-backend

Pas la peine d'abstraire les nœuds dataflow et les opérations catalogue par backend tant que les index custom (FTS + sparse) ne sont pas persistés proprement. Sinon on construit sur du sable.

### Prio 1 : IndexBlobStore — persistence ACID des index custom

**Problème actuel** : lucivy FTS et sparse vector stockent leurs données dans des fichiers à côté de la DB. Pas de backup unifié, pas de transactions ACID avec le reste du graph, fichiers orphelins possibles.

**Solution** : trait `IndexBlobStore` (doc 20) avec persistence en DB.

#### 1a. Lucivy FTS → IndexBlobStore

- Implémenter `sync_to_store()` / `load_from_store()` sur LucivyHandle
- Les fichiers Tantivy (segments, meta) deviennent des blobs dans `_index_blobs`
- À l'ouverture : matérialiser les blobs en fichiers temporaires → mmap → search
- Au commit : sync incrémental (nouveaux segments → save, segments mergés → delete)
- **CLOSE_LUCIVY_INDEX** (fait cette session) est un prérequis ✅

#### 1b. Sparse Vector → IndexBlobStore

- Même pattern : `sparse.mmap` + `sparse_vectors.bin` + `sparse_dims.bin` → blobs dans `_index_blobs`
- Plus simple que lucivy (3 fichiers fixes vs N segments dynamiques)
- Le format mmap flat binary (fait cette session) est déjà le format blob ✅

#### 1c. Table `_index_blobs` dans le Catalog

- `_index_blobs(index_name STRING, file_name STRING, data BLOB, PRIMARY KEY(index_name, file_name))`
- Créée par `initialize()` si absente
- Utilisée par `CypherBlobStore` implémentation du trait

### Prio 2 : Abstraire les opérations DB dans le Catalog

Une fois les index persistés proprement, abstraire les opérations Cypher :

- `trait CatalogBackend` : create_table, insert, query, create_index, etc.
- `Rag3dbBackend` : implémentation actuelle (Cypher sur rag3db)
- `SupabaseBackend` : SQL Postgres + pgvector + pg_trgm (FTS)
- Les nœuds dataflow appellent le backend via le trait, plus de Cypher en dur

### Prio 3 : Adapter les nœuds dataflow

- `InsertRecordNode`, `DeleteRecordNode`, `UpdateRecordNode` → passent par `CatalogBackend`
- `EmbedNode` → inchangé (embeddings sont backend-agnostiques)
- `ChunkRecordNode` → inchangé (chunking est backend-agnostique)
- Search → adapté par backend (pgvector vs sparse_vector extension vs Qdrant API)

## Ce qu'on ne fait PAS maintenant

- Neo4j / Qdrant backends — après Supabase
- Migration de données entre backends
- UI d'administration multi-backend
- Abstraction du schema DDL (chaque backend a ses contraintes)

## Prio 4 : Ingestion de documents réels + DX

Après le multi-backend, la priorité devient l'ingestion de vrais documents (PDF, DOCX, HTML, etc.) et une interface développeur simple.

- **Nœuds d'ingestion document** : parsers/extracteurs intégrés dans le dataflow pipeline
- **Interfaçage facile** : API claire pour brancher un parser externe, exemples prêts à l'emploi
- **Exemples concrets** :
  - **Docling** (IBM) — extraction structurée de PDF/DOCX avec layout analysis
  - **Microsoft MarkItDown** — conversion de documents Office/PDF → Markdown propre
- L'objectif : un développeur branche son parser favori en 5 lignes et ingère des documents réels

## Ordre d'exécution

```
1. IndexBlobStore trait + FileBlobStore (refactoring, zéro changement de comportement)
2. CypherBlobStore (persistence dans _index_blobs via rag3db)
3. Tests E2E persistence avec CypherBlobStore
4. trait CatalogBackend + Rag3dbBackend (extraire le Cypher actuel)
5. SupabaseBackend (SQL + pgvector + pg_trgm)
6. Tests E2E Supabase
7. Nœuds d'ingestion documents réels (Docling, MarkItDown)
8. Exemples et documentation DX
```

Les étapes 1-3 solidifient rag3db. Les étapes 4-6 ouvrent le multi-backend. Les étapes 7-8 rendent le framework utilisable en production avec de vrais documents.
