# Pilote concret du rouge SHOW_INDEXES, sans exécution native

Même [ticket](../tickets/2026-11-08-le-nombre-de-lignes-hors-du-graphe-d-un-index-vectoriel-lisible-sans-ecrire.md), entrée4 ; [contrat](04-rows-outside-graph-contract.md) et [recette05](05-rows-outside-graph-red-recipe.md).
Source C++ figée `5099a367ddf1e48db2b4065d91a61403de994571`, base
`df2a5789888d16247f0dad42dcdaa3c4ceea2094`, moteur attendu
`59ac9c86bc2b3529b590a7462ab7051e3ad26b8a`. Les nouveaux scripts vivent dans un
worktree/branche distinct `cx-hotfixs-rows-outside-graph-red-pilot` ; aucun fichier
C++ ni CMake modifié depuis5099. Aucun fichier produit réservé.

## Préparation fichiers/Git distincte de l'admission

[prepare.py](rows-red/prepare.py) exige sur luciepc un worktree source privé à5099
propre et un worktree pilote au SHA complet publié propre. Il refuse le réemploi
des dossiers et crée seulement un bundle Git complet autonome, ses copies de
scripts et la recette avec SHA256. Le bundle comprend l'historique nécessaire au
BUILD_ID : pas de clone partagé ni d'alternates, ni de source externe à lire
pendant le run. Pas de sous-module Rust requis par les seules cibles C++.
Une préparation interrompue reste conservée et n'est pas automatiquement réessayée.

```
python3 docs/coeur-cpp-hotfixs-tickets/16-11-2026/rows-red/prepare.py \
  --pilot <SHA40-pilote> --source-root <worktree-prive-absolu-a-5099>
```

Ce script ne prend aucune place, ne lance ni système natif, ni unité, ni service.
Il crée `~/.cache/rag3weaver-build/cx-hotfixs-rows-outside-graph-red-v1` et un
répertoire vide `cx-hotfixs-rows-red`. Préparation PC NON FAITE à la publication.
Pas de lecture du SDK/hôte59, de base Lucie, d'API7970 ni de clef.

## Commande sous admission exacte, pas exécutée

Après revue RO du SHA de pilote ET de la recette préparée, et attribution C7
distincte de capacité : `bash <dossier-red-v1>/launch.sh`.
[launch.sh](rows-red/launch.sh) retire les overrides CPU, prend le vrai `poste lourd`
32G/swap0 puis appelle GNU timeout TERM2390/KILL10 autour de toute
[entry.py](rows-red/entry.py), finalisation incluse. Aucun travail final hors
watchdog, aucun timeout de file confondu avec le budget après acquisition.

Entry reprend les garanties du superviseur cœur0de relu, avec bornes2400 :
controller≤2310, unité RuntimeMax≤2320 plus Stop10, fin native≤2330 après
déduction de la préparation, finalisation≤2380. Unité privée
`cx-hotfixs-rows-red-01.service`, `KillMode=control-group`, cpu quota exact du
parent admis, j=min(8,POSTE_FILS,floorquota), mémoire/swap vérifiés. Scratch
tmpfs32G, réseau absent, belle priorité d'arrière-plan. L'unité préexistante
est refusée avant invocation ; aucune unité d'autrui n'est arrêtée.

[controller.py](rows-red/controller.py) clone uniquement le bundle vérifié,
vérifie SHA source et fichiers natifs, configure120s, compile1800s les seules
cibles `vector_index_diagnostics_test` et extension vector. Staging60s commun.
Toutes bornes MIN(soldeglobal), jamais une addition qui dépasse2400. Source RO
en configuration/compilation sauf extension/vector/build privée, puis source et
build RO durant le test. Tmp/home/résultats privés seuls RW. Environnement nettoyé,
IN_MEM_MODE=false, MAX_NUM_THREADS2, OMP/OPENBLAS bornés aux jobs admis, ccache et
Git auto-maintenance désactivés. Aucun Cargo, SDK magasin, instrumentation C1,
geo, filtre canonique ou service commun.

Avant le test : copie ELF/extension/archive moteur/header/CMakeCache/commandes
de compilation, SHA et identité native concordants, sans réécrire le BUILD_ID.
Puis UNE invocation exacte du cas1 de5099, fail_fast, cap30s et XML brut :
`VectorIndexDiagnosticsTest.ACompleteIndexReportsAKnownZero`.
Les trois autres fixtures restent NON JOUÉES. Chaque groupe est TERM/KILL/récolté
même leader déjà sorti, résidus cgroup avant/après refusent toute suite.

## Rouge attendu, jamais une passe de livraison verte

[receipt.py](rows-red/receipt.py) exige sortie1, un seul cas effectivement joué,
une seule failure sans error/skip, identité native attendue unique, puis assertion
`rows-outside-graph-is-readable` et erreur binder exacte
`Variable rows_outside_graph is not in scope.`. Les prémisses (extension chargée,
quatre vecteurs, index READY) doivent avoir réussi. Erreur de compilation,
ressources, identité, prémisse ou forme inattendue : refus, pas reproduction.
Le format XML/message natif est encore NON QUALIFIÉ ; s'il diffère, conserver le
refus et demander une nouvelle revue, aucun élargissement automatique du parseur.

Après résultat : rehash des artefacts, quiescence de l'unité, aucun PID attribué,
inventaire brut hashé, reçu et résumé. `EXPECTED_BINDER_RED_OBSERVED` signifie
uniquement que le rouge d'API manquante est vu ; le résumé finit TOUJOURS par
`ROUGE`. Code0 du collecteur signifie preuves du rouge recueillies, aucun droit
de push master. Reçu incomplet/arrêt forcé : absence de preuve, jamais vert ni
quiescence inventée. Pas de retry ni nettoyage des anciens témoins.

## Vérifications et limites

Six tests de reçu synthétiques publiés avec le pilote : stricte identité et cas,
refus prémisses/poolfull/autrecolonne/succès/crash/skip/erreur supplémentaire,
localisation source et message binder attendu. Ils ne font appel à aucun moteur.
Aucun rouge produit ni observation de la colonne manquante n'est revendiqué par
ces fixtures XML fabriquées. Leur statut CPU et les vérifications AST/bash/diff
seront publiés séparément après le SHA de cette préparation.

Pilote natif NON JOUÉ, C++5099 NON COMPILÉ ; aucun contre-exemple réellement
observé. Le premier rouge natif reste obligatoire avant tout FIX. Exactitude
snapshot/graphe, vrai postCOPY déterministe, repli NULL et coût restent dans le
même ticket ; ce premier rouge ne qualifie aucun de ces contrats. Revue externe
et admission neuve obligatoires avant tout lourd.
