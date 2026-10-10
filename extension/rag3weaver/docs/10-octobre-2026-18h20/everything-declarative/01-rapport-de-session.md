# Everything declarative (chantier I) — rapport de session

Ouvert le 10 octobre 2026 au soir. Pour l'orchestration (`rag3db-c7`), pour
Lucie, et pour la session qui reprendrait. La page de conception est
`../orchestration/03-le-proto-tout-declaratif.md` ; ce qui est su du code est
dans `02-knowledge-dump.md`, à côté.

## 1. Où l'on travaille

- Poste : **luciepc**, worktree `~/git_workspaces/rag3db-proto`, branche
  `proto-declaratif`, partie de `origin/master` à `51a92a4cd`.
- Target unique : `~/.cache/rag3weaver-build/target-I` (là-bas),
  `CARGO_INCREMENTAL=0` aux suites, gardé entre les lots, effacé au-delà de
  60 Go ou en fin de chantier.
- Tout lourd sous `~/.cache/rag3weaver-build/poste lourd`, `timeout` devant
  tout test, `-j 8` au plus.

## 2. Ce qui est décidé

- **Langage déclaré dès le lot 1 : `typescript`** (Lucie, 10 oct., par
  l'orchestration) : les types sont retirés au chargement par un outil Rust,
  puis le JavaScript va au moteur embarqué. Pas de vérification de types à la
  `tsc` : ce sont les ports du nœud qui vérifient entrées et sorties à la
  frontière. `javascript` reste accepté (du TypeScript sans types) ; `rhai`
  reste le branchement existant.
- Tranché par la session, accepté par l'orchestration, réversible : le
  serveur de `serve` (lot 4) reprend `tiny_http` et `daemon::servir`, déjà
  dans le crate, plutôt qu'axum.
- **Moteur JavaScript : rquickjs 0.14** (quickjs-ng 0.16.2), avec trois
  gardes : arrêt par échéance (`set_interrupt_handler`), plafond mémoire
  (`set_memory_limit`, sans les features `rust-alloc`/`allocator` qui le
  rendent sans effet), un `Runtime` par appel. **Retrait des types :
  `swc_ts_fast_strip` en `Mode::StripOnly`** (« TypeScript effaçable »,
  le même code que Node) : lignes et colonnes gardées, donc la ligne d'une
  erreur est juste sans carte de source ; `enum`, `namespace`, propriétés de
  paramètre de constructeur et `<T>expr` sont refusés en le nommant.
  Dépendances **non optionnelles**, comme rhai : TypeScript est le langage du
  produit. À vérifier avant fusion : qu'elles se lient sous Windows MSVC et
  macOS arm64 (paquet npm de `rag3db-90`, à prévenir à la fusion du lot 1).
- **Noms possédés dans le moteur de graphes, en tête du lot 2** : `Arc<str>`
  (ou `Cow<'static, str>`) pour les noms de `PortDef`, `NodeSchema`,
  `ConfigParam` et `node_type()`, rien d'autre ; relu par `rag3db-97` ;
  témoin : mille rechargements du même nœud, mémoire stable. Pas de fuite
  bornée acceptée.
- **Lib commune** : rebâtie par le cœur C++ (`rag3db-91`) après chaque push
  de C++ ; prouver sa date contre la tête avant toute suite. Les lots 1 et 2
  n'en ont pas besoin.
- **Rebase avant le lot 4, pas avant** : `RunScope` et l'exécution
  asynchrone (`rag3db-97`), `src/mcp.rs` (`rag3db-96`) arrivent sur master.
  Le lot 3 se branchera sur le montage `Reactor::watch_bound` de la session
  mémoire (les `reactions` du manifeste, aujourd'hui chargées sans être
  montées).
- Coordination : tout diff de `harness.rs` s'annonce à `rag3db-97` avant la
  fusion.

## 3. Les lots

| Lot | État |
|---|---|
| 1 — moteur de script générique, rhai + TypeScript/JavaScript | **sur master** (`6932a7da2`, page `03-lot-1-le-moteur-de-script.md`) |
| 2 — nœud entièrement scripté | **sur master** (`a944079a4`, page `04-lot-2-le-noeud-scripte.md`) |
| 3 — rechargement à chaud | **sur master** (page `05-lot-3-le-rechargement-a-chaud.md`) ; la montre des fichiers attend `watch_bound` |
| 4 — route → graphe → vue, `serve` | 4a-4c **sur master** (page `06-lot-4-route-graphe-vue.md`) ; 4d le jouet et son e2e en cours |
| 5 — l'outil `declare` | — |
| 6 — la page vivante | — |

## 4. Lot 1, ce qui s'est passé (10 oct., nuit)

- Le code : `src/script/` (interface, rhai, QuickJS, TypeScript, témoins),
  `harness::evaluate` qui délègue, `RhaiLimits` alias de `ScriptLimits`,
  trois dépendances (`rquickjs` 0.14, `swc_ts_fast_strip` 59, `swc_common`
  26 pour ses diagnostics). Page : `03-lot-1-le-moteur-de-script.md`.
- Témoins : rouges avec des bouchons (14 rouges, rhai et `harness::` verts),
  puis 18 sur 18. Deux corrections en route, par les témoins : QuickJS en
  mode script lit `import fs from 'fs'` comme une erreur sur `fs` (reclassé
  par la ligne écrite) ; `import('fs')` rend une promesse rejetée (les tâches
  sont vidées, en nombre borné, et le rejet se dit par sa raison).
- Suites voisines, après rebase sur `01dc4c7ce` (master n'apportait que du
  C++ de statistiques et des docs), lib commune de luciepc à `5798567dc`
  (aucun C++ entre elle et la tête) : `--lib` avec les features de
  `run_e2e.sh` (`rag3db-native,burn-embedder,burn-ocr,code,daemon`) 1 278
  verts, 0 rouge ; `--tests --no-run` avec les mêmes : tout compile. Aucun
  e2e ne passe par rhai (les gabarits `memory`/`validated-result` sont
  chargés par `chaque_gabarit_livre_se_charge`, dans la lib).
- Pas joué : `scripts/test_backend_harness.py` (chemins `target/debug` et
  `build/` en dur, pas de target par session) ; son chemin rhai est couvert
  par les tests de `validation_nodes` et `ref_nodes`.
- Une erreur de ma part : la première passe des voisins sans features
  (trois rouges et des cibles non compilées, tous « feature `code` »).
- Taille ajoutée au binaire : non mesurée.

## 5. Lot 1 sur master, plateformes et poids (11 oct.)

- Fusionné en avance rapide `01dc4c7ce..6932a7da2` après la relecture de
  `harness.rs` par la session recherche (aucune objection).
- Plateformes (par `rag3db-90`, sur `paquet-npm`) : manylinux_2_28 vert,
  macOS arm64 vert (`npm test` sur le runner), Windows MSVC en cours.
- **Poids** : binaire Linux 79 → 89 Mo strippé, macOS 81 → 93. Mesuré sur un
  binaire témoin en release : QuickJS +1,35 Mo, `swc_ts_fast_strip` +7,2 Mo
  (+4,85 avec LTO complète ; le parseur swc seul +2,1). Options rendues à
  l'orchestration : garder ; LTO au profil de publication ; notre effaceur
  sur le seul parseur swc ; oxc. Recommandé : garder pour le proto.

## 6. Lot 2 (11 oct., nuit)

- **Noms possédés** : `PortDef`, `ConfigParam`, `NodeSchema` en
  `Cow<'static, str>`, `NodeFactory::node_type()` emprunté, registre par
  `String`. **Deux fuites trouvées en service** par le témoin (un allocateur
  qui compte) : `GraphNode::from_definition` fuyait chaque nom de port libre
  à chaque création de nœud, `graph_tool.rs` les noms et descriptions des
  paramètres d'un outil lu d'un fichier. Rouge (+193 000 / +72 000 octets
  pour mille), puis stable. Fait par trois scripts rejouables
  (`~/.cache/rag3weaver-build/I/owned_names*.py` sur luciepc et ici, et
  `apply-owned-names.sh`). Relu et approuvé par la session recherche, **à
  commiter après sa fusion**, scripts rejoués sur sa tête.
- **Le nœud scripté** : forme décidée par l'orchestration (déclaration
  `nodes/<nom>.node.json` + script relatif, dossier découvert), JSON Schema
  par port vérifié à la frontière, configuration vérifiée à la création,
  script préparé une fois et partagé, nombres sous une seule forme. Témoins
  rouges sur l'exécution (5, bouchon) puis 11 sur 11 ; lib complète avec les
  features de `run_e2e.sh` 1 289 verts, 0 rouge.
- Hors lot, dit dans la page : le branchement du dossier `nodes/` dans un
  backend et les listes blanches (lot 4), ce que le script peut appeler,
  `Node::node_type()` possédé (après la fusion de la recherche).
- Erreurs en route : un bâti lancé sans les variables de la lib commune
  (arrêté par son pid, vérifié dans `/proc/<pid>/cwd`) ; le script des
  littéraux qui sautait les littéraux imbriqués et prenait un type de retour
  pour un littéral (corrigé, rejoué depuis les sources).
