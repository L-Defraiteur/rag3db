# Un point de reprise efface les relations des régions non touchées

- **État** : corrigé le 4 octobre 2026, commit « fix(stockage): un point de reprise ne libère plus les relations des régions qu'il n'a pas réécrites »
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

## Côté rag3weaver (arbre principal, 4 octobre)

- **La sonde** : `rag3weaver::relation_directions::count_both_directions(conn)`
  compte, par table de relation et par couple (départ, arrivée), les
  arêtes lues dans les deux formes `MATCH (a:From)-[r:T]->(b:To)` et
  `MATCH (b:To)<-[r:T]-(a:From)`. L'exemple `sens_des_relations <base>
  [<extension>]` l'applique à une copie de base, et sort en 1 si un couple
  diffère. Elle sert à dire si une base existante a perdu des arêtes.
- **Elle voit la recette** (`tests/sonde_sens_des_relations.rs`,
  `la_recette_du_banc_garde_les_deux_sens_egaux`, hors batterie, rouge
  aujourd'hui) : `knows` (2, 2) → (1, 0) après le second CHECKPOINT, et
  encore (1, 0) après réouverture. **Mais le 0 tombe sur la seconde forme**
  (`<-`), alors que la perte est dans le sens direct. Le planificateur
  choisit le sens de parcours : la sonde détecte la dissymétrie, sans dire
  de façon sûre quel sens est touché. Question ouverte pour le cœur C++ :
  quelle forme Cypher force chaque sens de stockage ?
- **Nos chemins du produit ne l'ont pas déclenché**
  (`un_fichier_retire_ne_fait_pas_perdre_les_aretes_des_autres_regions`).
  Le test prend 1 102 scopes (deux régions), dont seuls le premier et le
  dernier fichier portent un `USES_LIBRARY`. On retire celui du premier,
  puis CHECKPOINT :
  - par resynchronisation (le fichier part, ses scopes par DETACH DELETE) :
    (2, 2) → (1, 1) ;
  - par édition (`MESURE_GESTE=edition` : `reingest_file`, `DELETE r`, puis
    réingestion sans l'import) : (2, 2) → (1, 1).

  Pourquoi la condition n'est pas remplie n'est pas expliqué. Hypothèses :
  la position réelle des nœuds Scope (COPY puis MERGE), d'autres régions
  modifiées dans la même table entre les deux points de reprise, ou le sens
  compté. Ce vert ne prouve donc pas que rag3weaver est à l'abri.
