# Le nombre de lignes hors du graphe d'un index vectoriel, lisible sans écrire

- **État** : ouvert (8 novembre 2026, à la demande de l'orchestration ; après G3 et G5 dans la
  liste de la session hotfixs)
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
[les lignes d'un COPY forcé… restent hors du graphe](2026-10-18-les-lignes-d-un-copy-force-valide-apres-un-create-restent-hors-du-graphe.md)).
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

## Pour le fermer

La lecture, son témoin, et la ligne dans le ticket du 18 octobre qui dit que le produit peut
décider la reconstruction au seuil.

## Reprise du 16 novembre : contrat et témoin avant correctif

En cours par hotfixs, branche `cx-hotfixs-rows-outside-graph`, base
`927941a67a5805450d49f1e04b39fdd542a6059b` ; aucun lourd ni code produit.
[Note de lecture et chemins à coordonner](../16-11-2026/04-rows-outside-graph-contract.md).
La plage après numCheckpointedNodes n’est pas encore prouvée égale au nombre
exact de lignes visibles hors graphe : le code de recherche dédoublonne cette
plage et le graphe ; trous/NULL/suppressions/instantanés doivent aussi compter.
Avant implémentation, fixer le sens exact avec orchestration et arbre principal,
sans livrer un nombre d’offsets physiques sous un nom qui promet des lignes.
Témoin privé initial de projection à préparer ; échec attendu colonne absente,
NON JOUÉ à cette date. Aucun correctif avant rouge effectivement lu.
