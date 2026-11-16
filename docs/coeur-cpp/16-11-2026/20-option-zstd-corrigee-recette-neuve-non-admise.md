# Option zstd corrigée, recette neuve non admise

Refus réel de l'analyse01 et ticket publiés **avant correction** sous4185550a2a sur cx-coeur-dictionary-append-atomic (document19). GO7ec120clos. Aucun GDB n'a été lancé et ce refus ne renseigne pas le SIGSEGV.

C7 autorise le microcorrectif du pilote : **seule modification de la commande de décompression**, --output=<core> remplacé par deux arguments -o puis str(core), selon l'usage réellement acquis. --decompress, --sparse, --no-progress, entrée compressée, hash17Go, GDB, cinq pins, bornes et supervision inchangés.

Le test a été publié d16e2a4ba1 **avant sa première exécution CPU**. Il extrait par AST le véritable appel run du contrôleur, capture le vecteur sans exécuter l'outil et exige exactement /usr/bin/zstd --decompress --sparse --no-progress -o /private/core /private/input.zst. L'attendu est relié au stderr brut existant, copié constant. Ce test a effectivement échoué avant fix (1joué/1échec sur --output=), puis les9contrôlesCPU passent après correction. [Preuve et empreintes](preuves/core-ro-preparation/zstd-option-fix-cpu.json).

Le stderr constant conserve ses espaces de fin de ligne et sa ligne finale à l'identique : ne pas le reformater pour faire passer diff--check. Le diff des fichiers de code/recette/documentation est propre ; ces octets de reçu restent la contre-preuve fidèle. Les tests CPU ne sont **ni une réussite réelle zstd ni une preuve GDB**.

## Nouveau pin préparé, sans réutiliser01

[Recette révisée](outils/core-ro/recipe.json), SHA314d3f603501fa62a40a89c05456f088a58f40668031f7334f299d13dee1cc4e. Les identifiants privés passent à02 pour garantir une portée fraîche : J cx-core-ro-02, scratch cx-coeur-core-ro-02, unité cx-coeur-core-ro-02.service. Seuls ces identifiants et les empreintes de scripts/recette changent en plus de la forme d'option ; aucun reset de01, aucun ancien END repris, aucun chemin d'entrée GDB/zstd/ELF/core/header modifié.

Commande proposée après RO et GO distinct : `bash /home/lucied/.cache/rag3weaver-build/cx-core-ro-02/launch.sh`. Toujours120s total après vrai poste4G/swap0/quota, contrôleur80/unité90+5/fin native95/final110/watchdog115+5, décompression20 et GDB35 dans la même portée. Le chemin du core temporaire et celui du sysroot vide de la commande GDB suivent le scratch02 ; les options et commandes d'analyse restent identiques, sans inférieur/autoload/solib/debuginfod.

Aucun staging02, décompression ou GDB exécuté par ce correctif. Source C++ et anciennes preuves/staging01 inchangés. Nouvelle RO demandée sur ce paquet avant toute admission ; ticket de décompression, ticket SIGSEGV et C1 toujours ouverts. Pas de retry automatique, test C++, rebuild, service ou nouveau modèle.

## Staging02 autorisé ensuite, sans analyse

Copies privées réalisées au mot de C7, après publication du correctif : J=/home/lucied/.cache/rag3weaver-build/cx-core-ro-02, six scripts37f2b52aa834f3359292bd0e58f2c82be4689b32 et recette314d vérifiés ; cinq entrées vérifiées/copiées dans J/inputs/revérifiées, chemins runtime de la recette inchangés. Source propre au staging, preuves01 intactes.

[Reçu PC rapatrié](preuves/core-ro-staging-02/preparation.json), SHA256 c6f413f46b7f81fe4105169b600177b7e2d1f9efec550d7c358dc6444054d609. Scratchcx-coeur-core-ro-02 vide, unité02 not-found/MainPID0/ControlGroup vide, aucun processus attribué ; raw/resume/core décompressé absents. Aucune décompression, GDB, allocation poste ou réservation. Pin02 prêt à l'admission distincte, qui reste requise. RO Hotfixs et C7 favorables au delta, sans qualification native ni attribution au crash.
