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

**Attend** : le hash du proto pour `LlmNode.tokens`, avec son puits double,
`TokenEvents` et `Spot`. Ensuite, chez moi :
- le témoin du modèle local : `llama-server` et qwen3-1.7b, lancés puis
  arrêtés par le témoin ;
- la mesure :
  - le surcoût par jeton, sur 100 000 fragments ;
  - le premier jeton chez le consommateur, comparé à la fin de la
    génération.

## Le magasin de libs

La recette de rebâti de l'arbre principal publie désormais d'elle-même.
Premier rebâti publié : 03 h 49, sur `e6f626df4`, avec le correctif du
balayage filtré. Relu de mon côté :
- `courant` porte le même identifiant que `BUILD_ID` et que la section
  `.rag3db_build_id` ;
- la somme md5 de la lib est la même dans le magasin et dans
  `build/lecteurs-csv` (`c2ddbb67…`) ;
- celle de l'extension vector est conforme à l'annonce (`545d634c…`).

## Les leçons

- **check_features jette la sortie de cargo.** La preuve qu'un vert n'est
  pas vide, la ligne `Checking rag3weaver`, se fait donc à part : un
  `touch src/lib.rs` puis un `cargo check --tests` du jeu le plus large,
  dans un fichier.
- **Un worktree qui contient le sous-module** ne s'efface qu'avec
  `--force`, une fois son état lu propre. Sa branche ne s'efface qu'avec
  `-D`, après avoir prouvé que son bout est un ancêtre de master.
