# Un prédicat sur les nœuds et sur l'arête d'un saut fait balayer toute la table des arêtes

- **État** : ouvert (14 novembre 2026).
- **Famille** : moteur
- **Gravité** : lenteur — ×7,3 sur un saut, alors que chaque moitié du prédicat seule est presque gratuite
- **Atteignable en service** : oui, dès que 2a′ pose sa clause de visibilité sur les nœuds et les arêtes d'un saut ; contourné côté rag3weaver par une autre forme de requête (la forme C ci-dessous)
- **Touche rag3weaver** : non pour le défaut (le planificateur de rag3db) ; oui pour le contournement (la traduction IR → dialecte de 2a′-1)
- **Pour** : le cœur (le planificateur)
- **Signalé par** : everything declarative, mesure du 14 novembre 2026 sur une copie de code-lourd, lib 0c41f9fe6 (`docs/everything-declarative/14-11-2026/03-le-pas-2a-prime-stocker-les-generations-sans-fuite.md`, § 6)

## Ce que c'est

Sur une copie de code-lourd (82 855 `Scope`, 151 744 `CONSUMES`), avec trois colonnes ajoutées sur
`Scope` et `CONSUMES` (`_couche STRING`, `_depuis INT64`, `_jusqu_a INT64`, toutes les lignes à
`'ancre'`, 0, NULL), on fait un saut vers le nom le plus appelé (780 lignes en sortie). On compte la
médiane de 20 exécutions, seul sur le poste :

| forme | ms |
|---|---|
| sans prédicat | 4,8 |
| prédicat sur les deux nœuds seuls | 5,9 |
| prédicat sur l'arête seule | 4,6 |
| **A** : nœuds et arête dans une seule clause `WHERE` | **38,1** |
| B : `WITH` entre le bout et le saut | 25,3 |
| **C** : le bout filtré dans le `MATCH`, l'arête et l'autre bout après un `WITH` | **6,2** |
| D : une propriété de l'arête dans le motif | 35,8 |

Le prédicat de visibilité, pour une variable `v`, est
`v._couche = 'ancre' AND v._depuis <= 0 AND (v._jusqu_a IS NULL OR v._jusqu_a > 0)`.

- **La forme A** (le plan dans `14-11-2026/plans-du-predicat/forme-A-une-seule-clause.txt`) pousse le
  filtre de l'arête dans un `SCAN_NODE_TABLE` / `SEMI_MASKER` de toute la table, puis fait une
  jointure par hachage sur `b._ID` : toute la table des arêtes est lue.
- **La forme C** (`forme-C-with-apres-le-saut.txt`) filtre après la jointure, sur les seules lignes du
  saut.

## Le témoin à écrire (cœur)

Sur une petite base à deux tables de nœuds et une d'arêtes portant ces trois colonnes, la forme A ne
lit pas plus de lignes d'arêtes que la forme C (le compte des lignes lues ou le plan), et sa durée
reste à moins de ×1,5 de celle de C.

## Ce qui le fermerait

Un planificateur qui ne transforme pas un prédicat sur l'arête, conjoint à un prédicat sur le nœud
de départ, en balayage de la table entière, quand le nœud d'arrivée est déjà borné. Le témoin vert.
En attendant, rag3weaver écrit la clause sous la forme C (2a′-1).
