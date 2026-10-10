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
| 1 — moteur de script générique, rhai + TypeScript/JavaScript | **vert**, prêt à fusionner (page `03-lot-1-le-moteur-de-script.md`) |
| 2 — nœud entièrement scripté | — |
| 3 — rechargement à chaud | — |
| 4 — route → graphe → vue, `serve` | — |
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
