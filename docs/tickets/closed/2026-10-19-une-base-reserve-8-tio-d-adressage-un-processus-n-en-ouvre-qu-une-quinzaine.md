# Une base réserve 8 Tio d'adressage : un processus n'en ouvre qu'une quinzaine

- **État** : corrigé par le commit qui déplace ce ticket (19 octobre 2026, seconde session cœur
  C++, forme relue par le cœur C++), par le remède 1 ; voir « Ce qui a été fait », à la fin.
- **Gravité** : une ouverture de base refusée en service, au hasard de l'ordre d'ouverture, dès
  qu'un processus tient une quinzaine de bases sur disque.
- **Atteignable en service** : oui. C'est l'hôte d'équipe de la vision (un index par dépôt),
  plus les tâches de l'agent et leurs copies de base (page 09 de everything declarative), plus la
  mémoire longue.
- **Touche rag3weaver** : oui (`rag3db_connection.rs`, la configuration d'ouverture) ; le remède
  est dans le moteur.
- **Pour** : hotfixs, dans sa file après la rétention pour tous les index (c7).

## Ce qu'on voit

Le moteur `mmap`e d'un bloc la région `max_db_size` de chaque base (`MAP_NORESERVE` : de
l'espace d'adressage, pas de la mémoire), et place ses pages à des adresses fixes dedans. Par
défaut, c'est 8 Tio par base sur disque. Un processus Linux x86-64 adresse 128 Tio en espace
utilisateur : la seizième base ouverte en même temps échoue.

```
ConnectionError("Buffer manager exception: Mmap for size 8796093022208 failed.")
```

Le 19 octobre, à luciepc, dans la lib entière : deux témoins de `copie_liens_tests` rouges sur ce
message, au moment où deux témoins de tâches (`daemon_host::taches::tests`) ouvraient quatre bases
de plus (une base principale et sa copie, deux fois). Les bases en mémoire avaient déjà rencontré
la même borne le 25 août (`IN_MEMORY_MAX_DB_SIZE`, 1 Tio, `rag3db_connection.rs`).

## La recette

Un processus qui ouvre seize bases sur disque, distinctes, sans `RAG3DB_MAX_DB_SIZE` :
`Rag3dbConnection::open` dans une boucle, en les gardant ouvertes. La seizième ou la dix-septième
échoue sur le message ci-dessus (l'adressage déjà pris par le reste du processus avance la borne).
Avec `RAG3DB_MAX_DB_SIZE=1099511627776` (1 Tio), cent bases s'ouvrent.

## Ce qui est fait

- Les témoins de la lib : sous `cfg!(test)`, sans `RAG3DB_MAX_DB_SIZE`, une base sur disque
  réserve 1 Tio (`bc2cb48e7`). Les e2e lient la lib compilée hors test et réservent toujours
  8 Tio (l'arbre principal, 19 octobre) : une suite e2e qui ouvrirait des dizaines de bases
  retomberait sur la borne.
- En service, seule issue aujourd'hui : `RAG3DB_MAX_DB_SIZE`, pour tout le processus. Une base
  plus grosse que la valeur posée échouerait alors à grandir.

## Le remède attendu (le moteur)

L'une des deux, ou les deux :

1. **Une réservation qui grandit à la demande** : une région de départ modeste, puis des régions
   de plus en sus quand la base grossit, au lieu d'une seule de 8 Tio d'emblée. C'est la forme
   générale ; elle touche le placement des pages à adresses fixes.
2. **Une taille par base, dite dans le manifeste** (`max_db_size`), lue par l'ouverture de
   rag3weaver comme l'est déjà `buffer_pool`. C'est petit, et le produit choisit par base (une
   copie de tâche, un index de dépôt).

## Le témoin attendu

Un processus ouvre trente-deux bases sur disque sans `RAG3DB_MAX_DB_SIZE`, et toutes s'ouvrent.
Et une base qui grossit au-delà de sa première réservation continue de s'écrire (pour le
remède 1).

## Ce qui a été fait (19 octobre 2026)

**Le remède 1, la réservation qui grandit.** `VMRegion` ne réserve plus sa taille maximale d'un
bloc. Elle réserve des tranches de 16 Gio, à mesure que les groupes de cadres s'ajoutent
(`addNewFrameGroup`, sous son verrou, avant de rendre l'indice) :
- une tranche est une puissance de deux et un multiple d'un groupe de cadres, donc un groupe ne
  chevauche jamais deux tranches. L'adresse d'un cadre est `chunks[idx >> shift] + (idx & mask)
  × frameSize` ;
- la table des tranches, de taille fixe, est écrite en release après la réservation réussie, et
  lue en acquire sans verrou, par `getFrame` (le chemin chaud) comme par `contains` et
  `releaseFrame` ;
- la taille maximale d'une base ne change pas : le nombre de tranches est borné, et la tranche
  grandit si la région en demande plus de 1 024 ;
- la région temporaire prend la même classe, avec une tranche bornée à sa taille. Sous
  Windows, chaque tranche est un `VirtualAlloc MEM_RESERVE`, et le destructeur libère chacune ;
- une réservation se refuse désormais en cours de route, et non plus à l'ouverture. Le refus est
  une erreur nommée (« Cannot reserve N GiB more of address space for this database »), traitée
  comme un refus du tampon. Au point de reprise, il ne condamne la base qu'après le premier
  changement de la mémoire.

Une petite base réserve 16 Gio au lieu de 8 Tio, une base de 50 Gio en réserve 64. Un processus
en ouvre des milliers au lieu d'une quinzaine.

**Témoins** (`test/storage/buffer_manager_test.cpp`, sur luciepc) :
- `VMRegionTest.ThirtyTwoDatabasesOpenInOneProcess` : rouge 2/2 sur fe700661e (« base 15 : Mmap
  for size 8796093022208 failed »), vert 2/2 avec le lot ;
- `VMRegionChunks.FramesBeyondSeveralChunksAreWritable` : une région à petites tranches, des
  cadres écrits et relus au-delà de plusieurs tranches, `contains` juste aux bords, et le refus
  au-delà de la taille maximale ;
- `VMRegionChunks.ARefusedChunkIsANamedError` : une tranche refusée par le système donne l'erreur
  nommée, et les cadres déjà réservés restent bons.

`buffer_manager_test` est vert en entier (6 tests).

**Le remède 2**, `max_db_size` par base au manifeste, n'est plus nécessaire pour ouvrir. Il garde
un sens pour borner la taille d'une base : c'est au produit (everything declarative) de le
vouloir ou non. Le contournement de la lib sous `cfg(test)` (1 Tio, `bc2cb48e7`) peut partir.

