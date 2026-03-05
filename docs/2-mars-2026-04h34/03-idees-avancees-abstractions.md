# 03 — Idées avancées & abstractions cross-domain

Extensions au-delà des use cases concrets du doc 02. Fonctionnalités transversales qui exploitent le graph comme avantage structurel sur les vector stores classiques.

---

## 1. Ingestion réactive par webhooks

**Problème :** re-ingérer en batch (re-clone, re-parse, re-drain) est coûteux et lent. Les sources changent en continu.

**Solution :** listener de webhooks qui déclenche des `catalog.upsert()` ciblés.

| Source | Webhook | Action |
|--------|---------|--------|
| GitHub | `push` event | Diff des fichiers modifiés → re-parse uniquement ceux-là |
| GitHub | `issues` / `pull_request` | Upsert entité Issue/PR |
| Shopify | `products/update` | Upsert Product + Variants |
| Google Drive | Changes API (polling ou push) | Upsert File, re-extract contenu |
| Gmail | `history.list()` (polling) | Ingest nouveaux mails depuis lastHistoryId |

**Mécanisme :** `catalog.upsert()` compare le content hash. Si inchangé → skip. Si changé → update entité, re-chunk, re-embed uniquement les chunks modifiés. Les relations existantes sont préservées sauf si la structure change.

**Ce qui existe déjà :** le hash detection dans `catalog.update()` + le lazy commit Lucivy. Manque : l'exposition comme service réactif (HTTP webhook handler ou queue listener).

---

## 2. Cross-KB search (recherche multi-domaine)

**Problème :** un utilisateur cherche "authentication" et veut des résultats de code, de documentation, de mails, et d'issues — chacun avec son propre scoring et ranking.

**Solution :** `catalog.search_across_kbs()`

```rust
let results = catalog.search_across_kbs(
    &["ScopeKB", "DocumentKB", "MailKB", "IssueKB"],
    "authentication middleware",
    SearchOptions {
        signals: BM25 | VECTOR,
        limit: 20,
        per_kb_limit: 10,  // max 10 par KB avant fusion
        fusion: RRF { k: 60 },
    },
)?;
// results: Vec<SearchResult> avec result.kb_name pour identifier la source
```

**Implémentation :** lance N recherches en parallèle (une par KB), puis fusionne avec RRF inter-KB. Chaque KB a ses propres signaux et scoring, la fusion normalise.

**Avantage graphe :** après la fusion, un pass d'explore BFS peut relier les résultats entre eux. "Ce scope dans ScopeKB est `DEFINED_IN` ce fichier qui est `MENTIONED_IN` ce mail" → contexte croisé impossible avec un vector store.

---

## 3. Lineage / provenance tracking

**Problème :** "d'où vient cette information ? quand a-t-elle été ingérée ? est-elle à jour ?"

**Solution :** relation `SOURCED_FROM` universelle avec métadonnées de provenance.

```
Entité quelconque
  └─ SOURCED_FROM → Source {
       source_type: "github" | "drive" | "shopify" | "manual" | ...
       source_url: "https://github.com/user/repo/blob/main/src/auth.ts"
       ingested_at: timestamp
       version: "abc123" (commit sha, drive revision, etc.)
       content_hash: "sha256:..."
     }
```

**Use cases :**
- "depuis quand on parle de ce pattern dans la codebase ?" → filtre sur ingested_at
- "ce document a changé combien de fois ?" → count(SOURCED_FROM) par entité
- "quelles entités viennent de ce repo GitHub ?" → explore Source→SOURCED_FROM→*
- Garbage collection : supprimer les entités dont la source n'existe plus

---

## 4. Schema inference automatique

**Problème :** déclarer les filter fields manuellement est fastidieux, surtout pour de nouvelles sources.

**Solution :** `catalog.auto_configure()` analyse les N premières entités et infère les types optimaux.

**Règles d'inférence :**

| Pattern détecté | Type inféré | Filter Lucivy |
|-----------------|-------------|----------------|
| < 20 valeurs uniques string | String | exact match |
| Numérique, grande variance | Double/Int64 | range |
| Numérique, 2-3 valeurs | Int64 | exact |
| Booléen (true/false, 0/1) | Bool | toggle |
| Contient "\|" ou "," (listes) | Tags (String[]) | has-any |
| > 200 chars en moyenne | Text | FTS (contentFor KB) |
| < 50 chars en moyenne | String | titleFor KB |
| Format date ISO | Timestamp | range |

