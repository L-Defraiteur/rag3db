# Une insertion validée après un COPY forcé fait sauter ses lignes à la recherche

- **État** : ouvert (18 novembre 2026, session hotfixs ; trouvé en écrivant le lot (a) du ticket
  [le nombre de lignes hors du graphe](2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md))
- **Famille** : moteur
- **Gravité** : réponse fausse — des lignes validées ne sont plus jamais rendues par la recherche
  vectorielle, sans erreur, jusqu'à la reconstruction de l'index
- **Atteignable en service** : dès qu'un COPY forcé valide juste après un CREATE_VECTOR_INDEX (la
  recette du ticket du 18 octobre,
  [les lignes d'un COPY forcé… restent hors du graphe](../../tickets/2026-10-18-les-lignes-d-un-copy-force-valide-apres-un-create-restent-hors-du-graphe.md)),
  puis qu'une insertion ordinaire suit ; plus largement, dès qu'un trou existe entre le graphe et
  la fin de la table
- **Touche rag3weaver** : oui — ses recherches perdent ces lignes, et la colonne
  `rows_outside_graph_at_most` les dit à 0, donc aucune reconstruction ne se déclenche
- **Pour** : session hotfixs (cœur C++)

## Ce que c'est

La recherche lit le graphe, puis balaie une à une les lignes de `[numCheckpointedNodes,
numTotalRows)` (`searchFromUnCheckpointed`). Le rattrapage d'un CREATE s'arrête à la première ligne
d'un COPY forcé pas encore validé : ces lignes restent hors du graphe, au-delà de
`numCheckpointedNodes`, et le balayage les trouve.

`OnDiskHNSWIndex::commitInsert` relie au graphe chaque ligne d'une insertion ordinaire, puis pose
`numCheckpointedNodes = offset + 1` sans regarder ce qui la précède. La première insertion validée
après le COPY pose donc la borne au-delà des lignes du COPY : elles ne sont plus dans la plage
balayée, et elles n'ont jamais été dans le graphe.

## Le témoin

`VectorIndexUpdate.AnInsertAfterAForcedCopyKeepsItsRowsReachable`
(`test/transaction/concurrence/vector_index_update_test.cpp`) : la recette du COPY forcé (21 000
lignes atteintes), puis une insertion ordinaire. Rouge trois fois sur trois sur 324cd097bc
(luciepc, Release) : « 20001/21001 reached », contrôle
`rows-of-a-forced-copy-stay-reachable-after-an-insert`.

## Le correctif attendu

La borne n'avance que d'une ligne qui la suit immédiatement : une ligne reliée au graphe, ou une
ligne sans vecteur (elle n'est jamais un résultat ; un SET qui lui donne un vecteur passe par
`update` et la relie). Une ligne reliée au-delà d'un trou reste aussi dans la plage balayée ; le
balayage ne rend pas deux fois une ligne déjà rendue par le graphe.
