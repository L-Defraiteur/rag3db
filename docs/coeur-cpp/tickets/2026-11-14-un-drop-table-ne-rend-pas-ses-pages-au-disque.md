# Un DROP TABLE ne rend pas ses pages au disque

- **État** : ouvert (14 novembre 2026, hotfixs) — souhait
- **Famille** : moteur
- **Gravité** : basse. Rien n'est faux, mais de la place disque est gardée pour rien.
- **Atteignable en service** : oui, à chaque table retirée. Le premier cas sera le retrait de l'ancien
  index après 1b, sur chaque base de code.
- **Touche rag3weaver** : par le moteur (la taille des bases sur disque)
- **Signalé par** : hotfixs, sur la répétition du retrait de l'ancien index, avec c7
- **Pour** : cœur c++

## Ce qu'on voit

Sur une copie de `code-rag3db-depot` migrée par 1b (`1b-copie/migree-20261114-1641`), on fait
`CALL DROP_VECTOR_INDEX` sur l'index HNSW de `Scope_Chunk`, puis `DROP TABLE Scope_CHUNKED_FROM`,
`DROP TABLE Scope_Chunk` et `CHECKPOINT`. Ces tables portent 127 195 morceaux, leur texte, leurs vecteurs
de 768 dimensions et 129 400 arêtes.

Le fichier de la base garde ensuite **la même taille au octet près** : 3 962 748 928 octets avant et
après, mesuré deux fois. Les pages des tables retirées, de l'ordre de 1 Go d'après l'ajout de
`ScopeContent`, deviennent des pages libres dans le fichier. La base peut les réutiliser pour ses
écritures suivantes, mais le disque ne les récupère jamais.

Le détail est dans la page de hotfixs `docs/coeur-cpp-hotfixs-tickets/13-11-2026/01-sonde-des-bases-de-lucie.md`,
section « Le retrait de l'ancien index après 1b ».

## Le souhait

Rendre les pages libres au disque. Trois formes possibles, à départager :
- un compactage du fichier : déplacer les pages vivantes vers le début, puis tronquer, comme le
  `VACUUM` de SQLite ou de DuckDB ;
- une réécriture par `EXPORT DATABASE` puis `IMPORT DATABASE` dans un fichier neuf. Elle existe peut-être
  déjà dans le moteur, il faut vérifier qu'elle garde tout ce que rag3weaver pose : index HNSW, colonnes
  BLOB, `_catalog_meta` ;
- au moins, tronquer les pages libres de la fin du fichier au point de reprise.

## Ce qu'il faut pour le fermer

1. Un témoin rouge : créer une table, la remplir d'environ 100 Mo, faire `CHECKPOINT`, `DROP TABLE` puis
   `CHECKPOINT`. La taille du fichier ne baisse pas.
2. La forme choisie, puis le même témoin, vert : le fichier rendu à moins de 10 % de sa taille d'avant
   le remplissage.
