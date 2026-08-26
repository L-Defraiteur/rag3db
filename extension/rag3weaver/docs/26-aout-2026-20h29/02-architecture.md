# 02 — L'architecture, telle qu'elle est le 26 août 2026 au soir

Une carte, pas un tutoriel. Chaque section dit **où c'est** et **pourquoi
c'est comme ça**.

## 1. Les trois couches

```
                 ┌──────────────────────────────────────────┐
   fiches .mmd   │  Outils  (search, search_expand, read,    │
                 │           grep, list, edit, trace)        │
                 └────────────────────┬─────────────────────┘
                                      │  GraphTool : fiche + gabarit
   nœuds         ┌────────────────────▼─────────────────────┐
                 │  Dataflow  (38 types de nœuds)            │
                 │  DataflowRuntime — parallèle par niveau   │
                 └────────────────────┬─────────────────────┘
                                      │  services
   moteur        ┌────────────────────▼─────────────────────┐
                 │  Catalog — entités, relations, recherche  │
                 │  rag3db (fork kuzu) + lucivy + burn       │
                 └──────────────────────────────────────────┘
```

**Rien ne saute une couche.** Un outil est un graphe ; un graphe est fait
de nœuds ; un nœud parle au catalogue par un **service** nommé.

## 2. Le dataflow

- `src/dataflow/graph.rs` — `DataflowGraph`, nœuds et arêtes typées.
- `src/dataflow/runtime.rs` — l'exécuteur. **Trois phases par itération** :
  préparer (séquentiel, sort les valeurs du magasin de ports avec le
  décompte des consommateurs), **exécuter le niveau** (`run_level` : un fil
  par nœud prêt, `std::thread::scope`), ranger (séquentiel, dans l'ordre
  topologique). Publie `RunStarted`/`NodeRun`/`RunFinished`.
- `src/dataflow/port.rs` — `PortValue` (`Data(Arc<dyn Any>)` | `Trigger`),
  **à nous** depuis le 26 août ; `take_or_clone` pour un éventail légitime.
- `src/dataflow/node_registry.rs` — `NodeSchema`, `ConfigParam` (avec
  `choices` et `json_schema`), `NodeFactory`. **`Choices` vit ici**, donc
  sur le nœud qui consomme le paramètre.
- `src/dataflow/graph_tool.rs` — `GraphTool` : en-tête `%% tool:`,
  `%% param:`, `%% choices:`, `%% on:`, `%% policy:`, `%% result:`.
  `bind(&NodeRegistry)` fait **hériter** chaque `$var` du paramètre de nœud
  qu'il alimente, à travers l'imbrication.
- `src/dataflow/services.rs` — `ServiceRegistry`, avec `layered(parent)`
  pour ajouter une clé à un appel sans copier.

**Services usuels** : `catalog`, `conn`, `dialect`, `embedder`,
`fts_handles`, `sparse_handles`, `reranker`, `ocr`, `llm`, `file_source`,
`file_access`, `event_bus` (publier), `events` (lire), `parent_run`,
`run_topic`, `scope`, `node_id_cache`.

> **Deux clés pour deux rôles.** `event_bus` publie, `events` lit. C'est ce
> qui empêche un graphe de trace de se tracer lui-même.

## 3. Les événements

- `src/events.rs` — **un bus, plusieurs sujets**, créés à la demande.
  Sujets fournis : `catalog`, `search`, `agent`, `dataflow`, `messages`,
  plus `run.<id>` et `run.<id>.inbox`.
  - **La cellule est l'espace de noms** : le sujet réel est
    `org/project/<sujet>`, et `in_scope(&Scope)` est la seule porte. Une
    fuite inter-organisations est **inexprimable**.
  - `cursor(topic, name)` : récepteur **nommé et gardé**, pour qu'un nœud
    construit plus tard retrouve ce qui s'est passé avant lui.
  - `across_scopes(by)` : vue inter-cellules, explicite et **auditée**.
- `src/dataflow/reactor.rs` — la boucle qui rend un graphe événementiel.
  Un fil, un runtime tokio dedans, une tâche par sonnette, un `select`
  entre « un événement arrive », « un lot est dû » et l'arrêt. Politiques
  `each` / `batch <ms>` / `debounce <ms>`.
- `src/dataflow/trace_nodes.rs` — `EventSourceNode` (draine des sujets,
  `inbox` et `self` relatifs au run) et `TraceSinkNode` (écrit `Trace`, et
  `Run` / `Message` **liés** par `CHILD_OF`, `SENT_BY`, `SENT_TO`).

**Un run est une adresse.** `RunStarted { run, parent, kind, name, scope }`
→ un arbre agent → outil → graphe → nœuds, cherchable dans `Trace`.

## 4. L'agent

