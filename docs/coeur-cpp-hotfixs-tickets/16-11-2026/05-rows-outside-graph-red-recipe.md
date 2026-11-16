# Compteur hors graphe : recette du premier rouge, à relire avant admission

Ticket : [lecture hors graphe](../tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md), entrée4 de la passation.

Statut : **PRÉPARÉ, NON COMPILÉ, NON JOUÉ**. Cette publication demande une revue de
recette, ne lance rien et ne réserve aucun fichier de production. Le premier rouge
reste à observer avant tout correctif. Aucune ABI, lib du magasin, SDK ou service
7970 ne change ; aucune base de Lucie n'entre dans cette recette.

## Source et rapprochement

Base relue : `df2a5789888d16247f0dad42dcdaa3c4ceea2094`. Nouveau worktree
`/home/lucied/git_workspaces/cx-hotfixs-rows-outside-graph-df2`, branche de même nom.
Ancienne branche `cx-hotfixs-rows-outside-graph` et son worktree conservés.
Le range-diff des deux commits `927941a6..03fb43f8d6` vers
`df2a578988..4f64e3959a` donne deux égalités : aucune résolution masquée.
Depuis927, aucun changement sous `src/`, `extension/vector` ou `test/transaction`
dans cette base. Le delta cœur `68590270..888c81825` déplace la réservation des
offsets du dictionnaire et ajoute sa cible autonome ; il ne change pas les
primitives utilisées ici. Son bundle global demeure indépendant et gelé.
Sa cible `dictionary_append_exception_test` n'est pas dans masterdf2 : ne pas
la copier ici ; conserver ses lignes si elle arrive lors d'un rebase ultérieur.

Consommateurs relus dans cette base :
`generic_search_nodes.rs:208-234` cherche les positions par nom ;
`dialect.rs:2061` demande `RETURN *` ; `backend.rs:2849` demande explicitement
`extension_error`. Aucun de ces fichiers n'est modifié ni réservé.

## Quatre fixtures, une cible et une vraie lecture

Cible `vector_index_diagnostics_test`, source
`test/transaction/vector_index_diagnostics_test.cpp`. Chaque fixture émet elle-même
`BUILD_ID=` depuis le header généré. Ce n'est pas une ligne ajoutée par le pilote.
Le build_id de la source testée doit être identique dans le header, l'ELF moteur,
l'extension vector et les sorties des cas effectivement exécutés.

1. `ACompleteIndexReportsAKnownZero` : quatre FLOAT[4], index READY ; lecture
   nommée par `conn->query("CALL SHOW_INDEXES() RETURN index_name, rows_outside_graph;")`.
2. `ReadingTheCountInAReadOnlyTransactionTakesNoWriteLocks` : même lecture sous
   `BEGIN TRANSACTION READ ONLY`, transaction réellement read-only, aucun verrou
   détenu avant/après. Cela ne mesure pas une attente ni un verrou transitoire.
3. `AScanRangeContainingAlreadyIndexedRowsIsNotAnOutsideCount` : quatre résultats
   atteints avec watermark4, puis watermark0 temporaire restauré par RAII,
   graphe inchangé : le compte exact reste0. Contre-exemple contrôlé de fixture,
   pas preuve d'un entrelacement de production.
4. `DeletedRowsAndNullVectorsDoNotBecomeOutsideRowsAfterRebuild` : suppression,
   vecteur NULL, reconstruction ; deux vecteurs éligibles et compte exact0.

Après réussite de la projection nommée, chaque lecture vérifie aussi `RETURN *` :
les huit champs historiques et leurs types restent dans le même ordre,
`rows_outside_graph` est en neuvième position, type UINT64. La nullabilité est
un contrat de valeur (pas un drapeau exposé par LogicalType) ; ces fixtures
non concurrentes exigent un zéro connu. Elles ne testent PAS encore le cas NULL.

Le premier passage ne joue que le cas1 avec `--gtest_fail_fast` et XML privé.
Sur cette base sans correctif, le rouge légitime attendu est le refus au binder
de la colonne `rows_outside_graph`, après prémisses toutes réussies. Un échec de
compilation, extension absente, index non READY ou identité incohérente est un
refus de recette, pas la reproduction du défaut. Aucun appel à une méthode C++
inventée ; aucune évaluation Graph/count ne remplace ce callee SHOW_INDEXES.
Les trois autres cas restent NON JOUÉS tant que la colonne est absente ; ils
seront joués sans skips après le correctif, avec inventaire exact des quatre noms.

