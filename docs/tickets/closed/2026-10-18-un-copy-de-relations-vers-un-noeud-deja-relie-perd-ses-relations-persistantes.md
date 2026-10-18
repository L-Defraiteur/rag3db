# Un COPY de relations vers un nœud qui a déjà des relations persistantes les fait disparaître

- **État** : corrigé le 18 octobre 2026 par le commit qui ferme ce ticket (cœur C++). La cause :
  l'annulation retire les groupes de queue « vides » de la collection
  (`NodeGroupCollection::rollbackInsert` → `GroupCollection::getNumEmptyTrailingGroups`), et le
  vide se jugeait sur `getNumRows()`, les lignes en mémoire seules ; un groupe CSR dont toutes les
  relations sont dans son bloc persistant comptait 0 et partait avec elles, le COPY suivant
  repartait d'un groupe neuf, et le point de reprise rendait la perte durable. Le vide se juge
  par `isEmpty()`, que `CSRNodeGroup` définit avec son bloc persistant. Témoin
  `test/transaction/copy_rollback_persistent_rels_test.cpp` (COPY annulé seul, puis COPY validé,
  depuis une sous-requête, sans propriété, puis CREATE ; relus après `CHECKPOINT` et réouverture).
- **Gravité** : **perte de données, durable** : après un COPY annulé (puis un COPY validé), un `CHECKPOINT` puis une
  réouverture, les dix relations d'avant du nœud ne sont plus nulle part (`k <= 10` compte 0 dans
  toute la table, le nœud 1 n'a plus d'entrante) : perdues au stockage, pas seulement au
  balayage (sonde du 18 octobre, luciepc, master b47dc6066).
- **Atteignable en service** : oui, dès qu'un COPY de relations s'ajoute à une table qui a déjà
  des relations validées et reprises (point de reprise) pour les mêmes nœuds source.
- **Touche rag3weaver** : seulement si un COPY de relations s'annule (échec en cours de route, ou
  `BEGIN … ROLLBACK` autour) vers des nœuds déjà reliés et repris ; la perte mesurée par
  codeparsers sur `code-lourd` n'est pas ce défaut (voir plus bas).
  **Éprouvé par le chemin du produit** (arbre principal, 18 octobre) : le produit écrit bien ses
  liens par COPY dès 200 arêtes d'un groupe, depuis des nœuds déjà liés (`LinkRecordNode`,
  `copier_les_liens`, un CSV sans propriété). Sur dix liens, un CHECKPOINT, puis trois cents de
  plus depuis la même source ou vers la même arrivée, en transaction automatique, sous COMMIT
  **et sous ROLLBACK**, le compte est juste (`dataflow::copie_liens_tests`, gardés comme garde).
  La perte de code-lourd était un défaut du produit, fermé :
  [les relations d'un index dépendaient du chemin d'écriture](closed/2026-10-18-les-relations-d-un-index-dependent-du-chemin-d-ecriture.md).
- **Ouvert le** : 18 octobre 2026, session cœur C++, trouvé en écrivant le témoin du rollback
  sous lecteurs.
- **Pour** : cœur C++ (à placer par l'orchestration ; c'est une perte, donc avant les lenteurs).

## Recette (jouée, déterministe)

```cypher
CREATE NODE TABLE A(id INT64 PRIMARY KEY);
CREATE REL TABLE R(FROM A TO A, k INT64);
UNWIND range(0, 3) AS i CREATE (:A {id: i});
MATCH (a:A {id: 0}), (b:A {id: 1}) UNWIND range(1, 10) AS i CREATE (a)-[:R {k: i}]->(b);
CHECKPOINT;                                   -- les 10 relations du nœud 0 sont persistantes
-- fichier csv : 1 500 lignes « 0,2,k »
BEGIN TRANSACTION;
COPY R FROM 'fichier.csv' (header=false);
-- pendant le COPY : l'écrivain compte 1 510, un lecteur compte 10 (juste)
ROLLBACK;
MATCH (a:A {id: 0})-[r:R]->() RETURN count(r);  -- attendu 10, rendu 0
COPY R FROM 'fichier.csv' (header=false);
MATCH (a:A {id: 0})-[r:R]->() RETURN count(r);  -- attendu 1 510, rendu 1 500
```

Les dix relations persistantes du nœud 0 ne sont plus rendues dès qu'un COPY a touché le nœud,
annulé ou validé. Suite de la sonde, après le COPY validé : toute la table compte 1 500,
`WHERE r.k <= 10` compte 0 ; après `CHECKPOINT` : 1 500 et 0 ; après fermeture et réouverture :
1 500, 0, et le nœud 1 n'a plus aucune relation entrante. **La perte est durable.** Sans le `CHECKPOINT` (les dix relations en mémoire), tout est juste : le
défaut tient à la rencontre d'un bloc persistant et d'une liste en mémoire pour le même nœud.

## La forme exacte (sonde de variantes, luciepc, master, 18 octobre 2026, une base neuve par cas)

