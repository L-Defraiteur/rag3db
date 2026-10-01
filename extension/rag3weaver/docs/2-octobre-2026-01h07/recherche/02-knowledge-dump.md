# Recherche — knowledge dump (2 octobre 2026)

Tout ce que la session « recherche » sait de sa partie, pour reprendre sans
relire trois semaines de transcript. Vérifié = prouvé par un test ou une
lecture datée ; le reste est marqué.

## 1. Le chemin de recherche unique

Depuis le 18 septembre 2026, **toute** recherche passe par
`Catalog::rechercher(&Arc<Mutex<Catalog>>, cible, requete, options)`
(catalog.rs, ~6690) : il monte les services (`register_search_services` —
conn, dialect, scope, fts_handles, plein_texte_natif, sparse_handles,
embedder(s), embedding_models/slug, reranker), instancie
`templates/tools/search_base.mmd` par `GraphTool` avec cinq paramètres de
fiche, **injecte `options` entières dans la config du nœud `source`**
(catalog.rs ~6780 — la fiche ne porte que cinq scalaires, la requête en
porte plus), **retire `render`** (un port consommé n'est plus lisible ; on
relit `resolve.results` et les métas en feuilles), exécute, fond les métas
(`fondre_les_metas`), fan-out de cellules par `fusionner_par_cellule`
(`options.scopes`), bascule de cellule par `options.scope`, requalifie
`EmbeddingModelUnavailable` à la frontière, émet `CatalogEvent::SearchCompleted`.
Le verrou du catalogue n'est **pas réentrant** : jamais de guard vivant à
travers un appel à `rechercher`.

Le graphe `search_base` : `source → bm25/vector/sparse → fuse → rerank →
paginate → resolve → render`, la source alimentant tous les nœuds en
`query` et les métas convergeant sur `render` (fan-in `merge_port_values` :
avertissements concaténés, `partial |=`).

**Les trois entrées du chemin** :
- `rechercher` (les ~220 appels de tests, `search_with_explore` est mort) ;
- **l'outil des agents** : `templates/tools/search.mmd`, qui contient le
  type de nœud `SearchTool` = `search_base.mmd` lié puis promu nœud
  (`graph_tool.rs`, `SEARCH_TOOL_NODE_TYPE`) ; onze verbes
  (`BUILTIN_TOOL_NAMES`) ; `search_base` n'est **jamais** offert comme
  outil ;
- **le graphe stratégie** : `Catalog::build_dataflow_graph` instancie
  `templates/search_expansion.mmd` (`query_source[KBQuerySourceNode] →
  primary_search[SearchTool] → fetch_related_i → compose`), le payload
  entre par le port `source.query` du composite (pas B du 18 septembre) et
  porte les `SearchOptions` entières ; la config `target`/`query` du
  composite n'est que le défaut qu'exige sa fiche. Garde `max_rounds` dans
  `build_dataflow_graph` (qui rend un `Result` depuis le 2 octobre).

**Le composite** (`GraphNode`) : runtime imbriqué à chaque exécution,
services empilés (`layered` + `parent_run`), ports libres exposés sous
`nœud.port` (`source.query` en entrée ; `render.text/results/query/meta` en
sortie — `render` réémet ce qu'il reçoit, c'est ce qui rend le sous-graphe
composable). Le canal `subscribe()`/report ne voit que le graphe parent ;
le `event_bus` voit tout, en deux étages (prouvé par `e2e_agent_loop`, qui
nomme les 9 nœuds internes — **passera à 10 avec `FieldWeightNode`**).

## 2. La fusion, et d'où viennent ses poids

`FuseResultsNode` (generic_search_nodes.rs) : fusion N-aire d'étiquettes —
trois ports nommés (`bm25`, `vector`, `sparse`) plus le port `signals` en
fan-in regroupé par `UnifiedResult::signal`. `fuse_signals` (search.rs)
fait le calcul (RRF `weight/(k+rang)` ou somme pondérée normalisée).
`DuplicatePolicy::Merge` fond les occurrences (étiquette `a+b`), `Keep` les
garde avec le score fusionné. `boost` : une étiquette en rôle `Boost`
module au lieu de fusionner (c'est ainsi qu'un reranker se **mélange**).

**Défauts du moteur** (`FusionConfig::default()`) : bm25 0,3, vector 0,7,
sparse 0,2. **Défaut du gabarit** `search_base` : bm25 0,6, vector 0,4
(penché plein texte à dessein — une correspondance exacte d'identifiant ne
doit pas couler sous le vecteur), sparse non nommé → 0,2 moteur.

