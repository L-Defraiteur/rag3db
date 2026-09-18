# Le dernier chemin parallèle — la stratégie, et trois écritures du même graphe

18 septembre 2026, tard le soir. Le monolithe `Catalog::search` est parti le
même jour (doc 01) ; le repérage qui suit dit ce qui reste de parallèle au
lanceur. Réponse courte : **le moteur est unifié, la construction ne l'est
pas** — il reste un chemin « stratégie » dont le graphe est écrit trois fois,
et un second point d'entrée « recherche contenable » à un seul usage.

## 1. L'état

| chemin | où | lignes | prod | tests | statut |
|---|---|---:|---:|---:|---|
| `Catalog::rechercher` | catalog.rs:6413 | 214 | 4 | ~218 | **le lanceur** |
| `SearchTool` (= `search_base.mmd` promu nœud) | graph_tool.rs:1755 | — | l'outil `search` | 15+ | sur le lanceur |
| `Catalog::search_with_explore` | catalog.rs:6988 | 36 | 0 | 1 | sur le lanceur + queue BFS hors graphe (arbitrage Lucie en cours) |
| `Catalog::search_with_strategy` | catalog.rs:7209 | 54 | **0** | 5 | composable mais parallèle |
| `Catalog::build_dataflow_graph` | catalog.rs:7131 | 70 | 0 (1 interne) | 8 | composable mais parallèle |
| `KBSearchNode` / `KBQuerySourceNode` | search_nodes.rs:88 | 52 | 1 site (catalog.rs:7164) | 6 | composable mais parallèle |
| `code_tools::grep_files` | code_tools.rs:706 | ~95 | `GrepNode` | — | autre domaine (fichiers, pas la base) — à assumer, pas à porter |

Le chemin stratégie ne duplique **aucune** étape du moteur : `KBSearchNode`
appelle `Catalog::rechercher` (search_nodes.rs:126), donc embarquement,
signaux, fusion, rerank, pagination et résolution sont exécutés une fois, au
bon endroit. C'est un runtime imbriqué : le graphe externe (4 nœuds) contient
un nœud qui lance le graphe interne (`search_base`).

## 2. Le doublon, précisément

Le graphe `query_source → primary_search → fetch_related_i → compose` est
écrit **trois fois** :

1. `Catalog::build_dataflow_graph` — 70 lignes de Rust, à la main ;
2. `templates/search_expansion.mmd` — 17 lignes, le même graphe nœud pour
   nœud, `include_str!`é seulement par un test de parsing (mermaid.rs:821) ;
3. le tronçon expansion de `templates/tools/search.mmd` — `SearchTool →
   FetchRelatedNode → ComposeNode` (+ `GroupFrameNode`/`RenderResultsNode`),
   l'outil vivant des agents.

À quoi s'ajoutent : l'enregistrement des services à la main (catalog.rs:7145,
au lieu de `register_search_services`), un service `reranker` mort (aucun
`RerankNode` dans ce graphe), une garde `max_rounds` statique qui ne borne
aucune boucle (le doc-comment « reactive expansion » promet ce que le code ne
fait pas), et une méta lue sur le seul `primary_search` — les durées des
expansions ne comptent nulle part.

