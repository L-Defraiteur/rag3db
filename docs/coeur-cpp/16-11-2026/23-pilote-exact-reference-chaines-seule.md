# Pilote exact : référence chaînes seule, sans sonde

**PRÉPARÉ, PAS ADMIS, AUCUN BUILD/TEST/STAGING.** C7 accepte la préparation de la proposition2ea7968915. Le GO du core120s reste clos. Nouveau pilote dérivé des primitives50e2/c8ef, limité à une référence et un cas, sans modification de C++ ni activation de sonde.

[Recette exacte](outils/reference-strings/recipe.json), SHA2ccafae5af5714774c5500dd02d15f87655f44f34aece8264ce0b8538530ab7b. Source75a4ba4d467aa5f07ef83494f5393154ab2392c6/moteur59ac9c86bc2b3529b590a7462ab7051e3ad26b8a, bundle privé local ~/.cache/rag3db-moteur-notes/reference-strings-preparation/source.bundle, ref unique cx-coeur-two-reds-baseline. SHA/taille bundle, scripts, deux fichiers test et dictionnaire sont dans la recette.

## Facteur conservé, facteur changé

Le dossier entier test/transaction/concurrence a le même OID sur source75 et sur le candidat historiqueed0, comme test/include et les helpers relus. Le delta des chemins moteur reste dictionary_chunk.cpp seul. Hors dossier concurrence, le candidat ajoute un test de refus append et son target CMake distinct (+85/+4 lignes), non construit par cette recette ; ne pas annoncer une égalité de tous les fichiers test du dépôt. [Preuve source/CPU](preuves/reference-strings-preparation/cpu-and-source.json).

L'environnement est reconstruit depuis **le véritable argv** du cas crashé50e2, pas depuis son env.json erroné : PATH/LANG/LC_ALL/TZ/MAX_NUM_THREADS2/OMP5/OPENBLAS5 et home/tmp privés. RAG3DB_STRING_DIAGNOSTIC doit être **absent**, même si le parent le fournit ;0 serait une activation par getenv. Le constructeur reçoit explicitement le même dictionnaire que celui journalisé et donné au lanceur, aucun environnement hôte récupéré.

Même configuration Release/rcOFF/stOFF/ASANOFF/TSANOFF/UBSANOFF/LTOOFF, BUILD_TESTS ON/SHELL OFF/SHARED OFF/extensions vides, cible concurrence_test seule. Même fixture/données et aucun nouveau lecteur ou instrument ; ses valeurs de pool effectives n'étaient pas journalisées séparément dans le cas crashé, ne pas inventer cette mesure. Les chemins privés changent nécessairement ; ordonnancement et état de charge entre deux exécutions restent des facteurs non déterministes.

## Entrée exacte proposée et budget

Après revue et GO distinct : `bash /home/lucied/.cache/rag3weaver-build/cx-reference-strings-01/launch.sh`. J neuf cx-reference-strings-01, scratch neuf vide cx-coeur-reference-strings, unité attendue absente cx-coeur-reference-strings-01.service. Aucun de ces répertoires/unités n'est créé par la préparation Git.

Vrai poste32G/swap0. La lecture du quota effectif doit donner cpu.max500000/100000, POSTE_FILS5 et j5 ; sinon refus avant unité native/compilation, sans changer quota ou fils de la machine. Le contrôleur vérifie aussi ces valeurs dans son cgroup. Sources RO, build RW seulement pour config/build, binaire/build RO pendant le cas ; home/tmp privés, namespace sans route, scratch32G.

900s TOTAL après acquisition, incluant source/bundle, hashes, export, cas et arrêt. Contrôleur780s ; RuntimeMax<=800/Stop10, fin native<=810 ; finalisation880 avant watchdog TERM890/KILL+10. Config120/build600/cas60, chaque phase min(cap,END contrôleur restant-10). Les120s hors plafonds nominaux couvrent staging/export/finalisation : si ces étapes prennent longtemps, le cas suivant est raccourci, pas la borne globale prolongée. Aucun compteur de passe précédent repris.

