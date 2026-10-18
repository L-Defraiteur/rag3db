# Une base ne dit pas quel moteur l'a écrite en dernier, et un moteur plus ancien de même format l'ouvre en écriture

- **État** : ouvert (demande de Lucie par l'orchestration, 18 octobre 2026 ; fin de la file du
  cœur C++)
- **Gravité** : corruption possible : un vieux client démarre un vieil hôte sur une base écrite
  par un moteur plus récent de même « version de stockage », l'ouvre en écriture et y écrit avec
  un code qui ne connaît pas ce que le récent y a mis (un champ d'en-tête, une colonne
  d'index, un enregistrement du journal). Rien ne le refuse aujourd'hui tant que le numéro de
  version de stockage est le même.
- **Atteignable en service** : oui, par un client et un hôte de versions différentes sur le
  même poste (G règle le chemin par l'activation par socket ; il faut aussi le filet du moteur).
- **Touche rag3weaver** : oui (l'hôte est le moteur).
- **Ouvert le** : 18 octobre 2026, session cœur C++.
- **Pour** : cœur C++, fin de file (après l'ADD paresseux).

## Ce que l'en-tête de base contient déjà

`DatabaseHeader` (`src/storage/database_header.cpp`), lu par `Checkpointer::readCheckpoint`
quand le fichier de données a des pages :
- les octets magiques `RAG3DB` ;
- la **version de stockage** (`storage_version`, un entier) : `StorageVersionInfo` la tire d'une
  table « version du produit → numéro » (`0.11.2.2 → 40`, `0.11.1 → 39`, …) ; le numéro ne
  bouge donc qu'avec la **chaîne de version du produit** (`RAG3DB_CMAKE_VERSION`), pas à chaque
  vrai changement de format ;
- l'étendue des pages du catalogue (`catalogPageRange`) et, depuis la version 40, l'étendue du
  fichier de données au point de reprise (`numDataPages`) ;
- rien sur le moteur qui a écrit : ni identifiant de bâti, ni date, ni hash.

Le contrôle (`validateStorageVersion`) : la version lue doit être **égale** à celle du moteur, ou
valoir 39 (lue par un moteur 40, qui la passe en 40 à son premier point de reprise) ; sinon
« Trying to read a database file with a different version. Database file version: N, Current
build storage version: M », dans les deux sens (base plus récente comme base plus ancienne), en
lecture comme en écriture. C'est l'esprit du `catalog version` de PostgreSQL, mais avec deux
trous.

L'identifiant de bâti existe à part : la cible `rag3db_build_id` et la fonction de table
`build_id()` (`src/function/table/build_id.cpp`) le rendent ; le témoin
`extension_build_id_test` s'en sert pour que l'extension et la lib soient du même bâti. Il n'est
écrit nulle part dans la base.

## Ce qui manque

1. **Une version de format qui ne bouge qu'à un vrai changement**, indépendante de la chaîne de
   version du produit : depuis la bifurcation, les lots qui changent ce qui est écrit (E10 n'a
   rien changé au format ; l'étendue du fichier a eu droit à 39 → 40 ; un prochain changement
   d'en-tête d'index, de journal ou de colonne ne passerait pas la chaîne `0.11.2.2`) doivent
   incrémenter un `STORAGE_FORMAT_VERSION` posé à la main, avec la note de ce qu'il change,
   comme `CATALOG_VERSION_NO` de PostgreSQL. La table « produit → numéro » reste pour lire les
   anciennes bases.
2. **L'identifiant du dernier moteur écrivain** dans l'en-tête (`build_id` du moteur, et la
   version de format qu'il écrit), posé à chaque point de reprise. Il ne sert pas au refus (deux
   bâtis différents de même format sont compatibles) mais au message, et au diagnostic.
3. **Le refus en écriture, nommé** : un moteur dont la version de format est plus ancienne que
   celle de la base refuse de l'ouvrir en écriture, avec les deux versions et les deux bâtis dans
   le message (« cette base a été écrite par le moteur ‹hash, date› au format 41 ; ce moteur
   ‹hash, date› écrit le format 40 : ouvrez-la avec un moteur au moins aussi récent »). La
   lecture seule d'une base plus récente reste refusée aussi (le format n'est pas lisible), mais
   par le même message. Une base plus ancienne s'ouvre et se migre au point de reprise, comme
   39 → 40 aujourd'hui, quand la migration existe ; sinon refus nommé.
4. **Le journal** : à vérifier que l'en-tête du journal (`WAL`) porte la même version de format,
   et que le rejeu d'un journal écrit par un moteur plus récent est refusé par le même nom
   avant de rejouer quoi que ce soit (aujourd'hui le rejeu lit des enregistrements typés sans
   version : un type inconnu est une exception brute).

## Témoin

`test/transaction/` : une base écrite avec un en-tête forgé à la version de format +1 et un
bâti « autre » ; l'ouverture en écriture et en lecture rend le message avec les deux versions et
les deux bâtis ; une base à la version −1 (quand une migration existe) s'ouvre, et son premier
point de reprise la passe à la version courante avec le bâti courant. Le même pour un journal
forgé.

## Pour le fermer

Les quatre points, le témoin vert, et la note dans la page de G sur le démarrage de l'hôte (le
filet du moteur s'ajoute à l'activation par socket).
