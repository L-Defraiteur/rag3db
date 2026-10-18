# Les lignes validées pendant le bâti d'un CREATE_VECTOR_INDEX manquent à l'index

- **État** : corrigé par les trois commits du lot qui déplace ce ticket (18 octobre 2026), forme
  (δ) arrêtée avec le cœur C++ et l'orchestration ; voir « Ce qui a été fait », à la fin.
- **Gravité** : réponse fausse. Des lignes vivantes ne sont jamais rendues par la recherche,
  jusqu'à la reconstruction suivante de l'index.
- **Atteignable en service** : oui. rag3weaver reconstruit l'index (`ajuster_l_index_pour_le_retard`,
  puis `rebuild_vector_index`) pendant que la montre écrit.
- **Touche rag3weaver** : oui, en principe ; non mesuré dans le produit.
- **Ouvert le** : 12 octobre 2026, session cœur C++ hotfixs, sur un relevé du cœur C++ en relisant
  la rétention (`closed/2026-10-12-un-index-retire-ou-recree-reste-visible-a-tous-dans-le-stockage.md`).
- **Pour** : hotfixs, avec le cœur C++.

## Ce que c'est

`CREATE_VECTOR_INDEX` bâtit l'index à l'instantané de sa transaction, sans verrou sur l'index de la
table. Une autre connexion qui insère, met à jour ou supprime des lignes de la table pendant le
bâti valide sans toucher l'index en construction : il n'est pas encore visible d'elle. À la
validation du CREATE, ces lignes manquent à l'index.

## Témoin

`VectorIndexUpdate.RowsCommittedDuringACreateAreIndexed` (`vector_index_update_test.cpp`), contrôle
`rows-committed-during-create-indexed`. 20 000 vecteurs de dimension 16 ; une connexion fait le
CREATE, une autre insère et valide en boucle pendant ce temps ; une recherche exhaustive
(efs = nombre de lignes) doit atteindre toutes les lignes vivantes. Rouge : environ 1 500 lignes
validées pendant le bâti, 20 006 atteintes sur 21 508 (deux passes, poste principal, 12 octobre).

## Ce qui a été essayé, et pourquoi c'est écarté

Prendre l'index de la table en exclusif dans le CREATE, avant le bâti et jusqu'à sa validation,
et le faire prendre en partagé à toute écriture. Le trou se ferme, mais un blocage apparaît. Le
CREATE force un point de reprise à sa validation, qui attend que toutes les autres transactions
partent. Un écrivain qui attend le verrou partagé derrière l'exclusif du CREATE est actif, et ne
peut pas partir. Le point de reprise expire (5 s), et le CREATE échoue dès qu'un écrivain le
croise. C'est pire que le défaut.

## Formes à peser

Les deux formes viennent de la famille de `CREATE INDEX CONCURRENTLY` de PostgreSQL : un bâti à un
instantané, puis un rattrapage des lignes validées après.

- **(β) Un rattrapage sous verrou exclusif, relâché avant la validation.** Le verrou n'est pris
  que le temps d'insérer dans l'index les lignes validées après l'instantané du bâti, puis il est
  relâché. On reprend ensuite en boucle (« des lignes validées depuis le rattrapage ? ») jusqu'à
  zéro, comme PostgreSQL attend un second instantané. Le piège : le rattrapage sous exclusif ne
  doit pas chevaucher l'attente du point de reprise forcé, sinon le même blocage revient pour
  quelques millisecondes. Il faut que le gestionnaire de verrous rende un verrou seul.
- **(γ′) Le point de reprise forcé ne compte pas une transaction bloquée au verrou en tête
  d'instruction**, sans rien avoir lu ni écrit avec son instantané. Cela demande de prouver que
  `lockIndexForWrite` précède toute lecture de l'instruction, ce qui n'est pas acquis aujourd'hui.

## Un défaut lié, à fermer avec celui-ci

Les états d'index des écrivains (`indexInsertStates`, `indexUpdateState`, `indexDeleteStates`) sont
rangés par position dans `NodeTable::indexes`. Un écrivain d'une table sans index secondaire ne
prend aucun verrou d'index. Un CREATE qui ajoute son index à la liste (`addIndex`, `push_back`), ou
dont l'annulation l'en retire, peut donc changer la liste sous un parcours d'écrivain : une course
de données, des positions décalées. Une assertion (`KU_ASSERT`, en Debug) le vérifie aux parcours.
La parade notée par le cœur : ranger ces états par `holderID`, et non par position. Elle ne suffit
pas seule, puisque le parcours de la liste reste une course avec le `push_back` sans verrou.