**Output :** suggestion de schema (KBs, filter fields, chunking config) que l'utilisateur valide ou ajuste. Pas d'auto-application sans confirmation.

---

## 5. Embedding adaptatif par KB

**Problème :** un seul modèle d'embedding pour tout est sous-optimal. Du code, de la documentation technique, et des descriptions produit ont des distributions sémantiques très différentes.

**Solution :** permettre un embedder différent par KB dans la config.

| KB | Embedder | Dim | Justification |
|----|----------|-----|---------------|
| ScopeKB (code) | CodeBERT / StarCoder | 768 | Entraîné sur du code |
| DocumentKB (docs) | multilingual-e5-large | 1024 | Multilingue, long contexte |
| ProductKB (produits) | e5-small | 384 | Rapide, suffisant pour descriptions courtes |
| MailKB (mails) | multilingual-e5-base | 768 | Multilingue, style conversationnel |

**Ce qui existe déjà :** le trait `DualEmbedder` supporte dense + sparse. Il suffit de permettre une instance d'embedder différente par KB dans `KbConfig`, au lieu d'un seul embedder global sur le `Catalog`.

**Impact :** chaque KB a son propre index HNSW avec les bonnes dimensions. Le search par KB appelle le bon embedder pour encoder la query.

---

## 6. Graph-aware reranking

**Problème :** la fusion RRF traite chaque résultat indépendamment. Mais des résultats connectés dans le graphe forment souvent un cluster thématique plus pertinent.

**Solution :** pass de reranking post-fusion qui exploite la topologie du graphe.

**Algorithme :**
1. Après fusion RRF → top N résultats
2. Pour chaque paire (i, j) dans le top N, vérifier s'il existe un chemin court (1-2 hops) dans le graphe
3. Si oui → boost mutuel : `score_i += cluster_bonus * (1 / distance_ij)`
4. Re-trier par score ajusté

**Exemples de clusters :**
- 3 méthodes dans la même classe (PARENT_OF chain) → cluster "implémentation d'une feature"
- Un scope + le fichier qui le définit + la library qu'il importe → cluster "module complet"
- Un mail + son thread + les contacts CC → cluster "conversation"

**Implémentation :** une requête Cypher de type `MATCH (a)-[*1..2]-(b) WHERE a._uuid IN [...] AND b._uuid IN [...]` pour trouver les paires connectées. Coût faible car on ne cherche que parmi les top N résultats (typiquement 10-20).

---

## 7. Notion / Confluence connector

**Spécificité :** structure hiérarchique native (workspace → pages → sous-pages → blocs) qui mappe directement sur le graphe.

```
Workspace
  └─ HAS_PAGE → Page { title, content, lastEditedAt, createdBy }
       ├─ HAS_CHILD → Page (sous-page)
       ├─ MENTIONS → Page (liens internes [[page]])
       ├─ HAS_DATABASE → Database { title, schema }
       │    └─ HAS_ROW → DatabaseRow { properties... }
       └─ CREATED_BY → User { name, email }
```

**Avantage graphe :** les liens internes Notion (`[[page]]`) deviennent des relations `MENTIONS` navigables par BFS. "Quelles pages référencent cette page ?" → explore inverse.

**KBs :** PageKB (hybrid sur title+content), DatabaseKB (fulltext sur propriétés structurées).

---

## 8. Abstractions cross-domain

Patterns récurrents qui traversent tous les domaines et pourraient devenir des primitives de premier ordre dans rag3weaver.

### 8a. Source — abstraction universelle d'origine

Toute entité vient de quelque part. Factoriser :

```rust
struct SourceInfo {
    source_type: SourceType,      // GitHub, Drive, Shopify, Manual, API, Upload
    source_url: Option<String>,   // URL canonique de l'original
    source_id: Option<String>,    // ID dans le système source (commit sha, drive file id, shopify product gid)
    ingested_at: chrono::DateTime<Utc>,
    content_hash: String,         // SHA-256 du contenu brut
}

enum SourceType { GitHub, GoogleDrive, Shopify, Gmail, Notion, Confluence, Upload, Manual, Webhook }
```

Toute entité créée via `catalog.create()` reçoit automatiquement un `SourceInfo` attaché. Permet le tracking de provenance, la déduplication cross-source, et le garbage collection.

### 8b. Hierarchie — pattern parent-enfant récurrent

Presque tous les domaines ont une hiérarchie :

