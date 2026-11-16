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
La plage de la table au-delà de `numCheckpointedNodes` est parcourue par le
fallback, mais sa longueur physique n'est pas le nombre exact de vecteurs valides
visibles réellement absents du graphe : dédoublonnage, NULL, trous et versions
doivent être respectés. Le contrat du16 novembre ci-dessous remplace l'hypothèse
initiale d'une simple différence de compteurs. Aucune lecture ne rend aujourd'hui
ce nombre : `SHOW_INDEXES` dit `state` et `extension_loaded`, pas cette part.

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

Contrat confirmé par orchestration et arbre : compte exact des vecteurs valides
visibles effectivement absents du graphe de l’appel, sinon NULL explicite.
Témoin privé `test/transaction/vector_index_diagnostics_test.cpp`, cible séparée
dans CMakeLists transaction, quatre cellules de lecture/contre-exemple contrôlé/
trous et NULL après reconstruction. NON COMPILÉ/NON JOUÉ ; aucun rouge lu.
Cellule postCOPY déterministe, cohérence d’instantané et coût restent à établir
avant production. Aucun fichier de production ni instrumentation C1 modifié.

Reprise après revue globale cœur : nouvelle branche/worktree
`cx-hotfixs-rows-outside-graph-df2`, rebasée sur
`df2a5789888d16247f0dad42dcdaa3c4ceea2094` ; anciennes preuves conservées.
Quatre fixtures actualisées avec BUILD_ID natif et projection complète des huit
champs stables + UINT64 final. Toujours NON COMPILÉES/NON JOUÉES.
[Recette du rouge pour revue](../16-11-2026/05-rows-outside-graph-red-recipe.md) :
vrai callee SHOW_INDEXES, rouge binder attendu, budget proposé2400s/32G ; pilote
exact à adapter et relire avant admission, aucun GO lourd ni production.

Pilote concret préparé ensuite sur branche/worktree distinct
`cx-hotfixs-rows-outside-graph-red-pilot`, sans changer source5099 ni C++ :
[superviseur et limites](../16-11-2026/06-rows-outside-graph-pilot.md).
2400s après vrai poste, unité privée bornée, un seul appel réel SHOW_INDEXES
attendu rouge au binder, résumé final toujours ROUGE. Préparation PC et run
NON FAITS ; recette hashée/RO/admission séparées avant tout lourd. Aucun FIX.

Préparation privée PC faite ensuite, autorisation C7 fichiers/Git seulement :
source5099/pilotedb833 détachés propres, bundle69688d86...42ba3 (657484714octets),
recette exacte a31745c5...ef44d [publiée](../16-11-2026/rows-red-prepared-recipe.json).
Quatre scripts rehashés conformes, raw/résumé/allocation absents et scratch vide.
Aucun CMake/test/build/poste/systemd ; rouge natif toujours NON OBSERVÉ, aucun FIX.

GO distinct C7-ROWS-RED-DB833-16nov2400 consommé ensuite UNE fois :
[refus de compilation et preuves](../16-11-2026/07-rows-red-compile-refused.md).
La liste initializer_list de vector<LogicalType> exige sa copie privée :
compilecode1, aucun cas exécuté, aucun rouge binder observé. Fin237,437s,
quiescent/aucun PID restant, collecteur1 et résuméROUGE. Paquet38fichiers
réhashé2postes0écart, manifestef06755...c179. Aucun retry ni production modifiée.
C7 autorise PRÉPA correction FIXTURE SEULE après cette MAJ AVANTdiff : utiliser
move/emplace pour la construction du vecteur attendu sans toucher aux quatre
assertions/contrat ; nouveau SHA/RO/GO nécessaires avant compilation/rejeu.