**Préséance au 2 octobre au soir** (master) : appelant (`options.fusion`) >
entité (`EntityConfig.fusion`, transporté par `SearchTarget.default_fusion`
en `Option` — **jamais aplati**) > `weights` du nœud (seulement si personne
ne déclare) > moteur. **L'échelle retenue par Lucie (en cours de codage,
branche `pas-c-ponderations`)** : appelant > `weights` (choix du graphe,
bat l'entité) > entité > `default_weights` (défaut du gabarit) > moteur —
et `search_base` migre ses 0,6/0,4 en `default_weights`.

**La leçon du 18 septembre (fusion aplatie)** : `unwrap_or_default()` sur
`default_fusion` faisait passer le défaut moteur pour une déclaration et
éteignait les poids du gabarit — l'outil des agents fusionnait 0,3/0,7 au
lieu de 0,6/0,4 et `merge_port_values` sortait du top 5. C'est le seuil
« markdown ×3 < JSON » d'`e2e_code` qui l'a attrapé. Chaque étage reste une
`Option` qui dit « déclaré » ou « muet » ; on n'aplatit jamais.

## 3. Résolution chunk → parent

Les nœuds de signaux résolvent **eux-mêmes** au parent
(`resolve_vector_chunks_with_dialect`, dialecte pris au service — pas
rag3db en dur), avec enrichissement (`enrich_fields`) : la fusion, le
rerank et la pagination travaillent sur des **parents** (la pagination
coupe des parents, pas des chunks). Dédoublonnage au premier rang ; le
meilleur chunk voyage dans `chunk` (`ChunkInfo` : texte + offsets
start/end line/char), `Detailed` porte les chunks attribués (`chunks`),
`SourceResolved` remonte à l'entité source d'une dérivée
(`_source_entity`/`_source_uuid` dans `data`, helper `source_info` —
depuis le 18, tout ça vit dans `dataflow/resultat.rs`, pas dans
`search_strategy.rs`). `result_mode` est `Option` sur les nœuds : absent =
hériter des options (motif B10). Le champ d'origine d'un chunk est
`_parent_field` (`_source_field` n'existe plus depuis le repli des KB).

## 4. Les signaux

- **Plein texte** : handles lucivy par table (service `fts_handles`,
  ouverts paresseusement par la source — `ensure_fts_handle`) ou le chemin
  natif (`plein_texte_natif`, `search_texte_natif`). Requête fuzzy
  `NgramContainsQuery` (séparateurs ignorés : « merge_port_values » se
  cherche « mergeportvalues », l'avertissement le dit), modes
  Contains/Regex (`bm25_mode`), `fuzzy_distance`. Le pré-filtre descend en
  offsets lucivy (`resolve_filter_to_ids` → `allowed_ids`) et la `doc_freq`
  se compte sur le sous-ensemble. **Instantané périmé** : si `Immediate`
  draine pendant le graphe, le handle du service peut manquer → repli sur
  le service `catalog` verrouillé brièvement.
- **Vecteur** : la requête est embarquée **une fois** par la source
  (`embarquer_la_requete`) et voyage dans le payload. Stockage par modèle
  (`vector_storage(chunk_table)` : index et colonne par slug —
  embarquements multi-modèles), même repli « catalogue vivant » que le
  plein texte. `compile_filter_for_vector` **systématiquement** (la cellule
  fuyait entre tenants par le chemin composable sinon) ; sans condition ni
  multi-cellule il rend `None`. Pool : `(limit + offset) × 2`, relevé à
  `rerank.candidates`.
- **Sparse** : service `sparse_handles`, poids moteur 0,2, jamais mesuré au
  banc (la référence du 2 octobre ne le couvre pas encore). Un signal muet
  explique son silence (`expliquer_le_silence_d_un_signal` : « pas
  embarqué » vs « rien à trouver »).

## 5. Rerank, pagination, consigne

`RerankNode` (cross-encoder, `keep_signal=true` dans le gabarit) rescide le
**pool** avant la pagination — sinon on rescorerait une page. `PaginateNode`
applique `offset` puis `limit` après le rerank, avant la résolution
finale ; `meta.fused_count` dit le total d'avant page. La **consigne de
cohérence** (`Consistency` immediate/eventual/strict) s'applique dans
`SearchSourceNode` (`appliquer_la_consigne_pour`, bornée à la fermeture de
la cible), et le port `meta` de la source dit le reste en file et
`partial`. Le **domaine de travail** (service `WorkDomain`) rétrécit la
recherche par `filter_condition` si l'entité a les champs — sinon il
l'avoue en avertissement. La source accepte un `QueryPayload` par son port
`query` optionnel (pas B) : câblé, il remplace la fiche entière.

## 6. Gabarits et catalogue

`templates/entities/*.json` : des `EntityConfig` prêts avec fiche
descriptive (category, description, note), posés par l'outil `place`
(famille `entity`), adoptés par `adopt`. Les dérivées : `DerivedConfig`
(`from`, `gather: Vec<GatherRule>`, `render: BTreeMap<champ, jinja>` —
gabarits dans `templates/render/`) ; une dérivée a ses tables `{kb}`,
`{kb}_Chunk`, `{kb}_DERIVED_FROM`, `{kb}_CHUNKED_FROM`, champs
`title`/`content`, et sa dette de rendu vit en base et se rattrape
(`rendre_le_retard`, et une recherche stricte à la réouverture solde la
dette). Les trois gabarits de dérivées proposés (pas C, étape 3) :
fil-et-racine, document-et-sections, entité-et-étiquettes — voir
`docs/2-octobre-2026-00h43/01-ponderations-dans-les-graphes.md` §4.

## 7. Les suites de test de la recherche, et ce que chacune garde

- `e2e_generic_search` (18) : les contrats du lanceur (Detailed de bout en
  bout, diagnostics, erreurs typées, `meta.signals` demandés), le port
  d'entrée de la source, la page et les comptes ; helper `cherche`
  (envelopper → lancer → `Arc::try_unwrap` → rendre le catalogue).
- `e2e_code` (24) : les onze verbes sur un vrai dépôt ; **corpus vivant =
  `src/dataflow/port.rs`** (`read_sources` filtré) — ne jamais l'alourdir ;
  le seuil « markdown ×3 < JSON » et « `### 1.` exact » sont les canaris de
  la fusion ; 16 restructurations de guards (verrou non réentrant).
- `e2e_scope` (9) : cellules multi-tenant ; le canari kuzu « la projection
  est honorée par QUERY_VECTOR_INDEX » (nom d'index demandé à
  `vector_storage`, jamais codé en dur).
- `e2e_search` (39), `e2e_result_mode` (10), `e2e_catalogue_gabarits` (12 ;
  l'embarquement de la requête utilise l'embedder **du catalogue**),
  `e2e_simple_entity`, `e2e_entites_derivees` (3 ; receveurs nus + helper
  `cherche`), `e2e_search_queue` (5 ; la consigne à travers le graphe
  stratégie), `e2e_dataflow_observe` (7 ; noms `query_source` /
  `primary_search`, ports `render.*`, tap sur `source.query`),
  `e2e_agent_loop` (8 ; traces à deux étages, les 9 nœuds internes
  **nommés**), `e2e_undo` (son `VERROU_BASE` sérialise à dessein — ne
  jamais « corriger » ; helper local homonyme `search_bm25`),
  `e2e_prise_atomique` (refus sporadique connu du verrou lecteur/écrivain,
  porté au cœur C++ le 18), `e2e_postgres` (migré à l'aveugle, attend un
  serveur), `e2e_graph_tool` dans la liste de livraison du pas C.
- Tous en `#[ignore]`, joués par `./run_e2e.sh --test <suite>` (features
  `rag3db-native,burn-embedder,burn-ocr,code,daemon`), **une suite à la
  fois** quand le target est partagé.

## 8. Le banc (`e2e_banc_etage`) et ses pièges

Corpus vivant = **tout `src/`** : chaque ligne de code ajoutée bouge ses
chiffres d'un cheveu. Il mesure : « tel quel » (le chemin réel par
`rechercher`), M1 (texte à l'aiguille), M2 (cosinus exact), M3 (les 20
chunks bruts du HNSW — variance de reconstruction mesurée 0,206→0,241
entre deux passes identiques), M1b, G (filtre par genre). **Aiguilles
fragiles** : des `("fichier.rs", "pub fn signature(")` en dur — le
nettoyage du 18 en a cassé deux en silence (réparé `3711da0ca`) ; après
toute fusion, **le banc doit compiler et tourner**. Référence du 2 octobre
(granite-278m, iGPU Flex32, 5 247 scopes, 43 questions) : tel quel 0,333 /
9 / 24 ; G **0,412 / 13 / 27** — c'est la cible de la pondération par
genre, et on ne compare **qu'à cette référence** (celle de septembre :
autre corpus, autres poids, autre poste). Le banc ne sort pas le rang par
question (manque noté — s'il le faut, l'ajouter dans le banc, pas dans
`src/`). Les quasi-ex-æquo se jouent à +0,002 de cosinus : une
reconstruction de HNSW suffit à flipper un rang 1.

## 9. Retiré le 2 octobre (`01791e347`), et où sont passés ses tests

- `search::fuse_results` (la forme à trois listes) : ses onze tests —
  seuls tests unitaires de la fusion — **portés sur `fuse_signals`**.
- La grappe d'exploration (`search_with_explore`, `explore_bfs`,
  `ExploreOptions/Result/Graph`) : supprimée ; `UnifiedResult.graph` était
  toujours `None`.
- `search_with_strategy` : supprimée ; sa garde `max_rounds` vit dans
  `build_dataflow_graph` (qui rend `Result`).
- `sparse-vector` alignée en 4.3.0 (`7c653f66c`) avant la mesure sparse.

## 10. Vérifié / non vérifié

**Vérifié** (tests datés) : tout le §1-§5 côté master au 2 octobre ; la
non-régression du chemin unique (17 suites vertes sur le tronc au 18, 43
questions du banc au 2).
**Non vérifié** : l'étape 1a de `pas-c-ponderations` (`130984f61`) n'a
jamais compilé — le rouge des quatre tests est le premier geste de la
reprise ; le passage du rôle `Boost` en « structurel, tous étages » est un
changement de comportement **documenté mais non prouvé** (aucun graphe ne
combinait boost et fusion déclarée) ; le poids sparse n'a jamais été
mesuré ; les valeurs de `Scope` (« file 0,6, namespace 0,7 » du doc de
conception) sont des exemples, **pas des mesures**.
