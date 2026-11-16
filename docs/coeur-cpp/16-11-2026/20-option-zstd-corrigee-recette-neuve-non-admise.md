# Option zstd corrigée, recette neuve non admise

Refus réel de l'analyse01 et ticket publiés **avant correction** sous4185550a2a sur cx-coeur-dictionary-append-atomic (document19). GO7ec120clos. Aucun GDB n'a été lancé et ce refus ne renseigne pas le SIGSEGV.

C7 autorise le microcorrectif du pilote : **seule modification de la commande de décompression**, --output=<core> remplacé par deux arguments -o puis str(core), selon l'usage réellement acquis. --decompress, --sparse, --no-progress, entrée compressée, hash17Go, GDB, cinq pins, bornes et supervision inchangés.

Le test a été publié d16e2a4ba1 **avant sa première exécution CPU**. Il extrait par AST le véritable appel run du contrôleur, capture le vecteur sans exécuter l'outil et exige exactement /usr/bin/zstd --decompress --sparse --no-progress -o /private/core /private/input.zst. L'attendu est relié au stderr brut existant, copié constant. Ce test a effectivement échoué avant fix (1joué/1échec sur --output=), puis les9contrôlesCPU passent après correction. [Preuve et empreintes](preuves/core-ro-preparation/zstd-option-fix-cpu.json).

Le stderr constant conserve ses espaces de fin de ligne et sa ligne finale à l'identique : ne pas le reformater pour faire passer diff--check. Le diff des fichiers de code/recette/documentation est propre ; ces octets de reçu restent la contre-preuve fidèle. Les tests CPU ne sont **ni une réussite réelle zstd ni une preuve GDB**.

## Nouveau pin préparé, sans réutiliser01

[Recette révisée](outils/core-ro/recipe.json), SHA314d3f603501fa62a40a89c05456f088a58f40668031f7334f299d13dee1cc4e. Les identifiants privés passent à02 pour garantir une portée fraîche : J cx-core-ro-02, scratch cx-coeur-core-ro-02, unité cx-coeur-core-ro-02.service. Seuls ces identifiants et les empreintes de scripts/recette changent en plus de la forme d'option ; aucun reset de01, aucun ancien END repris, aucun chemin d'entrée GDB/zstd/ELF/core/header modifié.

Commande proposée après RO et GO distinct : `bash /home/lucied/.cache/rag3weaver-build/cx-core-ro-02/launch.sh`. Toujours120s total après vrai poste4G/swap0/quota, contrôleur80/unité90+5/fin native95/final110/watchdog115+5, décompression20 et GDB35 dans la même portée. Le chemin du core temporaire et celui du sysroot vide de la commande GDB suivent le scratch02 ; les options et commandes d'analyse restent identiques, sans inférieur/autoload/solib/debuginfod.

Aucun staging02, décompression ou GDB exécuté par ce correctif. Source C++ et anciennes preuves/staging01 inchangés. Nouvelle RO demandée sur ce paquet avant toute admission ; ticket de décompression, ticket SIGSEGV et C1 toujours ouverts. Pas de retry automatique, test C++, rebuild, service ou nouveau modèle.
