# Doc 03 — Directions futures : reranking, évaluation, multi-tenancy, streaming

Date : 13 mars 2026

Réf : doc 02 (priorités multi-backend)

## 1. Reranking (cross-encoder)

### Problème

Le hybrid search fait du RRF (Reciprocal Rank Fusion) sur les résultats BM25 + vector + sparse, mais sans reranking. Le RRF mélange des scores hétérogènes (BM25 log-freq vs cosine similarity vs sparse dot product) — le classement final est approximatif.

### Solution

Un nœud `RerankNode` dans le dataflow, après retrieval :

```
Retrieve(BM25 + Vector + Sparse) → RRF top-K → Rerank(cross-encoder) → top-N final
```

Implémentations possibles :
- **API** : Cohere Rerank, Jina Reranker — simple, rapide à intégrer
- **Local** : BGE-reranker-v2-m3, ms-marco-MiniLM — via candle ou ONNX
- **Trait** : `trait Reranker { fn rerank(query, docs) -> Vec<(doc, score)> }` — même pattern que Embedder

### Impact

C'est le plus gros gain de qualité retrieval pour le moins d'effort. Les cross-encoders voient query+doc ensemble (attention croisée) vs les bi-encoders qui encodent séparément.

## 2. Évaluation (RAGAS-style)

### Problème

On ne peut pas optimiser ce qu'on ne mesure pas. Quand on change la stratégie de chunking, les poids hybrid, ou qu'on ajoute le reranking — comment savoir si c'est mieux ?

### Solution

Un module d'évaluation intégré :

```rust
let eval = catalog.evaluate(&[
    EvalCase { query: "...", expected_ids: vec![...], expected_answer: "..." },
    // ...
]).await;

// eval.recall_at_k(5)  → 0.85
// eval.mrr             → 0.72
// eval.faithfulness     → 0.91 (si LLM disponible)
```

Métriques :
- **Recall@K** : les bons documents sont-ils dans le top-K ?
- **MRR** (Mean Reciprocal Rank) : à quel rang apparaît le premier bon résultat ?
- **Faithfulness** (optionnel, nécessite LLM) : la réponse générée est-elle fidèle aux chunks récupérés ?
- **Context precision** : les chunks récupérés sont-ils pertinents ?

### Intégration

- Dataset d'éval = JSON simple `[{query, expected_ids?, expected_answer?}]`
- Peut tourner en CI pour détecter les régressions de qualité
- Compatible avec les datasets BEIR/MTEB existants

## 3. Multi-tenancy

### Deux niveaux de multi-tenancy

#### Niveau 1 : notre cloud (nous = opérateur)

Notre plateforme cloud gère plusieurs organisations :

- **orgId** : identifie l'organisation cliente
- Chaque org a ses propres databases/catalogues
- Le cloud doit savoir : quelle orga possède quelles databases, quotas, billing
- Isolation forte : une org ne peut jamais voir les données d'une autre

```
Cloud Platform
├── Org "acme-corp" (orgId: abc123)
│   ├── Database "prod" → Catalog "knowledge-base"
│   └── Database "staging" → Catalog "test-kb"
├── Org "startup-xyz" (orgId: def456)
│   └── Database "main" → Catalog "docs"
```

#### Niveau 2 : les users gèrent leur propre multi-tenancy

Les développeurs qui utilisent rag3weaver veulent aussi du multi-tenant pour leurs propres utilisateurs :

- **projectId** : existe déjà comme filtre sur search
- Un développeur déploie une seule instance rag3weaver, mais sert N clients
- Chaque client ne voit que ses documents

```
App du développeur (1 instance rag3weaver)
├── projectId "client-A" → documents client A
├── projectId "client-B" → documents client B
└── projectId "client-C" → documents client C
```

### Approches par backend

| Backend | Isolation org (niveau 1) | Isolation projet (niveau 2) |
|---------|------------------------|-----------------------------|
| **rag3db** | DB séparée par org | projectId filter (existant) |
| **Supabase** | Schema séparé ou DB séparée | Row Level Security (RLS) sur projectId |
| **Neo4j** | Database séparée (Enterprise) | Label ou property filter |

### Points à résoudre

- **Trait `CatalogBackend`** doit intégrer orgId + projectId dès le design, pas après
- **Quotas/billing** : compteurs par org (documents, embeddings, storage)
- **API keys** : scoped par org, avec permissions (read-only, read-write, admin)
- **Data residency** : certaines orgs veulent leurs données dans une région spécifique

## 4. Streaming / watch mode

### Problème

L'ingestion est batch (create → drain). En production, les documents arrivent en continu.

### Solution

Un mode watch qui surveille une source et ingère automatiquement :

```rust
catalog.watch(WatchSource::S3 { bucket: "docs", prefix: "uploads/" })
    .with_parser(DoclingParser::new())
    .on_new(|doc| catalog.create("Document", doc))
    .on_delete(|id| catalog.delete("Document", id))
    .start().await;
```

Sources possibles :
- **S3/GCS bucket** : événements S3 ou polling
- **Dossier local** : inotify/fswatch
- **Webhook** : endpoint HTTP qui reçoit des documents
- **Queue** : SQS, RabbitMQ, Redis streams

### Lien avec les parsers documents

Le watch mode se combine naturellement avec les nœuds Docling/MarkItDown (doc 02 prio 4) :

```
Source (S3) → Parser (Docling/MarkItDown) → create() → drain() auto
```

Le `EventBus` existant dans le Catalog peut servir de backbone pour les notifications (document indexé, erreur de parsing, etc.).

## Ordre de priorité (ajouté aux prios du doc 02)

```
1-3.  IndexBlobStore (solidifier rag3db)               ← en cours
4-6.  CatalogBackend + Supabase                        ← multi-backend
7-8.  Ingestion documents réels (Docling, MarkItDown)  ← DX
9.    Reranking (cross-encoder)                         ← qualité retrieval
10.   Évaluation (RAGAS-style)                          ← mesurer la qualité
11.   Multi-tenancy (orgId + projectId dans le trait)   ← production cloud
12.   Streaming / watch mode                            ← ingestion continue
```

Le multi-tenancy (11) doit être pensé dès l'étape 4 (trait CatalogBackend) même s'il n'est implémenté que plus tard. Le reranking (9) peut arriver plus tôt si on veut un quick win qualité.
