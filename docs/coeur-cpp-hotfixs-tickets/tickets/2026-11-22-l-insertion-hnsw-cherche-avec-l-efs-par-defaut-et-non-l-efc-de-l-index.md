# L'insertion HNSW cherche avec l'efs par défaut, et non avec l'efc de l'index

- **État** : ouvert (22 novembre 2026, hotfixs). Correctif écrit sur la branche `hnsw-insert-uses-efc` (b82423516b),
  **non livré faute de témoin rouge réel** (voir « Le témoin »).
- **Famille** : moteur
- **Gravité** : confort. Un paramètre de l'index ne fait pas ce qu'il dit : l'efc ne règle que le CREATE. Rien n'est
  perdu, et le rappel est le même (0,997 à 0,9985 sur code-lourd, avec ou sans).
- **Atteignable en service** : oui, pour tout index créé avec un efc autre que 200.
- **Touche rag3weaver** : non, rag3weaver crée ses index à 200, qui est aussi l'efs par défaut.
- **Signalé par** : hotfixs, la mesure de l'insertion pour ED
  (`docs/coeur-cpp-hotfixs-tickets/22-11-2026/01-le-cout-d-un-set-de-vecteur-sous-hnsw.md`)
- **Pour** : hotfixs

## Le constat

Les quatre états d'écriture de l'index (`initInsertState`, `finalize`, `initUpdateState`, `initDeleteState`)
construisent un `HNSWInsertState`, dont la recherche prend `QueryHNSWConfig{}` (`hnsw_index.cpp:613` et `:626`),
c'est-à-dire l'efs de requête par défaut, 200. L'`efc` de l'index ne sert qu'au CREATE.

## Le correctif écrit

`b82423516b` (branche `hnsw-insert-uses-efc`) : les quatre états reçoivent `config.efc`, et `HNSWInsertState` en fait
l'`efs` de sa recherche (`ef = max(ml, efc)`). Il compile, et la passe de tête n'a pas été jouée.

## Le témoin : trois essais, aucun ne départage

1. **2 000 points gaussiens en dimension 32, efc 4 000**, les arêtes d'un nœud neuf contre ses 60 plus proches
   exacts : vert **sans** correctif. L'ef de 200 suffit déjà.
2. **20 000 points en dimension 64, efc 30 000**, jugés après les dix insertions : rouge avec **et** sans correctif.
   Les arêtes inverses des insertions suivantes font élaguer le nœud (`createRels`, au-delà de ml), et les distances en
   double ratent des voisins à la frontière.
3. **Le même jeu, chaque ligne jugée juste après son insertion, contre les plus proches rendus par le moteur**
   (`QUERY_VECTOR_INDEX` à efs 40 000) : encore rouge avec **et** sans correctif (54 à 60 sur 60). Le graphe est
   orienté, et pas fortement connexe. L'insertion entre dans la couche basse par un nœud ; la requête de contrôle,
   jouée après, peut entrer par un autre, par exemple le nœud neuf s'il a été tiré dans la couche haute. Les deux
   recherches « exhaustives » ne couvrent donc pas le même ensemble.

Un témoin qui lise l'ef effectivement utilisé demande de le rendre observable : un compteur de l'index, ou une
colonne de `SHOW_INDEXES`. C'est un choix de conception, hors du lot.

## Ce qui le fermerait

Un témoin qui lise l'ef de l'insertion (l'observable ci-dessus), rouge sur master, vert avec `b82423516b` ; puis la
mesure avant/après sous `poste mesure` : le temps d'un SET seul et d'un lot de 64, et le rappel@10, à efc 50, 200 et
400. Les deux mesures prévues le 22 novembre n'ont pas tourné : `poste mesure` les a refusées 24 fois (code 76), de
13 h 09 à 16 h 07.
