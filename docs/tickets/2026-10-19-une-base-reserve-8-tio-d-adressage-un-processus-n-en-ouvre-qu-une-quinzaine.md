# Une base réserve 8 Tio d'adressage : un processus n'en ouvre qu'une quinzaine

- **État** : ouvert — vu le 19 octobre 2026 (everything declarative), contourné dans les témoins
  de la lib seulement (`bc2cb48e7`).
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
