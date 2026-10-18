# Un bâti d'index prend tous les fils de travail et affame les autres connexions, lecteurs compris

- **État** : ouvert
- **Gravité** : blocage. Pendant la phase parallèle du bâti, aucune autre connexion n'avance,
  lecteurs compris. La règle de Lucie est que personne ne soit bloqué.
- **Atteignable en service** : oui, dès que le moteur a peu de fils. C'est le cas du portable de
  Lucie, qui garde deux cœurs libres, et d'un pod sous quota de CPU.
- **Touche rag3weaver** : oui. Pendant qu'un index se bâtit, chaque appel d'outil attend la fin de
  sa phase parallèle.
- **Ouvert le** : 18 octobre 2026, banc de concurrence, à la demande de l'orchestration, d'après
  l'hypothèse de hotfixs.
- **Pour** : cœur C++, ou hotfixs, au choix de l'orchestration.

## Ce qu'on voit

Le scénario des vingt écrivains (`TwentyWriters.DdlEveryFifteenSeconds`) a été joué sur luciepc,
sous `poste mesure`, à 7ca43016f (master b665d94c3), avec un tampon de 2 Gio et le point de reprise
automatique en marche. Sont comptées les transactions qui durent plus d'une seconde et
chevauchent `CREATE_VECTOR_INDEX`, datées depuis le début du DDL. Les trois variantes sont vertes.

| variante | durée du CREATE | attentes de plus d'1 s pendant le CREATE | point de reprise forcé |
|---|---|---|---|
| deux fils (la configuration des tests) | 11,33 s | **les 26 acteurs** (20 écrivains de Doc, 2 de Note, 4 lecteurs), commencées de 51 à 71 ms, finies de 9 684 à 9 713 ms | 510 ms |
| les DDL limités à un fil (`CALL threads=1` sur leur connexion) | 21,96 s | aucune pendant le bâti ; 10 écrivains de Doc pendant le point de reprise forcé de la fin (de 21 075 à 22 294 ms) | 872 ms |
| seize fils (`MAX_NUM_THREADS=16`) | 4,38 s | aucune | 768 ms |

Une passe antérieure, à deux fils, avait déjà montré la même forme : tous les acteurs retenus de
43 à 62 ms, puis jusqu'à 9,6 s.

## La cause

Elle a été proposée par hotfixs et confirmée par les trois variantes. Le bâti en mémoire du CREATE
(`CreateInMemHNSW`, puis `_FinalizeHNSWIndex` et les insertions de relations) est une fonction de
table parallèle, découpée en morceaux sur `[0, numRows)`. Elle prend tous les fils de travail du
`TaskScheduler`. La requête de toute autre connexion est une tâche de plus dans la même file : elle
attend un fil libre, et ne commence qu'à la fin de la phase parallèle du bâti, d'un coup pour tous.
La demande de point de reprise automatique (250 ms au plus) et le point de reprise forcé de la fin
n'y sont pour rien.

## Forme à étudier

Une file équitable, comme le temps partagé des moteurs établis :
- un bâti d'index n'a jamais tous les fils, et en laisse au moins un aux autres connexions ;
- ses tâches cèdent la place entre deux morceaux.

Le même problème touche tout DDL long qui bâtit en parallèle.

## Témoin

`IndexCreationThreads.LeavesAThreadToReaders` (`test/transaction/concurrence/index_creation_threads_test.cpp`),
dans `known_red.txt`. La base a deux fils et 50 000 lignes de vecteurs `FLOAT[64]`. Une connexion
crée l'index, une autre lit une ligne par sa clé, en boucle, pendant le bâti. Contrôle
`index-creation-leaves-a-thread-to-readers` : aucune lecture n'attend plus de 2 s, ce qui couvre
largement le point de reprise forcé de la fin.

Le scénario des vingt écrivains tourne désormais sur huit fils, plus près de l'hôte. Son
`MAX_NUM_THREADS=2` reste jouable.