## Enveloppe proposée, non admise

Une seule place `poste lourd` sur luciepc, budget32G, swap0, j=min(8,
POSTE_FILS, quota effectif), délai global2400s **après acquisition réelle**.
Préparation60s, configure120s, build1800s, témoin30s, finalisation réservée60s ;
chaque délai est MIN(solde), jamais ajouté au global. Pas de GPU ni de Cargo.
Le quota enfant ne doit pas être800% si le poste a admis moins.

Le pilote doit être publié et relu avant admission : réutiliser les garanties
de supervision relues au cœur0de (watchdog extérieur TERM2390/KILL10,
unité native privée RuntimeMax≤2320/Stop10/KillMode=control-group,
fin native≤2330, finalisation≤2380, groupes récoltés même leader sorti,
preuve de quiescence avant verdict). Ce document ne prétend pas qu'un pilote
adapté existe déjà. Aucun lancement de cette recette tant que l'adaptation
exacte, ses SHA et l'admission de capacité ne sont pas publiés.

Source figée à un SHA complet portant uniquement les tests et ces docs,
exportée en bundle vérifié ; clone/scratch/build privés, réseau coupé,
source RO pendant tests, seuls tmp/home/résultats privés RW. CMake doit écrire
les extensions uniquement dans le clone privé pendant le build. Référence RO,
aucun ccache partagé, aucun accès aux bases/hôtes/SDK/lib59 du magasin.
Répertoire privé prévu sous `~/.cache/rag3weaver-build/cx-hotfixs-rows-outside-graph`
(pas créé ni préparé à distance par cette publication).

Configuration Release, Ninja, BUILD_TESTS=ON, BUILD_SHELL=OFF,
BUILD_EXTENSIONS=vector, BUILD_SHARED_LIBS=OFF, sanitizers/LTO/runtime-checks OFF,
AUTO_UPDATE_GRAMMAR=OFF, FETCHCONTENT_FULLY_DISCONNECTED=ON,
FETCHCONTENT_UPDATES_DISCONNECTED=ON, ccache désactivé. Seules cibles à bâtir :
`vector_index_diagnostics_test` et `rag3db_vector_extension`.
Environnement tests nettoyé : PATH=/usr/bin:/bin, LANG/LC_ALL=C, TZ=UTC,
IN_MEM_MODE=false, HOME/TMPDIR/RUNNER_TEMP privés, MAX_NUM_THREADS=2,
OMP_NUM_THREADS/OPENBLAS_NUM_THREADS bornés aux jobs admis.

Commande native prévue, dans le confinement privé et avec les chemins résolus :

```
<build>/test/transaction/vector_index_diagnostics_test \
  --gtest_filter=VectorIndexDiagnosticsTest.ACompleteIndexReportsAKnownZero \
  --gtest_color=no --gtest_fail_fast --gtest_output=xml:<journaux>/raw/red/result.xml
```

Exporter avant jugement les argv, sorties configure/build, ELF/extension/header,
CMakeCache/compile_commands et leurs SHA256. Garder stdout natif exact, XML,
code retour et refus initial ; vérifier identité avant tout autre cas.
Inventaire final des preuves et quiescence de l'unité/cgroup requis. Un rouge
attendu reste **ROUGE**, jamais une passe verte permettant de livrer. Un défaut
de supervision ou une preuve manquante reste aussi ROUGE avec sa cause distincte.
Pas de retry automatique, pas de nettoyage des anciens journaux.

## Frontière avant production

Les quatre fixtures ne prouvent ni une queue postCOPY non vide déterministe,
ni la cohérence snapshot/graphe sous concurrence, ni le repli NULL (holder absent,
extension indisponible), ni le coût de calcul. Ces cellules restent dans le même
ticket. Un graphe non versionné ne devient pas cohérent avec un snapshot par un
simple `const` ou un pointeur partagé. Tant que la preuve manque : NULL ; jamais
nombre physique moins watermark sous le nom `rows_outside_graph`.
Réservations production à coordonner avant édition ; aucune prise ici.
