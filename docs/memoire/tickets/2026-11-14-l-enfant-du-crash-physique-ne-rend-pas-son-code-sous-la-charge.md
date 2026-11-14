# L'enfant du crash physique ne rend pas son code 91 sous la charge d'une lib entière

- **État** : ouvert, le 14 novembre 2026. Instable sous la charge, vert seul. Il est inscrit dans `instables.txt` du poste luciepc, qui le rejoue seul quand il rougit sous la charge.
- **Famille** : produit
- **Gravité** : faux rouge. Un tour rougit sans que son lot en soit la cause. Un vrai défaut derrière n'est pas écarté.
- **Atteignable en service** : non vérifié. Le test tue un processus enfant pour simuler une mort brutale ; il reste à savoir si la reprise elle-même en dépend.
- **Touche rag3weaver** : oui, `src/dataflow/migrations/physical_index/tests.rs` (le raccord physique, 1e9bf5089 et 73586bf92).
- **Pour** : arbre principal, qui décide s'il y a un vrai défaut derrière.
- **Signalé par** : session mémoire (96), au rejeu du lot archjour, le 14 novembre 2026 à 13 h 11.

## Ce qu'on voyait

`dataflow::migrations::physical_index::tests::native_process_death_recovers_same_program_and_lost_ack_once`
a rougi une fois pendant `cargo test --lib` complet (2363 passés, 1 en échec). C'était à luciepc, sur
master 52575b6ba avec la lib 0c41f9fe6. L'assertion de `tests.rs:545` porte sur l'étape
`after-lexical-durability` : elle attend le code de sortie 91 de l'enfant (`physical_crash_child`) et
reçoit `None`, une fin par signal ou sans code.

Seul, sur le même arbre, la même lib et le même target, le test passe 13 fois sur 13 (3, puis 10).

## La fixture

Gardée par le test sous `/tmp/.tmp678Ni9`, qui est de la mémoire vive, et copiée à l'identique (1,6 Mio)
à luciepc sous `~/.cache/rag3weaver-build/fixtures-gardees/physical-crash-2026-11-14/`.

## Ce qui reste à savoir

- Comment l'enfant finit sans code sous la charge : tué par un signal, par le délai du test, ou par le
  manque de mémoire (POSTE_MEM_MAX valait 32G pour la passe).
- Si le délai d'attente de l'enfant est trop court sous la charge, ou si la reprise elle-même manque
  son accusé sous la charge.
