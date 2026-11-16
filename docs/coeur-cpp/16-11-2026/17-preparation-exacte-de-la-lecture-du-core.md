# Préparation exacte de la lecture du core existant

**PRÉPARATION, AUCUN GDB/ZSTD LANCÉ, AUCUNE ADMISSION.** C7 accepte la préparation d'une analyse seule120s/4Gio ; le GO de la paire2400s est clos. Cette recette ne lance aucun inférieur, test, build, service ou base. Elle ne corrige aucun C++ et n'attribue pas le SIGSEGV.

## Entrées et outils épinglés

[Recette exacte et commande GDB complète](outils/core-ro/recipe.json), SHA256 `9b0489196b524feabf2e4aca9870d57edd1a4956e872346bf6e232dc5b37bda2`. [Provenance RO et contrôles CPU](preuves/core-ro-preparation/provenance-and-cpu.json).

- Core système déjà conservé sur PC : cx-two-reds-01/post-run-core/core.system.zst,2387501octets,SHA77c13010a68f6ce77aabb1db849b087eb1bd8f682503490b8bd3fa568122d104. Aucun nouveau coredump.
- ELF archivé : cx-two-reds-01/raw/candidate/artifacts/concurrence,31549968octets,SHA5029277b742a1391ae570cbe2403a4dd6857b38535404d9121f281d333b01f5c. Source ed0c3c8af57f25e83ae5c4ba004ae3393e77812e, header/marqueur moteur888 Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF. Header lui-même épinglé650cec620083cd70467b41a0f2dad84620c0346b2ba5d16f135ad3591b19f577.
- /usr/bin/gdb,35318992octets,SHAe2db863afa4d39c743d5057509aa18a9a6330d5b7ccd71a63ffe70c8b862810a ; version18.1-1.1 lue dans l'index des paquets, pas par exécution de GDB.
- /usr/bin/zstd,214344octets,SHA4f8335318c5471e1b481bbb9eebdcc9cd9a31b0bcfb5e2393857d656f33160e8 ; version1.5.7-4 lue dans l'index.

Les chemins absolus figurent dans la recette. Toutes tailles/empreintes sont vérifiées sous portée avant analyse et revérifiées après quiescence, y compris après refus. Le lien core/ELF vient du PID, de la commande, de l'unité et des artefacts du lancement conservés ; aucune ligne BUILD_ID native n'avait été émise dans le cas crashé. Ne pas inventer cette attestation manquante.

## Symboles et SDK : portée honnête

Lecture Python des sections ELF, sans outil natif : .symtab1166688octets et .strtab4411044, .dynsym et note GNU présentes ; **aucune .debug_info/.debug_line**. Les fonctions peuvent être symbolisées, les variables locales et lignes source ne sont pas promises. Pas de recompilation debug implicite.

GDB charge seulement l'ELF archivé comme exécutable/symboles, jamais une lib rag3db installée. auto-solib-add=off, sysroot et solib-search-path pointent vers un dossier privé vide. Les bibliothèques du processus crashé ne sont pas remplacées silencieusement par celles aujourd'hui installées sur l'hôte ; leurs symboles peuvent donc manquer. GDB lui-même utilise le runtime de l'outil installé, avec environnement nettoyé, sans changer SDK/service. Le relevé de versions de paquets n'est pas une preuve que les bibliothèques du crash étaient identiques.

## Commandes et portée

Entrée proposée après RO/GO distinct : `bash /home/lucied/.cache/rag3weaver-build/cx-core-ro-01/launch.sh`. J neuf cx-core-ro-01, scratch extérieur neuf vide cx-coeur-core-ro, unité attendue absente cx-coeur-core-ro-01.service. Aucun staging distant réalisé par cette préparation.

Vrai poste lourd POSTE_MEM_MAX=4G/POSTE_SWAP_MAX=0, quota effectif lu, pas de forçage CPU. Watchdog extérieur120s total après acquisition : TERM115/KILL+5. Contrôleur80s, unité RuntimeMax<=90/Stop5, fin native<=95, finalisation souple110 avant watchdog. Délais locaux toujours min(cap,temps contrôleur restant-10). Tout refus arrête la recette, aucun retry.

Un namespace sans réseau, mémoire4G/swap0, tmpfs privé capacité32G pour le fichier creux. Entrées hôte/core/ELF RO ; uniquement J et home/tmp privés accessibles en écriture à GDB. Décompression20s avec commande épinglée `/usr/bin/zstd --decompress --sparse --no-progress --output=<scratch>/core <core.system.zst>`, incluse dans la même portée. Taille attendue17548656640octets et SHA15a21f6deeacb0c8c1400acd4196f230189971b7bd325703c0091d6f14d34e2f contrôlés avant GDB. L'original compressé et le core brut déjà extrait auparavant ne sont pas modifiés.

GDB35s, commande exacte en tableau dans recipe.json : --nx/--nh/--batch/--quiet, initialisations auto-load off, debuginfod enabled off, auto-solib-add off, sysroot/solib-search-path privés vides, pagination/confirm off, print frame-arguments none et print entry-values no. --se pointe l'ELF exact et --core le fichier privé décompressé. **Seules commandes d'analyse** : info files ; info sharedlibrary ; thread apply all bt12 ; info registers rip rsp ; quit. Aucun run/start/call/attach/target, script Python GDB, .gdbinit, shell GDB ou téléchargement debuginfod. DEBUGINFOD_URLS vide, HOME privé, environnement explicite identique pour Popen et env-i.

## Reçus et vérification

Commande, limites, environnement, stdout et stderr **séparés**, code/timeout/PID/wait/groupe/cgroup, hash du core décompressé et premier refus conservés avant jugement. Après arrêt de l'unité : MainPID0/ControlGroup vide, aucun processus attribué, rehash des entrées/outils et inventaire brut après fermeture des journaux. Preuve absente ou incomplète reste ROUGE, même si le watchdog empêche la finalisation. Pas de suppression des preuves du lancement précédent.

Huit contrôles CPU verts : liste exacte des commandes GDB sans inférieur, quota réel, rehash après rouge, absence de reçu jamais verte, stdout/stderr distincts, petit Python expiré/reap, groupe encore arrêté après sortie du leader et refus de préflight sans unité réelle. Bash syntax et AST Python contrôlés. **Ni décompression réelle, ni GDB, ni unité systemd d'analyse, ni natif/test/build exécutés.**

Le ticket crash publié a693b7373a et le parent C1 restent ouverts ; résultat éventuel de cette analyse à relire avant attribution. Recette/pilote à faire relire RO avant toute admission distincte.
