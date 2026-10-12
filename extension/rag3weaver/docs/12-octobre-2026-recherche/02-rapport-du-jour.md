# Rapport du jour, session recherche (12 octobre 2026)

*Tenu après chaque lot. Le précédent est le rapport de nuit du 11
(`../11-octobre-2026-recherche/08-rapport-de-nuit.md`).*

## Le premier flux de jetons (décision 3 de l'orchestration)

La page de conception est `01-le-premier-flux-de-jetons.md`. L'orchestration
l'a acceptée, et le proto aussi, avec ses objections, retenues.

| lot | commit | ce qu'il fait |
|---|---|---|
| ma part, 1 | `38a95fa9b` | `has_streams` ne compte que les sorties de flux d'où part une arête. Une sortie de flux sans arête **jette** ses éléments : `emit` les compte seulement, ce qui garde « posée » une sortie requise. La fusion de fin de nœud, `DataflowOutput::stream()` et le relais des éléments d'un sous-graphe non branché disparaissent, remplacés par un compte. Le témoin de parité (`flux_tests.rs`) a été rouge d'abord : le même graphe, avec ou sans port `tokens` non branché, reste sous les niveaux et rend la même sortie entière, port à port. La page suit les points du proto : `Spot`, le sujet `tokens`, les flux non branchés jetés, « les ports portent des flux ». |

**Vert** :
- check_features : 5 jeux sur 5, code 0, aucune ligne ROUGE ;
- `Checking rag3weaver` prouvé après `touch src/lib.rs` ;
- lib : 1533 passés, 0 échec, rejouée sous le verrou après le rebase.

| ma part, 2 | `6945cd260` | Sur `LlmNode.tokens` du proto (`7627fa92d`). Le témoin du vrai modèle : qwen3-1.7b sous `llama-server`, sur le processeur, lancé et arrêté par le témoin. Sur 397 jetons, le premier est lu par le consommateur à 210 ms, 0,53 ms après le serveur, et la génération finit à 14,1 s. `text` est la concaténation exacte. La mesure, sur 100 000 fragments en debug : le canal coûte 822 ns par jeton, contre 35,6 ms par jeton du modèle. Page 01 §7. |

**Reste** : le refoulement d'un consommateur lent vers la socket. Il se
mesure sur `Spot` et `TokenEvents`, que le proto pousse après ses lots de la
montre (l'orchestration a mis la fraîcheur en tête).

## Le magasin de libs

La recette de rebâti de l'arbre principal publie désormais d'elle-même.
Premier rebâti publié : 03 h 49, sur `e6f626df4`, avec le correctif du
balayage filtré. Relu de mon côté :
- `courant` porte le même identifiant que `BUILD_ID` et que la section
  `.rag3db_build_id` ;
- la somme md5 de la lib est la même dans le magasin et dans
  `build/lecteurs-csv` (`c2ddbb67…`) ;
- celle de l'extension vector est conforme à l'annonce (`545d634c…`).

**Lot 3d** (`e38cc586b`, demandé par l'orchestration) : le pont lit lui
aussi la lib du magasin.
- `exporter_la_lib` fait le choix de `run_e2e.sh` et pose les quatre
  variables sur un même dossier.
- `check_features.sh` l'appelle.
- `scripts/cargo_magasin.sh` est l'entrée de tout cargo joué à la main ou
  par un push.
- Un rebâti en place de `build/lecteurs-csv` ne change plus la lib d'un
  rejeu. Le témoin de bout en bout le prouve sur un vrai binaire de tests :
  sur l'ancien environnement, il ne démarre plus ; par l'enveloppeur, il
  passe sans rien recompiler.
- Page 07 §9.

**Relu** : le correctif de G pour Windows et macOS (`2f052d5db`, son lot C),
du code mort hors Linux dans `build_check.rs`. `check_features` ne joue que
Linux, et c'est noté dans la page 07 (§5).

## Les leçons

- **check_features jette la sortie de cargo.** La preuve qu'un vert n'est
  pas vide, la ligne `Checking rag3weaver`, se fait donc à part : un
  `touch src/lib.rs` puis un `cargo check --tests` du jeu le plus large,
  dans un fichier.
- **Un worktree qui contient le sous-module** ne s'efface qu'avec
  `--force`, une fois son état lu propre. Sa branche ne s'efface qu'avec
  `-D`, après avoir prouvé que son bout est un ancêtre de master.
- **Le FETCH_HEAD de l'arbre principal est partagé** : une autre session l'a
  écrasé entre deux de mes lectures. Je lis master depuis mon worktree,
  jamais depuis l'arbre principal.
- **Deux worktrees sur un même target** se paient d'une recompilation
  entière à chaque bascule (la garde efface les empreintes). Je n'en lance
  pas un pendant que la poussée de l'autre attend le verrou.
