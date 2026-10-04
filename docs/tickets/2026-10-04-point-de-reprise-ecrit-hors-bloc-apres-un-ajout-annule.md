# Le point de reprise écrit hors de son bloc après un ajout annulé

- **État** : corrigé le 4 octobre 2026, commit `05788a868` (« fix(stockage): après un COPY refusé, le point de reprise n'écrit plus hors de son bloc, et la ligne d'origine d'une clé en double reste dans l'index »)
- **Gravité** : plantage (corruption de tas)
- **Atteignable en service** : oui — un seul écrivain, un `COPY` refusé suffit
- **Touche rag3weaver** : oui s'il lui arrive qu'un `COPY` soit refusé ou annulé sur une table dont les dernières lignes n'ont pas passé de point de reprise
- **Ouvert le** : 4 octobre 2026, session cœur C++ (à partir de l'essai déterministe de la session du banc)
- **Pour** : cœur C++ (stockage)

## Ce que c'est

Après l'annulation d'un ajout dans un groupe de nœuds encore en mémoire, le point de reprise suivant recopie les lignes annulées dans un bloc dimensionné pour les seules lignes valides : il écrit au-delà du bloc.

## Recette minimale

Base sur disque, aucun point de reprise entre les étapes :

```cypher
CALL auto_checkpoint=false;
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);
UNWIND range(0, 199) AS i CREATE (:Doc {id: i, name: 'row ' + CAST(i AS STRING)});
COPY Doc FROM 'f.csv';   -- 200 lignes neuves puis « 5,duplicate » : refusé, clé en double
CHECKPOINT;              -- écrit hors bloc
```

N'importe quel point de reprise fait l'affaire : celui d'un `COPY` suivant, de `CREATE_VECTOR_INDEX`, du seuil, de la fermeture.

## Témoin

`test/transaction/checkpoint_after_failed_copy_test.cpp` (sans vecteur) et `test/transaction/vector_index_after_failed_copy_test.cpp` (le scénario du banc : `COPY` refusé puis `CREATE_VECTOR_INDEX`). Déterministes sous AddressSanitizer ; en Release le second mourait par « malloc(): mismatching next->prev_size », mais une corruption de tas ne tue pas à coup sûr.

## Cause

`ChunkedNodeGroup::rollbackInsert` (`src/storage/table/chunked_node_group.cpp`) ramenait le compte de lignes du bloc, pas ses colonnes. `NodeGroup::checkpointInMemOnly` dimensionne son bloc de sortie par le compte et y recopie tout ce que portent les colonnes (`ColumnChunk::scanCommitted` s'appuie sur le nombre de valeurs de la colonne). AddressSanitizer : heap-buffer-overflow, écriture de 50 octets dans un bloc de 32, `NullMask::copyNullMask` ← `NullChunkData::append` ← `ChunkedNodeGroup::scanCommitted` ← `NodeGroup::checkpointInMemOnly`.

Correctif : les colonnes sont tronquées avec le compte.

## Correctif de l'amont

Aucun : Vela (`vela/master`, 14 juin 2026) et Ladybug (`ladybug-main-2026-08-31`) portent le même code.

## Ce que ce ticket ne dit pas

Si c'est la corruption qui tue `e2e_code` (ticket « Une corruption de mémoire tue e2e_code ») : c'est un candidat sérieux — un ajout annulé puis un point de reprise —, non vérifié.
