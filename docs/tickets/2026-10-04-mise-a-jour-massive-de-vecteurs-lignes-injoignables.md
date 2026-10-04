# Après une mise à jour massive de vecteurs, des lignes restent injoignables dans l'index

- **État** : ouvert
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : peu. Son chemin principal, des lignes créées sans vecteur puis remplies, est vert ; seul le réembarquement d'une ligne gardée passe par ce défaut, et rag3weaver repasse par NULL ligne à ligne, ce qui est vert
- **Ouvert le** : 4 octobre 2026, session du banc (le défaut est au journal des chantiers §6 depuis le 3 octobre)
- **Pour** : cœur C++ (extension vector)

## Ce que c'est

Quand on remplace les vecteurs de beaucoup de lignes d'une table indexée, une recherche
exhaustive ne rend plus toutes les lignes, ou une ligne n'est plus rendue première quand on
cherche son propre vecteur. Le compte des pertes change d'une passe à l'autre (tirage des
niveaux du graphe). Depuis `c8fdaf196` (les anciens voisins restent joignables), environ un
essai sur deux perd encore des lignes.

## Recette minimale

```cypher
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);
UNWIND range(0, 999) AS i CREATE (n:Doc {id: i}) SET n.vec = [i % 17, i % 23, i % 29, i];
CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');
MATCH (n:Doc) WHERE n.id >= 0 AND n.id < 512 SET n.vec = [n.id % 13 + 0.5, n.id % 19, n.id % 31, n.id + 0.25];
MATCH (n:Doc) WHERE n.id >= 512 AND n.id < 1000 SET n.vec = [n.id % 13 + 0.5, n.id % 19, n.id % 31, n.id + 0.25];
-- recherche exhaustive (k = efs = 1000) depuis le vecteur d'une ligne : moins de 1 000 lignes
-- rendues, ou des lignes qui ne sortent pas premières sur leur propre vecteur, un essai sur deux
```

## Témoin

`test/transaction/concurrence/vector_index_update_test.cpp` (étiquette `vector-index-exact`),
chaque cas répété sur des tables neuves :
- `SetToAnotherVectorInBatchesOf512`, `TwentyRowsToDistinctVectorsLineByLine`,
  `OneRowUpdatedManyTimes`, `UpdateThenDelete` : rouges connus, cas longs
  (`CONCURRENCE_LONG=1`) ;
- `SetToAnotherVectorLineByLine`, `TenThousandRowsInBatchesOf512` : probabilistes.

## Cause

D'après la session cœur C++ : l'élagage des voisins retire des arêtes entrantes à des nœuds
que personne ne recontrôle. Il faudrait un passage en fin d'instruction, comme le
`finalize` de la suppression. L'état de mise à jour de l'index est aujourd'hui recréé à
chaque ligne.

## Correctif de l'amont (à lire, ne pas copier)

Aucun : Ladybug n'a ni mise à jour ni suppression dans son index HNSW.

## Pour le fermer

Les témoins passent au vert à chaque essai. Contournements sûrs en attendant : supprimer
puis réinsérer la ligne ; en masse, retirer l'index, poser les vecteurs, le recréer.