| cas | après l'annulation (attendu 10) | à la fin, nœud 0 | vers le nœud 1 (attendu 10) |
|---|---|---|---|
| COPY validé seul, relations d'avant persistantes | — | 1 510 (juste) | 10 |
| **COPY annulé seul**, persistantes | **0** | 0 | 0 |
| COPY annulé puis COPY validé, persistantes | 0 | 1 500 | 0 |
| COPY annulé **sans propriété**, persistantes | 0 | 1 500 | 0 |
| COPY **depuis une sous-requête** annulé, persistantes | 0 | 1 500 | 0 |
| COPY annulé puis **CREATE** validé, persistantes | 0 | 1 500 | 0 |
| CREATE annulé puis COPY validé, persistantes | 10 | 1 510 (juste) | 10 |
| COPY annulé puis COPY validé, relations d'avant **en mémoire** | 10 | **3 010** (les 1 500 annulées reviennent) | 10 |

Donc : **c'est l'annulation d'un COPY** (fichier ou sous-requête, avec ou sans propriété) vers un
nœud qui a des relations **persistantes** qui les efface, tout de suite (0) et pour toujours. Un
COPY validé seul est juste ; un CREATE annulé ne fait rien (stockage local). Un COPY qui échoue
en cours de route (clé en double, mémoire) s'annule : il suffit donc d'un COPY refusé pour
perdre les relations persistantes des nœuds qu'il touchait. La dernière ligne (relations d'avant
en mémoire : les 1 500 lignes annulées reviennent au COPY suivant) est le défaut des listes CSR
qui gardent les lignes annulées, corrigé par le lot « rollback sous lecteurs » (58c1110ca).

Ce que le produit en touche : son chemin CSV est en validation automatique, sans ROLLBACK
explicite ; il n'est atteint que si un COPY échoue (annulation), ou s'il enveloppe un COPY dans
un `BEGIN … ROLLBACK`.

## Sur une vraie base (codeparsers, 18 octobre 2026) — ce n'est pas ce défaut

Mesure de codeparsers, même lib 87a362e30, base `code-lourd` : réindexée au seuil haut sans COPY,
contre au seuil par défaut avec COPY. Par rapport à la référence sans COPY : `HAS_PARENT` −18,
`INHERITS_FROM` −11, `CONSUMES` +17, et 16 relations égales ; la base vive est identique au cas
avec COPY. Les arêtes perdues sont des relations fines (héritage, parent), et un `CONSUMES` de
trop apparaît sur le même couple de nœuds. Listes :
`~/.cache/codeparsers-notes/pertes-18oct/{ref,bas}-lourd-aretes.txt` sur le portable.
**L'arbre principal a montré que cette perte-là n'est pas ce défaut** : c'est un ordre de
résolution du produit (`CONSUMES` posé à la place de `HAS_PARENT` ou d'`INHERITS_FROM` sur le
même couple) ; ses trois témoins par le chemin CSV du produit (sources ou arrivées déjà liées, en
automatique et dans un `BEGIN … COMMIT`) restent verts. Le défaut de ce ticket reste réel par
Cypher, dans la forme exacte du tableau ci-dessus.

## Ce que disait la lecture (avant la cause)

Le COPY de relations dans un groupe non vide passe par `CSRNodeGroup::appendChunkedCSRGroup` :
`NodeGroup::append` des lignes en mémoire, puis `updateCSRIndex(i, startRow, length)` pour chaque
nœud `i` de l'en-tête du lot, **y compris ceux de longueur 0** (une liste `[startRow, 0]` naît
pour chaque nœud du groupe). Le balayage (`CSRNodeGroup::scan`) lit d'abord le bloc persistant
(`scanCommittedPersistent`) puis, quand il rend vide et que `hasCSRIndex()`, passe aux listes en
mémoire. Le point à trouver est pourquoi la part persistante du nœud 0 n'est plus rendue une fois
sa liste en mémoire non vide : `scanCommittedPersistentWithoutCache` (un seul nœud lié), la
longueur persistante du nœud (`getNumDeletionsForNodeInPersistentData`, en-tête CSR), ou
`initScanForCommittedInMem` qui remet `nextRowToScan` avant que la part persistante soit lue. Le
chemin du CREATE (stockage local versé à la validation) ne montre pas le défaut : la différence
avec le COPY est dans l'en-tête du lot et `updateCSRIndex` pour tous les nœuds.

## Témoin

La sonde ci-dessus, à écrire dans `test/transaction/` (un fichier, la recette, les trois comptes,
puis la relecture après `CHECKPOINT` et après réouverture pour dire si les relations sont perdues
au stockage ou seulement au balayage). Rouge sur master.

## Pour le fermer

La cause trouvée et corrigée, le témoin vert, et le témoin du rollback sous lecteurs rejoué avec
un `CHECKPOINT` avant le COPY (il l'évite aujourd'hui exprès).