| Domaine | Parent | Enfant | Relation |
|---------|--------|--------|----------|
| Code | File | Scope | DEFINED_IN |
| Code | Class | Method | PARENT_OF |
| Fichiers | Directory | File | HAS_FILE |
| Docs | Document | Section | IN_DOCUMENT |
| Notion | Page | SubPage | HAS_CHILD |
| Shopify | Collection | Product | IN_COLLECTION |
| Mail | Thread | Mail | IN_THREAD |

**Abstraction :** `HierarchyTrait` sur les entités — `parent()`, `children()`, `ancestors()`, `descendants()`. Permet un explore BFS générique sans connaître le nom de la relation.

### 8c. Mention — références croisées

Pattern universel de "A référence B" :

| Domaine | Source | Cible | Type de mention |
|---------|--------|-------|-----------------|
| Code | Scope | Scope | CONSUMES (appel de fonction) |
| Code | Scope | Library | USES_LIBRARY (import) |
| Docs | Section | Section | REFERENCES (lien interne) |
| Mail | Mail | URL | MENTIONS_URL |
| Mail | Mail | Contact | FROM / TO / CC |
| Notion | Page | Page | MENTIONS ([[lien]]) |

**Abstraction :** détection automatique de mentions dans le contenu texte (URLs, emails, identifiants, noms de fichiers, noms de fonctions). Un `MentionDetector` configurable par domaine qui crée les relations automatiquement pendant le drain.

### 8d. Temporalité — versioning et timeline

Beaucoup d'entités évoluent dans le temps :

| Domaine | Entité | Événements temporels |
|---------|--------|---------------------|
| Code | Scope | Créé dans commit X, modifié dans commit Y, supprimé dans commit Z |
| Docs | Document | Version 1 (jan), version 2 (fév), version 3 (mar) |
| Shopify | Product | Créé, prix changé, stock épuisé, remis en stock |
| Mail | Thread | Message 1 (lun), réponse (mar), forward (mer) |

**Abstraction :** `VersionedEntity` avec relation `PREVIOUS_VERSION` et `SUPERSEDED_BY`. Permet de répondre à "comment cette entité a évolué ?" et de chercher dans l'historique.

### 8e. Enrichissement — métadonnées calculées

Pattern récurrent d'ajout de métadonnées post-ingestion :

| Domaine | Enrichissement | Source |
|---------|---------------|--------|
| Code | Complexité cyclomatique | codeparsers (déjà fait) |
| Code | Coverage de tests | CI/CD integration |
| Docs | Résumé automatique | LLM |
| Docs | Langue détectée | fasttext / whatlang |
| Shopify | Score de popularité | Ventes / vues |
| Images | Description Vision AI | Claude / Gemini |
| Tout | Sentiment analysis | LLM / modèle dédié |

**Abstraction :** `EnrichmentPipeline` — une liste de fonctions `(entity) → enriched_fields` exécutées pendant ou après le drain. Configurable par type d'entité. Certains enrichissements sont gratuits (complexité, langue), d'autres coûtent (Vision AI, LLM) et sont opt-in.

### 8f. Access control — multi-tenant et permissions

Dès qu'on connecte des sources réelles (mails, Drive, Shopify), la question des permissions se pose.

**Abstraction :** `TenantScope` — chaque entité appartient à un tenant (user_id, org_id). Les recherches sont automatiquement filtrées par tenant via `allowed_ids` pre-filter.

```rust
// Transparent pour l'appelant
let results = catalog.search_as("ScopeKB", "auth middleware", tenant_id)?;
// Internement: ajoute un filtre Cypher MATCH (e) WHERE e._tenant_id = $tenant_id
// → résolu en allowed_ids avant la recherche
```

Déjà partiellement supporté via `FilterCompiler::split()` (filtre Cypher → allowed_ids). Il suffit d'ajouter un champ `_tenant_id` systématique et un helper `search_as()`.

---

## 9. Tableau récapitulatif abstractions × domaines

| Abstraction | Code | Docs | Shopify | Mail | Drive | Notion |
|-------------|:----:|:----:|:-------:|:----:|:-----:|:------:|
| Source (provenance) | commit sha | upload id | product gid | message id | file id | page id |
| Hierarchy (parent/child) | class→method | doc→section | collection→product | thread→mail | dir→file | page→subpage |
| Mention (cross-ref) | CONSUMES, USES_LIB | liens internes | — | FROM/TO, URLs | — | [[liens]] |
| Temporalité (versions) | commit history | doc versions | price changes | thread timeline | revision history | page history |
| Enrichissement | complexité, coverage | résumé, langue | popularité | sentiment | OCR, description | — |
| Access control | repo privé/public | permissions | shop owner | user mailbox | sharing settings | workspace members |