## Pour le fermer

Le témoin vert, rejoué 10 fois, sans que `VectorIndexRebuild` ni les témoins de la rétention ne
rougissent, et un CREATE croisé par des écrivains qui réussit sans attendre le délai du point de
reprise.

## Ce qui a été fait (18 octobre 2026)

Aucune des deux formes ci-dessus : (β) gardait un verrou exclusif, (γ′) supposait que le verrou
d'index précède toute lecture de l'instruction, ce qui est faux (le cœur C++ : la partie MATCH
d'un MATCH … SET lit avant ; une table sans index visible ne prend aucun verrou d'index).

**(δ) Le rattrapage à la validation, une fois les autres partis, sans verrou.** C'est l'esprit de
`CREATE INDEX CONCURRENTLY` de PostgreSQL, qui attend les transactions plus vieilles que son
instantané puis rattrape.

- `Transaction::addPreCommitAction` : une action jouée en tête de `Transaction::commit`.
  `TransactionManager::commit` attend le départ des autres avant de l'appeler. Une validation
  forcée (base sur disque) les attendait déjà. La base en mémoire les attend désormais aussi,
  sous le même délai, sans point de reprise : les deux modes ont la même garantie. À ce moment,
  rien ne peut plus changer sous le CREATE, et les versions sont encore en mémoire (aucun point de
  reprise ne passe tant qu'il est ouvert).
- `OnDiskHNSWIndex::catchUpCommittedAfter`, dans une `Transaction::LatestCommittedView`, compare
  pour chaque ligne l'instantané du bâti et le dernier état validé :
  - une ligne apparue entre dans le graphe ;
  - une ligne disparue (supprimée) en sort, avec la réparation de ses voisins ;
  - une ligne au vecteur changé (`NodeTable::wasColumnUpdatedByCommitAfter`) reprend sa place.
  Les chemins sont ceux des écrivains (`updateInternal`, `deleteInternal`, `finalizeDelete`).
- Le rattrapage s'arrête à la première ligne dont l'insertion n'est pas vue : les lignes d'un
  COPY forcé validé après le CREATE dans la file du point de reprise (A7). Elle et les suivantes
  restent lues par le balayage des lignes d'après le graphe (`searchFromUnCheckpointed`), qui saute
  désormais les lignes que le graphe a déjà rendues.
- Le défaut lié : les états d'index des écrivains sont rangés avec le holderID de leur index et
  l'index lui-même (`IndexWriteState`), lus une fois sous `indexesMtx` partagé. Les positions des
  propriétés sont rangées par holderID. L'étape 3 de la validation et `rollbackIndexInsert`
  parcourent la liste sous ce verrou.

**Témoins**, tous rouges sur `ecd0324f3` et verts sur luciepc (trois passes chacun) :
- `RowsCommittedDuringACreateAreIndexed`, sorti de known_red ;
- `ChangesCommittedDuringACreateAreSeen`, sur disque et en mémoire (`VectorIndexCreateInMemory`) :
  insertions, suppressions, SET du vecteur, et une ligne déplacée puis supprimée dans la même
  fenêtre ;
- `AWriterOpenAcrossTheCreateIsCaughtUp` : une transaction ouverte avant le CREATE, validée
  pendant son attente, sans que le CREATE atteigne le délai ;
- `AForcedCopyCommittedRightAfterACreateIsReached` : rouge sur le seul premier commit (1 000
  lignes manquent), vert avec la borne.

**Ce qui reste :**
- Les lignes du COPY de la file restent hors du graphe, au balayage, jusqu'à la reconstruction de
  l'index. La recherche est juste, mais elle balaie davantage. C'était déjà le cas sur master pour
  toute ligne d'après le graphe.
- Le rattrapage tourne pendant que les nouveaux BEGIN attendent : un balayage de visibilité de la
  table, plus une opération HNSW par ligne changée. Le point de reprise qui suit les retient de
  toute façon.

