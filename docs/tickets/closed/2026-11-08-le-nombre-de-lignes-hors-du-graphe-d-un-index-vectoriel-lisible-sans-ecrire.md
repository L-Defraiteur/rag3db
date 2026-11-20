# Le nombre de lignes hors du graphe d'un index vectoriel, lisible sans écrire

- **État** : fermé (20 novembre 2026) par le lot (a), sur master depuis le 18 novembre au soir ; ouvert le 8 novembre
  2026, à la demande de l'orchestration
- **Famille** : moteur
- **Gravité** : lenteur — sans ce nombre, le produit ne sait pas quand reconstruire un index dont
  une part de lignes est hors du graphe, et chaque recherche la balaie une à une
- **Atteignable en service** : oui, dès que des lignes validées restent hors du graphe (un COPY
  forcé validé juste après un CREATE_VECTOR_INDEX, le CREATE concurrent)
- **Touche rag3weaver** : oui — c'est lui qui décidera la reconstruction (`rebuild_vector_index`)
- **Pour** : session hotfixs (cœur C++)

## Ce que c'est

Les lignes d'une table indexée qui ne sont pas dans le graphe HNSW sont balayées une à une par
chaque recherche (ticket
[les lignes d'un COPY forcé… restent hors du graphe](../2026-10-18-les-lignes-d-un-copy-force-valide-apres-un-create-restent-hors-du-graphe.md)).
Le moteur connaît leur nombre : les lignes de la table au-delà de `numCheckpointedNodes`
(`extension/vector/src/include/index/hnsw_index.h:93`), plus celles que le dernier bâti n'a pas
prises. Aucune lecture ne le rend aujourd'hui : `SHOW_INDEXES` dit `state` et `extension_loaded`,
pas cette part.

## Le seuil qui l'attend

Le critère retenu par la session recherche : **la recherche ne doit pas doubler**. Avec la mesure
du banc du 8 novembre (t₀ ≈ 13,4 ms à k = 10 sur 120 000 vecteurs de dimension 768, ≈ 1 µs par
ligne hors du graphe), le seuil est N* ≈ 13 400 lignes à la dimension 768. Le seuil se calcule par
le produit, depuis ce nombre et la dimension ; le moteur ne fait que le rendre.

## La forme attendue

Une lecture sans écriture ni verrou d'écriture :
- soit une colonne de `SHOW_INDEXES`, par exemple `rows_outside_graph` (nulle pour un index qui
  n'en a pas la notion), à la manière de `state` et d'`extension_error` ;
- soit un `CALL` en lecture de l'extension vector, qui rend aussi les lignes du graphe.

Elle doit répondre sous l'instantané de l'appelant, comme `state` (lu au holder du stockage).

## Le témoin

Un index bâti, puis un COPY forcé validé juste après le CREATE (la recette du ticket du 18 octobre)
: la lecture rend le nombre de lignes hors du graphe ; après la reconstruction, elle rend zéro.

## La décision de Lucie (18 novembre 2026, par c7)

« ok (a) mais étudier si (b) est si lourd à faire et ce qu'il pourrait permettre d'autre ».

- **(a), à faire en lot, rouge d'abord** : une borne supérieure sans changement de format, la colonne
  `rows_outside_graph_at_most` de `SHOW_INDEXES`. Elle compte les lignes au-delà de `numCheckpointedNodes`, plus ce
  que le dernier bâti a sauté, et elle est nommée comme une borne, pas comme un compte exact. Elle suffit au seuil de
  reconstruction du produit (N* ≈ 13 400 lignes à la dimension 768). Le témoin de Codex
  (`test/transaction/vector_index_diagnostics_test.cpp`, branche `cx-hotfixs-rows-outside-graph-red-pilot-v2`) se
  reprend, avec le nom de colonne et la valeur attendue de (a).
- **(b), une page courte, sans code, « à lire par Lucie »**, après (a). Elle chiffre ce que le marqueur d'appartenance
  coûterait vraiment (format, GC et MVCC, migration des bases, en jours), et ce qu'il ouvrirait d'autre (quelles
  requêtes ou quelles garanties). La conception de Codex
  (`docs/coeur-cpp-hotfixs-tickets/17-11-2026/01-…` à `03-…` sur sa branche) en est le point de départ.

## Pour le fermer

La lecture, son témoin, et la ligne dans le ticket du 18 octobre qui dit que le produit peut
décider la reconstruction au seuil.

## La fermeture (20 novembre 2026)

**La lecture** : `SHOW_INDEXES` rend une neuvième colonne, `rows_outside_graph_at_most` (UINT64), sous l'instantané de
l'appelant et sans verrou en lecture seule (`221b1d801e`). Pour un index vectoriel, ce sont les lignes de
`[numCheckpointedNodes, numTotalRows)`, celles que chaque recherche balaie une à une. C'est une **borne** : elle compte
aussi, dans cette plage, les lignes supprimées et celles déjà reliées par le rattrapage d'un CREATE. NULL pour un index
qui n'en a pas la notion. Choix (a) de Lucie du 18 novembre ; la page (b) (le marqueur d'appartenance) est à part.

**Le témoin** : `VectorIndexDiagnosticsTest` (`test/transaction/vector_index_diagnostics_test.cpp`) et le COPY forcé
d'`AForcedCopyCommittedRightAfterACreateIsReached`, qui lit ses mille lignes dans la colonne, puis 0 après la
reconstruction. Rouges sans la colonne, verts avec.

**Trouvé en chemin, et corrigé dans le même train** (`1cfdaf353e`) : une insertion validée après un COPY forcé posait
`numCheckpointedNodes` au-delà des lignes du COPY, qui n'étaient alors ni dans le graphe ni balayées (une réponse
fausse), et que la colonne aurait comptées 0. Ticket fermé :
`docs/tickets/closed/2026-11-18-une-insertion-apres-un-copy-force-fait-sauter-ses-lignes-a-la-recherche.md`.

**La ligne dans le ticket du 18 octobre** est écrite : le produit peut décider la reconstruction au seuil.
