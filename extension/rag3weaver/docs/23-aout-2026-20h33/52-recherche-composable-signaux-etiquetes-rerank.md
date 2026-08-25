# 52 — La recherche composable : signaux étiquetés, fusion N-aire, rerank

25 août 2026, soir. Suite directe de la question posée en relisant les dettes :
*« ça devrait pas être composable dans les graphes de recherche, plutôt qu'en
dur en config ? »* Réponse : oui, et l'état des lieux montrait pourquoi la
config était le mauvais endroit.

## 1. Ce qu'on a trouvé en relisant le graphe de recherche

**`Catalog::search` n'est pas un graphe.** ~450 lignes impératives
(`catalog.rs:3155`) : résolution de cible → embedding → vecteur, BM25, sparse en
séquence → chunk→parent → fusion → rerank → pagination. Le mode « composé »
(`search_with_strategy`) construit un graphe dont `KBSearchNode` **rappelle**
`Catalog::search()` — une enveloppe d'expansion, pas une décomposition.

Les nœuds à grain fin existaient (`generic_search_nodes.rs`), la topologie
était composable, mais tout ce qui pèse était figé :

| point | état au matin |
|---|---|
| `FuseResultsNode` | `FusionConfig::default()` **en dur**, aucun paramètre, trois ports fixes `vector`/`bm25`/`sparse` |
| `BM25SearchNode` | `BM25Mode::Contains` forcé ; champs = ceux de la cible, jamais de la config |
| `SearchSourceNode.options` | transportées, **jetées** par tous les nœuds aval |
| rerank | **aucun nœud** — inline dans le monolithe uniquement |
| `VectorSearchNode` | Cypher direct (`CALL QUERY_VECTOR_INDEX`) là où le monolithe passe par `SearchBackend` |
| fan-out multi-cellules | `K = 60.0` littéral, indépendant de `rrf_k` |
| `fusion.rs` | **mort** : `boost_fuse`, `weighted_fuse`, `rrf_fuse` sans appelant hors tests |
| `title_boost` / `content_boost` | désérialisés, recopiés dans `KBMetadata`, jamais lus |

Et la pondération **par champ** n'existait nulle part de bout en bout : lucivy
a un `BoostQuery` dans le moteur mais ne l'expose pas dans son DSL JSON ; notre
requête est un `boolean.should` à égalité stricte ; le score rendu est un seul
`f64` ; `UnifiedResult.score` est un seul champ. Même lu, `title_boost` n'avait
rien à quoi se brancher.

## 2. La KB : à moitié construite

La spec de février avait la bonne réponse au boost : *plusieurs KB avec des
stratégies différentes* — une `TitleKB`, une `BodyKB`, on cherche dans les deux
et on pèse. Les **index** ont été construits (`KBUpdateNode` agrège, chaque KB
a ses `_title`/`_content` séparés dans lucivy). La **combinaison** n'a jamais
existé : une requête = une cible. À la place, on a collé sur la KB une recette
figée (`signals`, `keyword_weight`, `rrf_k`, `title_boost`).

Correction de principe : **la KB garde l'index, perd la recette.** Ses poids
deviennent au mieux les défauts d'un gabarit ; la recette est un graphe.

## 3. Ce qui a été fait

**Le principe : un nœud par (signal × champ), et la fusion pèse par nom.**

- `UnifiedResult.signal: Option<String>` — chaque nœud de recherche étiquette
  ses résultats (son nom, ou `signal=`). C'est l'information « quel signal a
  produit ce hit » qui était perdue dès la sortie du nœud.
- `search::fuse_signals(&[(&[SearchResult], SignalConfig)], strategy, rrf_k)` —
  la fusion **N-aire** ; `fuse_results` à trois listes n'est plus qu'une
  enveloppe pour `Catalog::search`. Règles inchangées (une seule liste →
  brute ; `top_k` ; `Fuse` puis `Boost` ; ré-attachement chunk/données).
- `FuseResultsNode` : ports `vector`, `bm25`, `sparse` **et `signals` en
  fan-in** — tout ce qui y arrive est regroupé par étiquette, ordre de première
  apparition. Config : `strategy`, `rrf_k`, `weights='label:w,…'` (ou objet
  JSON), `boost='a,b'` (rôle `Boost` : module sans entrer dans la fusion),
  `top_k`, `signal`. Défauts : `vector` 0,7 / `bm25` 0,3 / `sparse` 0,2 /
  autres 1,0.
