# Le témoin de mort du raccord physique meurt par un signal, une fois sur dix

- **État** : corrigé (19 novembre 2026, 46101db7b5, PR #50), arbre principal ; relevé par embarquement le
  13 novembre.
- **Famille** : produit
- **Gravité** : la lib entière rougit sans cause dans le lot qui la joue, une fois sur dix.
- **Atteignable en service** : non, tant que le raccord physique n'est pas adopté par l'index réel.
  Depuis la décision de Lucie du 13 novembre, le raccord est un banc d'essai des mécanismes.
- **Touche rag3weaver** : oui (`src/dataflow/migrations/physical_index/tests.rs`, le raccord physique).

## Ce que c'est

`dataflow::migrations::physical_index::tests::native_process_death_recovers_same_program_and_lost_ack_once`
a rougi une fois sur dix tours de la lib entière, et les neuf autres tours ont rendu 2220/0 :

- à luciepc, sur master `00e9fde57` ;
- lib `rag3db-build-835d5298f…`, sans `RAG3WEAVER_EMBED_SERVICE` ;
- sous `POSTE_MEM_MAX=32G poste lourd`.

```
panicked at src/dataflow/migrations/physical_index/tests.rs:545:9:
assertion `left == right` failed: after-lexical-durability:
  left: None
 right: Some(91)
PHYSICAL_FAILED_FIXTURE /tmp/.tmpGr3L1P
```

Avant la panique, quatre lignes `PHYSICAL_WITNESS` (`engine_storage_version` 43) : les quatre
premiers points de coupure sont passés.

## Ce que dit `left: None`

Le témoin lance un processus enfant (`physical_crash_child`) qui doit sortir par le code **91** au
point de coupure demandé, ici « after-lexical-durability ». `None` est le code de sortie absent :
l'enfant n'est pas sorti, il a été **tué par un signal** (SIGSEGV, SIGABRT, SIGKILL…) avant ou
pendant ce point. Ce n'est pas une mauvaise reprise : c'est l'enfant lui-même qui meurt autrement
que prévu. Le message de l'enfant (sa sortie standard et d'erreur) est dans la sortie du tour.

## Les traces gardées, à luciepc

- la sortie entière du tour : `~/.cache/rag3weaver-build/activation/tour-7.log` ;
- la fixture : `/tmp/.tmpGr3L1P`.

Embarquement n'y a pas touché.

## Ce qu'il faut pour le fermer

1. Lire, dans `tour-7.log`, la sortie de l'enfant au stade « after-lexical-durability » : quel
   signal, et où.
2. Départager un défaut du raccord (son point de coupure après la durabilité lexicale), du moteur
   ou de lucivy (un fil de fond au moment de la sortie), ou de l'environnement.
3. Si le raccord n'est pas adopté par l'index réel, poser la question à Lucie : garder ces témoins
   dans le jeu courant de la lib, ou les sortir du jeu par défaut. Un banc d'essai ne doit pas
   faire rougir les lots des autres.

## Une occurrence de plus (18 novembre 2026, 22 h 5x)

La même signature, à la même ligne (`tests.rs:545`, « after-lexical-durability », `None` contre `Some(91)`), dans la
lib entière de la passe du lot 4 de 2a′-3 :

- à luciepc, branche `arbre-principal/couches-de-travail`, bb8cc375a8 ;
- lib 3d706f709, sous `poste lourd`.

Le lot ne touche ni le raccord physique ni ses tests : 2 632 autres tests verts, et les suites e2e vertes dans la
même passe. Le rejeu de la même tête juge.

## Fermeture (19 novembre 2026)

**La cause, trouvée par le vidage.** À luciepc, sur le code d'alors, le témoin a rougi 1 fois en 40 tours, au point
« after-lexical-durability ». Le noyau a noté une faute de segmentation dans un fil `scheduler-4`, et le vidage
(`coredumpctl`, puis gdb, sans ptrace) montre deux choses au même instant :

- le fil principal est dans `exit()` → `_dl_fini`, appelé par `physical_boundary` (`std::process::exit(91)`) ;
- un fil `rag3db::common::TaskScheduler` est dans `antlr4::Lexer::emit`, en train d'analyser une requête, et saute dans
  une table que les destructeurs statiques viennent de détruire (SIGSEGV, SEGV_ACCERR).

Ce n'était donc ni la reprise ni le moteur en service. La **sortie simulée** lançait les destructeurs pendant qu'une
requête était en vol, alors qu'une vraie mort n'en lance aucun.

**Le correctif** (46101db7b5) : toute mort simulée d'un témoin passe par `fautes_de_test::simulated_death`, qui sort par
`libc::_exit`. Cela vaut pour la coupure du raccord (91) et pour l'enfant de la publication native (73 et 74). Les
autres `process::exit` relevés ne sont pas des morts simulées : `e2e_tx_par_paquet_arret` meurt déjà par SIGKILL,
`e2e_arret_brutal` est tué par son parent, et les autres rendent un résultat d'enfant.

**La série** : 0 rouge sur 150 tours sur le correctif, soit 120 tours du raccord et 30 de la publication native.

**À garder** : une requête s'analysait en fond pendant `execute_initial_batch`, au point « after-lexical-durability ».
Ce n'est pas un défaut pour une mort simulée, mais c'est un fil vivant à connaître si le raccord est un jour adopté par
l'index réel.