`KBSearchNode` face à `SearchTool` : recouvrement partiel, un contrat chacun.
`KBSearchNode` reçoit sa requête **par un port** (`QueryPayload` complet,
`SearchOptions` entière) — branchable en aval d'un nœud qui calcule la
requête ; `SearchTool` la reçoit **par substitution de gabarit** (5 scalaires,
figés à l'instanciation), mais expose ses nœuds internes au graphe parent et
se compose en `.mmd`. Le mécanisme qui réconcilierait les deux existe déjà :
`rechercher` injecte `options` dans la config du nœud source après
instanciation (catalog.rs:6498), et `SearchSourceNodeFactory` sait la lire.

Enfin, un déménagement dû depuis longtemps : `UnifiedResult`, `ChildSummary`
et `source_info` vivent dans `search_strategy.rs` alors qu'ils sont le type de
port de **toute** la chaîne composable (port.rs:116, generic_search_nodes,
render, report, checkpoint). Le fichier porte le nom d'un chemin dont il ne
contient que ~70 lignes.

## 3. Le chantier proposé (dans l'ordre, chaque pas vert seul)

- **A. Une seule écriture du graphe d'expansion.** Le corps de
  `build_dataflow_graph` devient une instanciation de
  `search_expansion.mmd` (le motif exact de `rechercher`, catalog.rs:6484),
  services par `register_search_services`, service `reranker` mort retiré.
  70 lignes → ~20, et le gabarit cesse d'être un fossile de test.
- **B. `SearchSourceNode` gagne une entrée de requête.** Un port `query`
  optionnel (`QueryPayload` ou requête + options), en plus de la config —
  la généralisation du mécanisme d'injection de catalog.rs:6498. C'est le
  pas qui rend `SearchTool` branchable en aval.
- **C. `SearchTool` absorbe `KBSearchNode`.** `KBQuerySourceNode` +
  `KBSearchNode` + `templates/search.mmd` + `templates/search_expansion.mmd`
  tombent ensemble (52 l. + 2 gabarits + 2 fabriques) ; l'absorption inverse
  est impossible (nœud Rust opaque, runtime imbriqué à chaque exécution).
  Dépend de B.
- **D. Les types déménagent.** `UnifiedResult`/`ChildSummary`/`source_info`
  quittent `search_strategy.rs` (vers `dataflow/port.rs` ou un module de
  types du graphe) ; `search_strategy.rs` ne garde que la stratégie — ou
  disparaît avec elle, selon E.
- **E. À trancher avant d'ouvrir : le sort de `search_with_strategy`.**
  0 appelant de production, 13 sites de test (e2e_search_queue éprouve la
  **file d'écriture** à travers lui, e2e_dataflow_observe éprouve
  l'observation à travers `build_dataflow_graph`). Trois options : le
  supprimer et porter ces tests sur le gabarit instancié (A rend ça
  presque gratuit) ; le garder comme surface de commodité sur le gabarit ;
  le fondre dans `rechercher` (une `SearchOptions` avec expansions ?). Ma
  pente : A d'abord — après A, la suppression est un pas court, et les
  tests d'observation ne perdent rien.

Hors périmètre, explicitement : `grep_files` (il cherche dans les fichiers,
pas dans la base — le contenir dans `GrepNode` est le bon état final) ; la
grappe explore et `fuse_results` (arbitrages de Lucie en cours, doc 01 §7) ;
le serveur/démon (aucune entrée de recherche, vérifié).

## 4. Ce que ça rapporte

Une seule écriture du graphe d'expansion au lieu de trois ; un seul point
d'entrée « recherche contenable » (`SearchTool`), avec le port d'entrée qui
manquait aux graphes d'agents pour calculer leurs requêtes en amont ;
`search_strategy.rs` qui cesse de cacher l'infrastructure du graphe sous le
nom d'un chemin mort ; et ~250 lignes de moins une fois E tranché en faveur
de la suppression.

## 5. Livraison (18 au soir, tard)

A, B, C, D livrés dans l'ordre, chacun vert seul (lib 977 et les suites du
chemin à chaque pas ; C : observe 7, agent_loop 8, code 24 — les traces
montrent désormais les neuf nœuds internes de la recherche sous le run du
graphe stratégie). `KBQuerySourceNode` reste : c'est l'émetteur de payload,
et l'absorber exigerait un param `options` sur la fiche de `search_base`,
surface héritée par l'outil des agents — à arbitrer si on le veut.

Une leçon en passant : le pas D avait d'abord posé `UnifiedResult` dans
`port.rs` — et `e2e_code` a rougi, JSON gonflé de 6 963 à 17 511 caractères.
Rien de cassé : **`port.rs` est le corpus vivant du test** (`read_sources`
filtré sur lui), et 300 lignes de plus y diluaient `merge_port_values`. Le
test-canari a fait son travail ; les types vivent dans
`dataflow/resultat.rs`, leur propre module, et le corpus n'a bougé que d'une
ligne d'import. E — le sort de `search_with_strategy` — attend le mot de
Lucie, porté avec explore et `fuse_results`.
