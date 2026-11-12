# CI clang-format18 : inventaire, source pure et recette privée

Nuit du12au13novembre2026. Même [ticket CI ouvert](../../coeur-cpp-hotfixs-tickets/tickets/2026-10-25-la-ci-de-master-est-rouge-depuis-le-18-octobre-cassert-dans-geo-et-format-clang.md),
porteur cœur C++, propriétaire unique de la fenêtre C++ pure. Base a783013f1,
worktree cx-coeur-ci-format18. **Aucune bibliothèque posée, aucun code logique ou
trace privée bloquée repris.** Source/recette publiées avant bâti/test privé.

## Faits CI lus, sans lancer ou relancer un workflow

Dernier CI Workflow observé : [43313396407](https://github.com/L-Defraiteur/rag3db/actions/runs/43313396407),
en attente sur a783 ; les précédents récents sont annulés. Le dernier échec
terminé de ce workflow lu est [40246212509](https://github.com/L-Defraiteur/rag3db/actions/runs/40246212509) :
sanity checks vert, code format rouge à git diff --exit-code ; shell build rouge
séparé. Pas de CI actuelle déclarée verte ni de nouveau défaut attribué au format.
[Faits et extrait du log du job](preuves/ci-format18/faits-ci.json).

Le log d'installation confirme clang-format-18 paquet **1:18.1.3-1ubuntu1**.
L'image locale déjà disponible rag3db-clang-format-18 (Ubuntu24.04) a été vérifiée :
image SHA d58d28d7a16180e13240382444b550ef1a0d186731665d6abccdad641343af45,
binaire /usr/bin/clang-format-18 SHA
55cc4a84a64347d12bbcc3fbe9c16599c57a75fda70703d1b91fc95ce1958d84,
version18.1.3-1ubuntu1, libclang-cpp/libllvm de même paquet. Pas de version23 pour
formater, pas d'installation de paquet ni accès réseau dans le conteneur.
Le workflow installe le major18 via apt sans figer son patch : le futur log CI
reste à vérifier avant clôture, sans prétendre connaître un job encore non joué.

Le retrait cassert est déjà sur master depuis9bf7aca5c : aucun refaire ni nouvelle
attribution. Les contrôles check-no-std-assert de src et extension passent, ainsi
que le hash de grammaire. La CI code-format a aussi des étapes Python/Rust : elles
ne sont pas réécrites ni déclarées validées par ce lot C++.

## Périmètre inventorié et exclusions

Le script exact scripts/run-clang-format.py énumère2094candidats : src1550,
test166,tools82,extension296. Il lit .clang-format-ignore. Les29annexes C++ sous
extension/rag3weaver/docs sont exclues, ainsi que le sous-module codeparsers ;
les docs racine sont aussi protégées. Restent2065sources, toutes suivies.
[Inventaire avec chemins](preuves/ci-format18/inventaire.json).

Exclusions publiées seules dans **25e5d653d** : docs/*, */docs/* et chemin du
sous-module avec ses descendants. Aucun sous-module initialisé/réécrit, aucune
source privée0511/35ea/fixture forensique importée ; les sources hors ce worktree
ne sont pas montées dans le conteneur. Le lien source tools/rust_api/rag3db-src
n'est pas suivi par os.walk ; aucun double parcours du dépôt.

## Formatage pur et vérifications

Commit **74ad9c25cd2a63e01abd54f25314992cda6fea60** :253fichiers C++ seulement,
1887insertions/1723suppressions, tous dans les2065candidats autorisés. Généré par
l'image vérifiée, pas d'édition manuelle de ces fichiers. Régions visuellement
relues : regroupement d'includes, mise en ligne des conditions et commentaires,
indentation/retours à la ligne, littéraux adjacents ; aucun changement métier.

Formatage4,489s puis contrôle réel4,539s, codes0, sortie du contrôle vide,
conteneurs supprimés. Sous poste lourd local, j8, conteneur1G/swap0/CPU8,
réseau coupé/racineRO, seul worktree possédéRW, délai180s. Les argv et reçus
sont [dans les preuves](preuves/ci-format18/).

**Piège du script : --dry-run n'est pas une validation.** Cette option imprime
les commandes et retourne sans invoquer clang-format. Le contrôle ici utilise
le script sans --in-place et sans --dry-run : il calcule le diff et refuse les
écarts. L'inventaire confirme2065fichiers non vides à juger ; pas de vert vide.

[Revue lexicale](preuves/ci-format18/revue-lexicale.json) des253fichiers avant/après :
zéro écart de tokens hors normalisation documentée des espaces/commentaires,
multiensemble des includes par région préprocesseur, littéraux adjacents et using
consécutifs. [Script](outils/cx-verifier-format-cpp.py), Pygments déjà présent.
Ce n'est pas une preuve formelle d'AST : l'ordre des includes est changé par le
style et exige encore le bâti privé et transaction_test. Aucun changement de
macro/constante/contrat recherché ni ajout de logique dans le commit de style.

Une première tentative de push du commit de style a reçu le refus serveur
générique `(failed)` ; la référence distante était encore25e5. Le push identique
sansforce a ensuite réussi vers74ad. Pas de réécriture de commit ou contournement
de garde ; branches et journaux conservés.

## Recette native publiée avant lourd

[Recette argv/env/limites](preuves/ci-format18/recette-build-transaction.json).
Source74ad figée, staging neuf privé et build neuf ; aucun objet ancien ni archive
0511/diagnostic réutilisé. Compilation native sur luciepc sous poste lourd, j8,
MemoryMax32G/swap0 et scratchdur32G, réseau isolé/sourceRO, seuls build/tmp et
sorties extensions privésRW. Aucun GPU/modèle/service/PG ni lib installée.

Enveloppe globale2400s après acquisition : contrôleur2370s, compilation1800s max,
transaction540s max dans le temps restant, arrêt10s au filet de l'unité. Premier
échec inattendu = arrêt/diagnostic ; aucune hausse ou répétition silencieuse.

CMake Release/testsON/shellON/vector;geo, sanitizers/runtime-check/LTO/backtracesOFF,
grammaire et FetchContent sans mise à jour, ccache désactivé. Cibles transaction_test,
rag3db_shell, rag3db_vector_extension et rag3db_geo_extension seulement. Le shell
fait partie du bâti privé, sans activation ni publication d'un exécutable utilisateur.
Les autres extensions optionnelles ne sont pas déclarées compilées par ce plan.

Transaction_test entier sans filtre, une invocation, --gtest_fail_fast et XML.
Environnement frais explicite, MAX_NUM_THREADS2, on-disk par absence IN_MEM_MODE,
TMPDIR/RUNNER_TEMP privés ; aucun chemin de trace externe ou fixture privée rejetée.
Lire nombre de tests réellement joués, échecs, skips et disabled : les outils
conditionnés par un WAL externe ne deviennent pas verts par défaut. Toute absence
inattendue est rapportée, aucun filtre ajouté pour la masquer.

Le runtime réemploie uniquement les binaires et extensions de ce build privé,
au chemin source compilé ; les tests ne réécrivent que leurs copies/DB temporaires.
Logs/XML, cache/compile_commands, headers générés/BuildID, SHA des artefacts,
limites et preuve d'arrêt sont à exporter. Rien de ceci n'est joué au moment de
cette publication. L'ancienne trace bloquée demeure une autre action interdite,
pas un objectif que cette recette tenterait d'atteindre autrement.

## Intégration et fermeture

Après source publiée et recette coordonnée : bâti/test bornés, revue du diff et
des preuves, puis tour master explicite à toi. Aucun master/code/lib poussé par
ce document. Les3contrôles et la garde de nettoyage s'appliqueront au lot final.

Le ticket reste ouvert même si les contrôles locaux passent : l'exclusion, le
formatage et la règle de passe doivent être intégrés, puis les jobs CI concernés
réellement verts sur master. Des runs annulés/pending ou un format local vert
ne satisfont pas ce critère. Tout échec shell/Python/Rust restant sera distingué
au lieu d'être déclaré résolu par un formatage C++.