`src/agent.rs` — `Agent::run` est un réacteur de session : il boucle, il
compte, il arrête. Ce qui est déclaré : `AgentLimits` (`max_iterations`,
`token_budget`, `stop_on_repeated_error`, `final_nudge`), `with_events`,
`with_run_id`, `with_inbox`.

Deux mécanismes de rattrapage, tous deux nés d'échecs mesurés :
- **`recover_tool_calls`** (`src/llm.rs`) — un appel resté dans le texte
  (Qwen3-Coder, Hermes, Mistral) est exécuté au lieu de conclure le tour.
  Borné (8 appels, 64 Ko), jamais silencieux (`Warning` sur le bus,
  `Usage.recovered_calls`).
- **La boîte lue entre deux tours** — jamais au milieu d'un appel.

## 5. Le catalogue

`src/catalog.rs` (~5 400 lignes) — le cœur.

- **Entités** : `register_entity(name, EntityConfig)`. Champs `is_title` /
  `is_content`, `hashsafe` (identité stable), `return_fields` (ce qu'une
  recherche rend en plus), `signals` (BM25 / vecteur / sparse), **`chunked`**
  (`Some(false)` = pas de chunks, refusé avec un signal vecteur ou sparse).
- **File d'attente** : `create` / `link` / `update` / `delete` **ne
  touchent pas la base** — ils remplissent `pending`. `drain()` compile
  cette file en un graphe et l'exécute. *Simuler puis recompiler* est déjà
  la forme ; le compilateur, lui, est encore naïf.
- **Recherche** : `resolve_search_target(name)` → `SearchTarget`
  (`parent_table`, `chunk_table`, `chunk_rel`, `bm25_fields`,
  `enrich_fields`, `default_signals`). Puis fusion RRF ou pondérée.
- **Multi-tenant** : `Scope { org, project }` estampille chaque ligne.

**Où vivent les index** — et c'est contre-intuitif :

| Index | Table |
|---|---|
| Plein texte (lucivy) | **parente** |
| Vecteur (HNSW) | **chunks** |
| Sparse | **chunks** |

C'est pourquoi une entité sans chunk reste cherchable en BM25 et devient
invisible en vectoriel.

## 6. La recherche

`src/search.rs` — `SearchOptions` (`consistency`, `signals`, `bm25_mode`,
`limit`, `result_mode`), `UnifiedResult`, fusion N-aire `fuse_signals`.

- `src/dataflow/generic_search_nodes.rs` — `VectorSearchNode`,
  `BM25SearchNode`, `SparseSearchNode`, `FuseResultsNode` (port `signals`
  en fan-in, poids et boost par étiquette), `RerankNode`,
  `ResolveParentNode`.
- `src/dataflow/render_nodes.rs` — `RenderResultsNode` : markdown compact
  (liens `fichier:début-fin`, hiérarchie `Classe::méthode`, regroupement),
  **passe-plat** sur `results` pour que la composition continue.

## 7. Le code

- `codeparsers/` — tree-sitter, 12 langages. `analyze(root, sources)` rend
  fichiers, scopes, bibliothèques, relations, **et `pending`** (ce que le
  lot attend et n'a pas trouvé).
- `src/code.rs` — schéma `File` / `Scope` / `Library` / **`Symbol`**, onze
  relations. `ingest_code` puis `resolve_across_batches` : deux passes en
  deux requêtes `UNWIND`, **définisseur unique sinon abstention**.
- `src/code_tools.rs` — `FileSource` (`cursor`, `list`, `read`, `write`),
  `WorkingTree`, `Snapshot`, `RootPolicy` (la frontière d'accès), et
  `read` / `grep` / `list` / `edit` avec péremption par hash et
  « vouliez-vous dire ».

## 8. Les modèles

- `src/burn_device.rs` + `burn_*` — BGE-M3, MiniLM, rerankers, PP-OCRv6,
  Qwen2.5-0.5B, sur **wgpu/Vulkan**. Vérifié le 26 août : `libvulkan_radeon`,
  vraie carte, ~1,7 cœur de CPU.
- `src/openai_llm.rs` — un seul client pour OpenAI, Vertex, AI Studio,
  Mistral, vLLM, Ollama, **et `llama-server`** (aucun adaptateur : une
  racine d'URL sans authentification suffit).

## 9. Les invariants qu'on s'est donnés

1. **Fire and forget** : publier ne bloque jamais.
2. **La cellule est un espace de noms, pas un filtre.**
3. **Un run ne s'écoute pas lui-même** (payé par la trace qui se traçait).
4. **Une relation manquante vaut mieux qu'une relation fausse** —
   définisseur unique, sinon abstention.
5. **Jamais silencieux** : tout renoncement se compte et part sur le bus.
6. **L'index est un service rendu, pas une porte.**
7. **Le rendu est une politique du graphe, pas une question posée au
   modèle** — chaque paramètre exposé est une décision qu'il peut rater.
