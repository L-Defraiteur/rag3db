# CI format18 : bâti réussi, premier arrêt de préparation avant tests

Même [ticket CI25oct ouvert](../../coeur-cpp-hotfixs-tickets/tickets/2026-10-25-la-ci-de-master-est-rouge-depuis-le-18-octobre-cassert-dans-geo-et-format-clang.md),
source74ad inchangée. Aucun test privé bloqué repris, aucune lib posée.

## Premier résultat natif

1099/1099 étapes de bâti réussissent, dont transaction_test, shell, vector et geo
liés. Puis le contrôleur échoue AVANT toute invocation transaction_test : il
créait test-tmp/test-work sous leur parent déjà remonté RO. Errno30 EROFS est une
erreur de préparation du runner, pas une erreur de compilation ni un rouge de test.
Le rapport de phase dit build car cette étiquette n'avait pas encore été changée ;
le build.log et son reçu returncode0 établissent le succès de compilation.

Unité cx-coeur-ci-format18-74ad-01 : service161,855s/CPU1244,091s/pic5,1G/swap0,
Max32G/scratch32G vérifiés, arrêt1. Aucun enfant test lancé ; poste rendu0/4,
MainPID0. Aucun replay inchangé. Les sources/flags restent ceux publiés dans doc22.

[Preuves arrêt01](preuves/ci-format18/arret-01/), incluant premier-echec.json,
reçu build0, manifeste avant test et journal unité. L'archive intégrale avec le
pack Git demeure hors worktree local et distant, SHA5289fbeeb15f873a3347711ab5b7419d69be808e9daad6f968427bf4764161cd.
La copie documentaire exclut seulement ce pack source déjà identifié par commit :
330220octets/SHAa43097e6d8c085125c8581cfe04113cb86c7c2ebba4872c90320359889231815.
29fichiers de l'archive intégrale rehashés sans écart ; reçu avec chemins stables.
Les artefacts natifs physiques étaient dans le scratch éphémère : leurs empreintes
sont conservées, mais pas de paquet final sauvé avant cette erreur de runner.

## Correction de préparation, publiée avant nouvelle tentative

[Delta outer](preuves/ci-format18/arret-01/delta-outer.patch),
[delta inner](preuves/ci-format18/arret-01/delta-inner.patch),
[recette corrigée et empreintes](preuves/ci-format18/arret-01/recette-corrigee.json).

- Créer test-tmp/test-work dans le scratch AVANT son gel et monter ces deux
  seules sorties privées RW. Source, build et extensions restent RO pendant tests.
- Exporter les binaires/headers avant la préparation et les tests pour qu'un autre
  rouge conserve ses artefacts ; aucune installation ou substitution de lib.
- Nommer distinctement la phase test_preparation. Aucun changement du C++, des
  flags, des quatre cibles ni de l'argv/env de transaction_test.

Le nouveau staging est neuf car le premier tmpfs a disparu. Budget **cumulé**
conservé :161,855s déjà consommées ; seconde unité2220s+10s arrêt au plus,
contrôleur2190s. Maximum cumulé2391,855s, sous les2400s annoncées, compilation
1800s max et tests540s max seulement dans le temps global restant. J8/Max32G/
swap0/scratch32G inchangés. Aucun relâchement de sandbox ni reconfiguration métier.

Ce delta est une correction nommée de la recette, pas un replay inchangé ou une
réinitialisation silencieuse du budget. Premier autre rouge : arrêt/diagnostic.
Aucun résultat de transaction_test encore disponible dans cette note ; revue et
tour master explicite restent nécessaires, CI future verte avant clôture.
