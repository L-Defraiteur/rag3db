# Après un COPY refusé pour clé en double, la ligne d'origine disparaît de l'index de clé primaire

- **État** : corrigé le 4 octobre 2026, commit `05788a868` (« fix(stockage): après un COPY refusé, le point de reprise n'écrit plus hors de son bloc, et la ligne d'origine d'une clé en double reste dans l'index »)
- **Gravité** : réponse fausse, silencieuse, puis durable
- **Atteignable en service** : oui — un seul écrivain
- **Touche rag3weaver** : par un seul chemin, facultatif et éteint par défaut (`RAG3WEAVER_COPY_NAISSANCES=1`, un `COPY` sur une table non vide) ; ses autres `COPY` de nœuds ne visent qu'une table vue vide ou des clés neuves (réponse de la session de l'arbre principal, 4 octobre)
- **Ouvert le** : 4 octobre 2026, session cœur C++
- **Pour** : cœur C++ (stockage, index de clé primaire)

## Ce que c'est

Un `COPY` refusé pour clé en double retire de l'index de clé primaire la clé de la ligne qui était déjà là, si cette clé n'a pas encore passé un point de reprise. La ligne reste dans la table ; la recherche par clé ne la trouve plus, un `MERGE` de cette clé créerait un doublon, et le point de reprise suivant rend la perte durable.

## Recette minimale

Base sur disque ou en mémoire, **aucun point de reprise** entre la création de la ligne et le `COPY` (un index vectoriel créé entre les deux en écrit un, et masque le défaut) :

```cypher
CALL auto_checkpoint=false;
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);
UNWIND range(0, 199) AS i CREATE (:Doc {id: i, name: 'row ' + CAST(i AS STRING)});
COPY Doc FROM 'f.csv';                    -- 200 lignes neuves puis « 5,duplicate » : refusé
MATCH (n:Doc {id: 5}) RETURN count(*);    -- 0, attendu 1
MATCH (n:Doc) RETURN count(*);            -- 200, juste
```

Un balayage `WHERE n.id = 5` retrouve la ligne : c'est l'index qui la perd, pas la table.

## Témoin

`test/transaction/checkpoint_after_failed_copy_test.cpp` : la clé 5 se retrouve après le refus, après un `CHECKPOINT`, après une réouverture, et après le même `COPY` sans le doublon. Rouge déterministe avant le correctif.

## Cause

`RollbackPKDeleter::processScanOutput` (`src/storage/table/node_table.cpp`) relit les lignes annulées et retire de la partie en mémoire de l'index la clé de chacune, sans regarder où cette clé mène. La ligne refusée porte la clé d'une ligne qui reste. Une clé déjà passée par un point de reprise est dans la partie persistante de l'index et n'était pas touchée.

Correctif : une clé n'est retirée que si elle ne mène pas à une ligne d'avant celles que l'on annule (une ligne qui reste précède toujours les lignes annulées). Un contrôle plus strict — « seulement si elle mène dans la plage annulée » — faisait planter le `COPY` qui suit un `COPY` annulé (`copy_tests`, `NodeCopyBMExceptionRecoverySameConnection`) : dans un `COPY` annulé, la clé d'une ligne d'une plage peut mener au-delà de cette plage. Observé, non expliqué.

## Correctif de l'amont

Vela porte le même code. Ladybug a réécrit l'endroit (`index->discardPrimaryKey`) ; non comparé.
