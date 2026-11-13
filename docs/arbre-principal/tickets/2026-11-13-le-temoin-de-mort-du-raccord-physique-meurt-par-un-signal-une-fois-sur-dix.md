# Le témoin de mort du raccord physique meurt par un signal, une fois sur dix

- **État** : ouvert (13 novembre 2026), relevé par embarquement ; pour arbre principal, à prendre
  après les deux reprises du lot 1a des vues (décision de c7).
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
