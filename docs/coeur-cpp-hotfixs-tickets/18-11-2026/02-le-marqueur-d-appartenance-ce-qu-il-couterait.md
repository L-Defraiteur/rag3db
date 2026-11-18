# Le marqueur d'appartenance au graphe : ce qu'il coûterait, ce qu'il ouvrirait

**À lire par Lucie.** Page courte, sans code, demandée par ta décision du 18 novembre sur le
ticket [le nombre de lignes hors du graphe](../tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md) :
« ok (a) mais étudier si (b) est si lourd à faire et ce qu'il pourrait permettre d'autre ».
Point de départ : les trois pages de conception de Codex du 17 novembre (branche
`cx-hotfixs-rows-outside-graph-red-pilot-v2`, `docs/coeur-cpp-hotfixs-tickets/17-11-2026/01` à `03`),
relues contre le code de master.

## Ce que (a) donne déjà

`SHOW_INDEXES` gagne `rows_outside_graph_at_most` : les lignes de la table au-delà de
`numCheckpointedNodes`, celles que chaque recherche balaie une à une
(`searchFromUnCheckpointed`). C'est une **borne** : elle compte aussi, dans cette plage, les
lignes supprimées, celles dont le vecteur est NULL, et celles que le rattrapage d'un CREATE a déjà
reliées au graphe (il s'arrête à la première ligne encore à valider). Une insertion ordinaire
n'y entre pas : son commit la relie au graphe et pousse la borne au-delà d'elle. Un point de
reprise ne la change pas ; la reconstruction de l'index la ramène à 0. Pour le seuil de reconstruction du produit (N* ≈ 13 400 lignes à la dimension 768),
la borne suffit : elle ne se trompe que dans le sens prudent.

## Ce que serait (b)

Un marqueur par ligne : « cette ligne est incorporée au graphe de **cet** index ». La forme que
Codex retient en dernier (sa page 03) : une troisième table de relations interne, de la table
source vers elle-même, un marqueur `o→o` par membre, hors des graphes de recherche. Elle profite de
ce que le moteur fait déjà pour les deux graphes HNSW (MVCC, journal, annulation, point de
reprise). Le compte exact devient alors : lignes visibles à vecteur valide, moins les membres.

## Ce qu'il coûterait

| Morceau | Ce qu'il faut | Estimation |
| --- | --- | --- |
| Tenir le marqueur | Chaque chemin qui change le graphe le tient dans la même transaction : bâti, `commitInsert`, `finalize`, `catchUpCommittedAfter`, `update` (NULL→valide, valide→NULL), `delete_`. « Assurer présent / absent » sans doublon ni erreur de clé, un témoin rouge par chemin. | 3 jours d'agent |
| Format | Version 44, sur le modèle exact de la 43 (l'état de construction du holder) : une base plus ancienne s'ouvre, son premier point de reprise la passe en 44 ; un index d'avant n'a pas de marqueur et rend NULL jusqu'à un nouveau CREATE. Le précédent existe : 40 à 43 sont passés ainsi. | 1 jour |
| Vieux lecteurs après un DROP | Codex relève que `reclaimDroppedTables` rend les pages d'une table supprimée sans prouver qu'aucun vieil instantané ne les lit. C'est **déjà** vrai des deux tables de graphe d'aujourd'hui : soit elles sont protégées et le marqueur l'est de même, soit c'est un défaut actuel qui mérite son ticket, avec ou sans (b). | ½ jour pour trancher |
| Lecture | Une passe sur la table et le marqueur, sous l'instantané, sans verrou ; bornée (NULL au-delà du budget, jamais un compte partiel). | 1 jour |
| Passe et mesures | Coût d'écriture : un marqueur de plus par ligne indexée, à côté de 30 à 60 arêtes déjà écrites (mu = 30, ml = 60) ; à mesurer, pas à supposer. | ½ jour |

**Total : environ 6 jours d'agent**, soit, à ton ratio (1 jour ≈ 1 h 30), **une grosse journée
réelle**, plus la file de push. Ce n'est pas « si lourd ». Codex le voyait plus lourd parce qu'il
traitait le format et la rétention comme des inconnues ; dans ce dépôt, le format a un précédent,
et la rétention est une question qui se pose déjà.

## Ce qu'il ouvrirait

1. **Un invariant vérifiable** : toute ligne vivante à vecteur valide est soit membre, soit dans
   la plage balayée. Aujourd'hui, une ligne perdue par le graphe (les défauts de ce mois en ont
   montré) ne se voit que si un test la cherche. Avec le marqueur, un `CALL` de contrôle la trouve
   sur une vraie base. C'est l'apport le plus utile.
2. **Un CREATE qui reprend** : le bâti d'une grande table, coupé ou fini en `INCOMPLETE`,
   reprendrait aux non-membres au lieu de tout recommencer.
3. **Une reconstruction partielle** : `rebuild_vector_index` n'incorporerait que les non-membres.
   Gain réel mais borné : la reprise aux non-membres équivaut à replier la plage balayée, ce
   qu'un `finalize` sans marqueur saurait déjà faire depuis `numCheckpointedNodes`.
4. **Le compte exact** au lieu de la borne : gain faible, la borne suffit au seuil.

Ce qu'il **n'ouvrirait pas** : il dit « incorporé », pas « atteignable ». Un nœud du graphe que
plus aucune arête ne mène reste membre et reste introuvable par la recherche ; le marqueur ne
remplace pas les témoins de rappel.

## Mon avis

Pas maintenant. (a) répond au besoin du produit. (b) vaut la peine le jour où l'un des deux
premiers points devient un besoin : un contrôle d'intégrité des index sur les bases réelles, ou
des CREATE assez longs pour qu'on veuille les reprendre. D'ici là, le ticket « vieux lecteurs
après un DROP » se tranche seul, car il concerne les graphes d'aujourd'hui.

Décision à toi : (b) maintenant, (b) plus tard sur besoin, ou jamais.
