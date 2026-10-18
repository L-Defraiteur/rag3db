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

| le refoulement | `00079b754` | Sur `TokenEvents` et `Spot` du proto. Un lecteur lent du graphe retient le modèle (avance maximale de 65 fragments pour un canal de 64). **`TokenEvents` vers un abonné lent du bus ne refoule pas : il perd** (1 932 jetons sur 2 000 à 1 ms par jeton, sur un bus de 64 ; rien au rythme réel d'un modèle). `Spot` ne pèse rien (sous le bruit du canal). Page 01 §8 ; les deux voies pour un écran qui ne doit rien perdre sont au proto et à l'arbre principal. |

## Lancer un graphe en fond (demande de Lucie du 11 octobre)

La page est `03-lancer-en-fond.md`, acceptée par l'orchestration et par le
proto, avec ses deux objections retenues (la fin va à un nom ; l'enfant n'a
jamais plus que son appelant).

| lot | commit | ce qu'il fait |
|---|---|---|
| A | `371ded8f9`, `4a2f734dc` | `DataflowRuntime::spawn` : un fil à lui, sous l'exécuteur concurrent, et la référence rendue tout de suite (34 µs). La fin est écrite dans la boîte de l'appelant, signée `outil #h`, et dite sur le bus (`RunFinished.cause`). `Detached` par défaut, `WithParent` déclaré. Le registre des runs vivants (`alive` est le `--qui`, `cancel` est réservé au parent ou à l'appelant), avec la durée et le nombre à la fois. `suivies()` est fondu dedans. Neuf témoins, plus qwen3 sous `llama-server` (un résumé dans la boîte à 1,48 s). |
| C | `80b97af1d` | `SpawnNode` : le verbe posé comme nœud, un lancement par message, un refus dit sur `run`. |
| deliver | `51025501e` | La fin passe par `Directory::deliver` (la boîte par nom du proto) : la fin d'un appelant au tour fini va à son tour suivant. |
| ordre | `fa5fa55bd` | `RunFinished` est émis avant que la fin ne soit écrite dans la boîte (une course vue par le proto). |
| flux ouvert | `ccf4c49b9` | Le second temps (page 04) : un run lancé vit tant que son flux d'entrée est ouvert. L'appelant le nourrit par `LiveIn` et le lit par `LiveOut`, deux canaux bornés qui refoulent à travers la frontière (le pont des sous-graphes, `execute_spawned`). Le registre tient les poignées : un tour qui finit ne ferme rien. La durée ferme l'entrée, puis annule après la grâce. Le minuteur et l'appelant sont des acteurs du hub : un appelant qui pense n'est pas un interblocage, un vrai l'est et il est dit. `SpawnNode` a son mode flux. Neuf témoins verts vingt fois de suite, dont qwen3 sous `llama-server`. Mesure : 7,8 µs l'aller-retour, 5,5 µs par message en continu, aucun perdu. |

**Reste** : le verbe `spawn` dans la boucle est chez le proto, et
`qui()`/`cancel_all` dans `daemon_status` chez l'arbre principal.

## L'annulation sous tous les exécuteurs (page 05)

Écrite pendant la pause de Lucie (12 au 18 octobre), **sans bâti ni test**.
Rebasée et relue le 18 sur `d888b6365` ; elle attend le jeton de
compilation de l'orchestration.
- Un run par niveaux a son hub d'annulation, attaché à la poignée d'arrêt ;
  ses nœuds le lisent par leur jeton, la boucle le regarde en tête de chaque
  niveau. Un run annulé rend `Ok` avec `stopped()`, et ses nœuds non lancés
  sont dits arrêtés.
- `EmitSink` s'arrête à l'annulation (donc `LlmNode` et `Decide`) ;
  `WaitOutputNode` dort par le jeton.
- `RunFinished` dit la cause de tout run arrêté, en identifiants
  (`requested`, `parent_finished`, `timeout`, `failed:<nœud>`, `deadlock`,
  `host_stopped`), et `cancelled_by`.
- `spawn` n'impose plus l'exécuteur concurrent à un run sans flux.
- La relecture du 18 a trouvé deux défauts, corrigés avant tout bâti : la
  cause était rangée par runtime (deux runs d'un même runtime pouvaient se
  la voler ; elle l'est par `run_id`), et un run en fond sans cause notée
  par le registre se disait `requested` même arrêté par un interblocage.

**À faire au jeton** : les 5 jeux, les rouges prouvés sur le code d'avant,
le lancement en fond rejoué vingt fois, la lib, la poussée. Puis la mesure
de dix runs en fond, pour chiffrer les bornes par défaut.

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

**Lots 3e et 3f** (`f3b0b2dcc`, `70327a3fd`) : l'extension vector se
cherche à côté de la lib réellement chargée (`/proc/self/maps`), en
production puis dans les tests, les e2e et les sondes. Une extension d'un
autre bâti est passée quand elle est cherchée, refusée quand elle est forcée,
et les deux identifiants sont nommés. Le lot 3f ferme le rouge de 12 h 58
(un rebâti en place avait fait tomber un rejeu sur la lib du magasin).

**Autres** :
- `run_e2e.sh` refuse plus d'un filtre positionnel (`af7fc62cc`) ;
- la règle des bases e2e est écrite dans la page des e2e (`680517566`) ;
- le ticket du réacteur instable est fermé (`7c08bd5f5`) ;
- relus : l'IR du proto (`9d6504f2c`) et le lot C de G.

**Refusé à moi, laissé à Lucie** : l'effacement de deux restes de bases
d'arrêt brutal sur luciepc (42 et 110 Mo, sous
`~/.cache/rag3weaver-build/`). Mon mode de permission refuse une écriture
sur un hôte distant. La commande est dans le message à l'orchestration.

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
- **Un témoin qui lit deux canaux l'un après l'autre** (la boîte, puis le
  bus) attrape un ordre d'écriture faux sous charge. Le correctif est
  l'ordre, pas une attente.
- **Un rejeu complet trouve ce que les tests du lot ne voient pas** : un jeu
  de features sans `code` (du code mort), ou le témoin des schémas sans
  configuration d'exemple. Deux poussées l'ont prouvé aujourd'hui.
- **Une mesure qui donne un coût négatif mesure autre chose.** `Spot` « moins
  cher » qu'un lecteur nu, c'était le réveil par fragment d'un lecteur qui
  vide le canal.