## Un cas, résultats distincts

Commande native : concurrence_test --gtest_filter=StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops --gtest_color=no --gtest_fail_fast --gtest_output=xml:<J>/raw/reference/strings/result.xml. Une exécution, ni candidat ni autre cas, aucun retry.

Code0 exige XML natif exact une suite/un cas achevé sans skip/error/disabled, marqueur RUN nominatif et cohérence des compteurs ; le reçu est vert pour ce seul cas. Code1 cohérent avec échecs XML est un rouge GTest complet, conservé puis arrêt. Timeout/crash/code inattendu/XML non recevable conserve la classification « incomplete or refused », code du wrapper et attente ;139/137 ne sont jamais traduits en signal natif ou OOM sans core/journal déjà produit. Aucun GDB n'est autorisé.

Le cas sans sonde n'émet pas de ligne BUILD_ID : le parseur **n'en invente pas et ne l'exige pas**. Au contraire, une émission inattendue est refusée comme écart du profil sans sonde. Provenance : source exacte propre, dernier commit réel des chemins build_id, fichiers tests/dictionnaire, header et marqueur dans l'ELF, chemin de lancement privé et hashes avant/après. Le reçu dit native_build_id_emitted=false ; cette chaîne de contrôle n'est pas une attestation stdout native.

ELF/archive/header/cache/compile_commands exportés avant assertions et lancement. Sur arrêt, le ledger conserve STARTED/REFUSED/ROUGE, aucun cas suivant. Les compteurs memory.events/memory.peak/memory.swap.peak/cpu.stat sont relevés dans le contrôleur avant fermeture du cgroup (portée unité build+cas, pas pic du seul test). Après arrêt de l'unité : MainPID0/ControlGroup vide/aucun PID attribué, rehash des artefacts présents et inventaire brut de logs fermés. Absence de preuve finale reste rouge. Core et journal déjà produits à préserver en cas de crash, sans nouvelle analyse implicite.

## Préparation réellement vérifiée

Dix contrôles CPU verts : vrai constructeur et vraie boucle avec drapeau hôte simulé1 mais aucun drapeau enfant, un seul filtre/cas/cap, GTest0/1, XML absent/sauté/désactivé/incohérent, absence d'identité runtime honnête, classification139 sans signal affirmé, premier rouge STOP, petit Python expiré/récolté et refus avant unité réelle. Premier essai7verts/3erreursSyntaxError a révélé un finally dupliqué à la composition ; fusion corrigée avant publication. Syntaxe AST, bash-n et diff contrôlés. **Aucun C++ compilé ou joué, aucune unité ou poste acquis.**

RO du paquet demandé avant admission. Un vert unique de référence ne prouve pas une régression du candidat ; un crash comparable n'établit pas la même cause C1. Ticket crash/parentC1/wrongRows228/append/plan lecteur restent ouverts. Aucun moteur59 posé, service touché ou master modifié.

## Copies privées et revue, sans admission

Staging réalisé dans le périmètre de préparation autorisé : J=/home/lucied/.cache/rag3weaver-build/cx-reference-strings-01, six scripts a202a1032a et recette2ccafae5 vérifiés. Bundle référence unique vérifié en lecture seule contre rag3db-colonnes et list-heads75a, SHA740550ff86872813f0922ed88aae7b3ce68782a6d78cb32695ca456cbc680e23. [Reçu PC rapatrié](preuves/reference-strings-staging/preparation.json), SHA02299e0b165c89d48bee9bcba48d173dd26c5c71a9df71b79371901d7077062d ; sorties bundle conservées à côté.

Scratch neuf vide, unité not-found/MainPID0/ControlGroup vide, aucun processus attribué, raw/resume/build absents. Aucun configure/build/cas, poste, réservation ou GDB lancé. Pin prêt pour admission distincte, sans capacité réservée. RO Hotfixs favorable au paquet ; métriques cgroup seulement si le finally est atteint (SIGKILL peut l'empêcher, aucune valeur inventée). Le staging ne transforme pas cet avis en GO.