- `BM25SearchNode(fields='a,b', mode=…)` — champ inconnu = **erreur
  explicite** (`'price' is not indexed on 'Product'`), pas un vide silencieux.
- `RerankNode(candidates=20, service='reranker', signal=…)` — le cross-encoder
  comme nœud. Re-score la tête, laisse la queue. Sans service ou sans texte de
  passage : avertissement et passage tel quel, jamais d'échec. Placé après la
  fusion il **remplace** ; branché sur `fuse.signals` avec `boost='rerank'` il
  **module**. La dette « le reranker remplace le score, un mélange serait
  possible » est close par topologie, sans mode.
- `VectorSearchNode` passe par `catalog.search_backend()` quand le service
  `catalog` est là (même chemin que le monolithe, agnostique), Cypher sinon.
- `result_mode` sur les trois nœuds de signal ; `source_resolved` résout vers
  l'entité source (KB) — c'est ce qui rend **deux KB fusionnables** : lignes
  d'index différentes, entités identiques.
- `ResolveParentNode` conserve l'étiquette (elle ne survivait pas au passage
  par `SearchResult`).
- `Catalog::build_dataflow_graph` enregistre le reranker du catalogue sous
  `"reranker"` ; `K` du fan-out → `search::DEFAULT_RRF_K` (public).
- Gabarit `templates/weighted_search.mmd` : deux BM25 sur deux champs, un
  vecteur, fusion à poids nommés, rerank, avec 10 variables.

Le « boost de titre » n'est plus un concept : c'est la branche `title` avec un
poids plus fort que la branche `content`.

```
source ─┬─► title["BM25SearchNode(fields='_title')"]      ─┐
        ├─► content["BM25SearchNode(fields='_content')"]  ─┼─► fuse["FuseResultsNode(weights='title:2,content:1,vector:0.7')"] ─► rerank ─► resolve
        └─► vector["VectorSearchNode"]                     ─┘
```

## 4. Mesuré

`e2e_generic_search` : **12/12**, dont quatre nouveaux.

- `generic_two_field_branches_weights_decide_order` — « Rust pandas » en mode
  split : la branche `description` retrouve le Rust Book, la branche `details`
  le Python Cookbook. `desc:1,det:0` → Rust Book premier ; `desc:0,det:1` →
  Python Cookbook premier. Les poids décident, sans rien dans le moteur.
- `generic_rerank_replaces_head_and_keeps_tail` — pool de 2 : le préféré du
  reranker passe premier, le troisième ne bouge pas même s'il est préféré.
- `generic_rerank_as_boost_signal_inside_fusion` — ordre BM25 `[Rust, Python,
  Knife]`, rerank en boost préférant le couteau → `[Knife, Rust, Python]` :
  le boosté monte, **les deux autres gardent leur ordre**.
- `generic_bm25_unknown_field_is_an_error`.

Les huit anciens (équivalence nœuds ↔ `Catalog::search`, BM25, vecteur,
hybride, sparse, plein hybride, rapport) restent verts — le vecteur via le
backend rend les mêmes uuids que le monolithe. Unitaires : 720/720, dont
16 sur les nœuds génériques et le gabarit pondéré qui se construit et se trie.

## 5. Ce qui reste, et qu'on n'a pas caché

- **`Catalog::search` est toujours le monolithe.** Le vrai renversement —
  qu'il construise et exécute un gabarit avec les défauts de `KBConfig` comme
  variables — est un chantier à part, avec les 206 E2E comme filet. Tant qu'il
  n'est pas fait, il y a deux chemins de recherche à maintenir.
- **Le titre d'une entité simple n'est pas dans l'index BM25** :
  `bm25_fields = content_fields()` (`catalog.rs:1800`). Une branche « titre »
  n'est possible aujourd'hui que sur une KB (`_title`/`_content` séparés).
  À corriger côté schéma : indexer le champ titre.
- **Pas de `column=` sur `VectorSearchNode`** : une seule colonne d'embedding
  par table, nom dérivé (`{kb}_embedding`, index `{table}_vec`). Une branche
  « vecteur du titre » demande une seconde colonne + index à l'ingestion.
- **La fusion inter-KB n'a pas d'E2E** : `source_resolved` est en place et
  testé par le chemin monolithe, pas par un graphe à deux KB.
- `fusion.rs` reste mort et public. À retirer quand le monolithe aura basculé.
- `title_boost` / `content_boost` : toujours « accepté mais non appliqué ».
  Leur remplacement est un gabarit ; ils partiront avec le monolithe.
