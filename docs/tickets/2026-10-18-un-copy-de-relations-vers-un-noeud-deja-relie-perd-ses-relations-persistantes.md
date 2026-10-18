# Un COPY de relations vers un nœud qui a déjà des relations persistantes les fait disparaître

- **État** : ouvert (reproduit sur master 8f4779bdc et sur le lot du rollback, luciepc, 18 octobre
  2026, par une sonde déterministe)
- **Gravité** : **perte de données en lecture** : après le COPY, validé ou annulé, les relations
  d'avant du nœud ne sont plus rendues (à vérifier après réouverture : perdues au stockage, ou
  seulement au balayage).
- **Atteignable en service** : oui, dès qu'un COPY de relations s'ajoute à une table qui a déjà
  des relations validées et reprises (point de reprise) pour les mêmes nœuds source.
- **Touche rag3weaver** : à vérifier (l'hôte insère ses arêtes par CREATE dans des transactions,
  pas par COPY ; une réindexation par COPY le toucherait).
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
annulé ou validé. Sans le `CHECKPOINT` (les dix relations en mémoire), tout est juste : le
défaut tient à la rencontre d'un bloc persistant et d'une liste en mémoire pour le même nœud.

## Ce que dit le code (lu, non prouvé)

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
