# Un point de reprise efface les relations des régions non touchées

- **État** : ouvert
- **Gravité** : perte
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (il supprime des relations et laisse passer des points de reprise)
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : cœur C++ (stockage)

## Ce que c'est

Après la suppression de toutes les relations d'un bloc de 1 024 nœuds, le point de reprise suivant efface aussi les relations de tous les autres blocs du même groupe de 131 072 nœuds, dans ce sens-là. Le sens inverse les garde ; la perte reste après réouverture.

## Recette minimale

```cypher
CALL auto_checkpoint=false;
CREATE NODE TABLE person(id INT64 PRIMARY KEY);
CREATE REL TABLE knows(FROM person TO person, MANY_MANY);
UNWIND range(0, 1024) AS id CREATE (:person {id: id});
MATCH (a:person), (b:person) WHERE a.id IN [0, 1024] AND b.id = 1 CREATE (a)-[:knows]->(b);
CHECKPOINT;
MATCH (a:person)-[r:knows]->(:person) WHERE a.id = 0 DELETE r;
CHECKPOINT;
MATCH (a:person {id: 1024})-[r:knows]->() RETURN count(r);   -- attendu 1, obtenu 0
MATCH (:person {id: 1})<-[r:knows]-() RETURN count(r);       -- sens inverse : 1
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — RelationsOfAnUntouchedRegionSurviveACheckpoint

## Cause

`src/storage/table/csr_node_group.cpp:543-551` : le compte des relations restantes ne parcourt que les régions modifiées ; s'il vaut 0, tout le groupe persistant est libéré.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `0be6fa597` (13 mars 2026) ; Vela `e5e700e73`. Défaut d'origine Kuzu `eaf97b755`.

## Pour le fermer

compter les relations sur toutes les régions du groupe ; le témoin passe au vert. Condition pour une base existante : une table de nœuds de plus de 1 024 lignes, et toutes les relations d'un bloc de 1 024 supprimées entre deux points de reprise ; signature : les deux sens ne s'accordent plus.
