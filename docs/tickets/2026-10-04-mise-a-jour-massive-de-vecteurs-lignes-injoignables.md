# Après une mise à jour massive de vecteurs, des lignes restent injoignables dans l'index

- **État** : ouvert
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : peu. Son chemin principal, des lignes créées sans vecteur puis remplies, est vert ; seul le réembarquement d'une ligne gardée passe par ce défaut, et rag3weaver repasse par NULL ligne à ligne, ce qui est vert
- **Ouvert le** : 4 octobre 2026, session du banc (le défaut est au journal des chantiers §6 depuis le 3 octobre)
- **Pour** : cœur C++ (extension vector)

## Ce que c'est

Quand on remplace les vecteurs de beaucoup de lignes d'une table indexée, une recherche
exhaustive ne rend plus toutes les lignes, ou une ligne n'est plus rendue première quand on
cherche son propre vecteur. Le compte des pertes change d'une passe à l'autre (tirage des
niveaux du graphe). Depuis `c8fdaf196` (les anciens voisins restent joignables), environ un
essai sur deux perd encore des lignes.

## Recette minimale

```cypher
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);
UNWIND range(0, 999) AS i CREATE (n:Doc {id: i}) SET n.vec = [i % 17, i % 23, i % 29, i];
CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');
MATCH (n:Doc) WHERE n.id >= 0 AND n.id < 512 SET n.vec = [n.id % 13 + 0.5, n.id % 19, n.id % 31, n.id + 0.25];
MATCH (n:Doc) WHERE n.id >= 512 AND n.id < 1000 SET n.vec = [n.id % 13 + 0.5, n.id % 19, n.id % 31, n.id + 0.25];
-- recherche exhaustive (k = efs = 1000) depuis le vecteur d'une ligne : moins de 1 000 lignes
-- rendues, ou des lignes qui ne sortent pas premières sur leur propre vecteur, un essai sur deux
```

## Témoin

`test/transaction/concurrence/vector_index_update_test.cpp` (étiquette `vector-index-exact`),
chaque cas répété sur des tables neuves :
- `SetToAnotherVectorInBatchesOf512`, `TwentyRowsToDistinctVectorsLineByLine`,
  `OneRowUpdatedManyTimes`, `UpdateThenDelete` : rouges connus, cas longs
  (`CONCURRENCE_LONG=1`) ;
- `SetToAnotherVectorLineByLine`, `TenThousandRowsInBatchesOf512` : probabilistes ;
- `TwentyRowsToTheSameVectorInOneStatement` : probabiliste depuis l'élagage classique
  (10 octobre) : 1 perte sur 400 essais, contre 0 sur 400 avant lui (non significatif). Vingt
  lignes posées sur le même vecteur : des copies, que la borne de l'élagage garde au plus près
  en décalage. La référence de coût de la mise à jour massive se prend sur cet élagage.

### TwentyRowsToTheSameVectorInOneStatement sous l'élagage classique : ce qui est établi ou non (10 octobre)

Le scénario : 1 000 lignes indexées (l2, dimension 4), puis vingt d'entre elles posées sur le même
vecteur `[7, 7, 7, 7]` en une instruction, par le chemin des mises à jour. Le contrôle compte en
défaut un essai où une ligne n'est pas atteinte par la recherche exhaustive, ou n'est pas rendue par
la recherche de son propre vecteur (à égalité près, parmi trente, assez pour les vingt copies), ou
une ligne supprimée revient.

- **Ligne présente dans la table mais pas dans le graphe ? Non établi.** La perte unique (1 essai
  sur 5, pendant la liste complète) n'a laissé que l'étiquette `vector-index-exact`, qui couvre les
  deux cas : la comparaison du banc ne garde pas la sortie détaillée de l'essai. Je ne sais donc pas
  si une ligne a été hors du graphe (un îlot) ou seulement hors de la recherche de son vecteur.
- **Ce qui est établi par la lecture** : vingt copies, c'est sous la borne de l'élagage (la moitié
  du degré, 30). Toutes sont gardées dans la liste de chacune, en premier, et ni la borne ni l'ordre
  du remplissage (par le plus lointain) ne jouent entre elles. Si l'élagage y est pour quelque chose,
  c'est par les listes des autres nœuds, réélaguées par la mise à jour : pour un nœud voisin, les
  vingt copies sont à **distance exactement égale**, et l'ordre entre elles tient au tri, qui n'est
  pas stable, sur des candidats que l'insertion parallèle livre dans un ordre variable.
- **Le compte** : 1 perte sur 400 sous l'élagage classique, 0 sur 400 avant lui. Non significatif.
  La perte est tombée sur un poste très chargé, ce qui va avec une course sur l'ordre, sans le
  prouver.

**Suites possibles, non demandées** :
- garder la sortie détaillée des essais rouges dans la comparaison du banc (`compare_known_red.cmake`),
  pour que la prochaine perte dise si c'est un îlot ;
- un bris d'égalité déterministe sur les distances égales (par décalage, après la distance) dans le
  tri des candidats, qui rendrait l'élagage de ces vingt copies reproductible ; à mesurer comme les
  autres variantes (extension prouvée, vrais vecteurs, ce témoin rejoué un grand nombre de fois).

## Cause

D'après la session cœur C++ : l'élagage des voisins retire des arêtes entrantes à des nœuds
que personne ne recontrôle. Il faudrait un passage en fin d'instruction, comme le
`finalize` de la suppression. L'état de mise à jour de l'index est aujourd'hui recréé à
chaque ligne.

## Correctif de l'amont (à lire, ne pas copier)

Aucun : Ladybug n'a ni mise à jour ni suppression dans son index HNSW.

## Pour le fermer

Les témoins passent au vert à chaque essai. Contournements sûrs en attendant : supprimer
puis réinsérer la ligne ; en masse, retirer l'index, poser les vecteurs, le recréer.
