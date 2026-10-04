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

### Mise à jour, 4 octobre vers 13 h (arbre principal)

- **Étalonnage, sur le moteur d'avant `80e3f2c32`** : la sonde voit la
  recette, `knows` (2, 2) → (1, 0) après le second CHECKPOINT et après
  réouverture. L'étalon lié par clé dit « direct depuis 1024 : 0, inverse
  vers 1 : 1 » : la perte est dans le sens direct, et la sonde la range de
  l'autre côté. **La sonde détecte la dissymétrie de façon sûre ; son
  étiquette de sens ne l'est pas.** La seconde version lie l'extrémité par un
  `WITH` avant d'étendre, mais le planificateur reste libre.
- **Sur le moteur corrigé** (`build/lecteurs-csv` rebâti sur `38c82360b`) :
  la recette est verte, (2, 2) → (1, 1), et l'étalon rend « 1, 1 ».
- **Le test du produit ne prouvait rien** : les deux scopes qui importent
  étaient aux décalages 17 et 210, dans la **même** région, bien que la
  source compte 1 102 scopes. L'ordre de création ne suit pas l'ordre des
  fichiers. Son vert ne dit donc pas que nos chemins étaient à l'abri. À la
  lecture, ils peuvent remplir la condition : une édition retire les arêtes
  d'un fichier par `DELETE r` sans en reposer si l'import a disparu ; une fin
  de synchronisation retire des scopes. Il suffit que ce soient les seules
  arêtes de leur région pour cette table.
- **Une base existante a pu perdre des arêtes** si elle a été écrite, éditée
  ou resynchronisée sur un moteur d'avant `80e3f2c32`, avec des points de
  reprise (toute base sur disque). Pour le contrôler, **sur une copie**,
  jamais sur une base qu'un backend tient :

  ```
  cp --reflink=always <base> <copie>   # avec <base>.wal, .shadow, .extensions s'ils existent
  cargo run --release --features rag3db-native,code --example sens_des_relations -- \
      <copie> <arbre>/extension/vector/build/libvector.rag3db_extension
  ```

  Sortie en 1 si une table de relation n'a pas le même compte dans les deux
  sens : la base a perdu des arêtes, et elle se réindexe de zéro.
