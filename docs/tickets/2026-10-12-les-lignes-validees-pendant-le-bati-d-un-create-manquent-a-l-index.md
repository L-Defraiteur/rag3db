# Les lignes validées pendant le bâti d'un CREATE_VECTOR_INDEX manquent à l'index

- **État** : ouvert (deux formes à peser, ci-dessous ; aucune tranchée)
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
