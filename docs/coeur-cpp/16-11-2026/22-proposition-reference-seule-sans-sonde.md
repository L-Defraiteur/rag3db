# Proposition discriminante : référence seule, sans sonde

**PROPOSITION, aucun pilote admis ni build/runtime/staging.** Après la lecture du core37f2, C7 demande de distinguer le rôle du delta append sans rejouer le candidat par habitude. Les preuves et tickets de la pile restent publiés1d5fc4f1a2, parent C1 et SIGSEGV isolé ouverts.

## Un seul facteur moteur changé

Source proposée **75a4ba4d467aa5f07ef83494f5393154ab2392c6**, moteur59ac9c86bc2b3529b590a7462ab7051e3ad26b8a. Un cas seulement : StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops, une exécution. Aucun candidat ed0 rejoué, aucun lecteur contrôlé, cas long, known_red, E2E ou autre suite.

Comparaison RO refaite : les deux fichiers test sont identiques octet pour octet à ed0c3c8af57f25e83ae5c4ba004ae3393e77812e ; seul dictionary_chunk.cpp diffère dans les chemins build_id. Aucune modification de fixture, nouvelle sonde ou compilation debug. [Recette de proposition et empreintes](preuves/reference-strings-proposal/recipe.json).

**Drapeau RAG3DB_STRING_DIAGNOSTIC absent**, comme dans le véritable argv du crash candidat50e2. Ne pas utiliser env.json, qui déclarait un drapeau perdu au second env-i. Ne pas définir le drapeau à0 : getenv non nul activerait quand même la sonde. Le correctif c8ef du transport d'environnement ne doit pas injecter une nouvelle lecture/sonde dans cette référence : construire explicitement le dictionnaire effectif sans cette clé.

Les valeurs PATH/LANG/LC_ALL/TZ/MAX_NUM_THREADS2/OMP5/OPENBLAS5 et toutes options CMake proviennent du véritable argv conservé, pas de souvenirs. HOME/TMPDIR/RUNNER_TEMP et chemins de build/sorties sont remplacés seulement par de nouveaux chemins privés. Pool configuré par la même fixture, aucune surcharge. Il reste un ordonnancement différent entre deux exécutions, qu'un test unique ne neutralise pas.

## Profil et proposition de budget

Même Release, rcOFF/stOFF/ASANOFF/TSANOFF/UBSANOFF/LTOOFF, cible concurrence_test seule, sans extensions/SDK externe. Sources et binaire RO pendant le cas ; réseau absent, scratch privé32Gio, mémoire32Gio/swap0 **comme le run qui a crashé**, pas4Gio de l'analyse du core. Pour annoncer le même profil CPU, exiger après vrai poste cpu.max500000/100000 et POSTE_FILS5 puis j5 ; si différent, refuser avant compilation, sans forcer la configuration de la machine.

Proposition900s TOTAL neuf après acquisition, configuration120/build600/cas60,120s restantes couvrant staging/export/arrêt/finalisation sous des échéances min(phase,END restant). Le build candidat avait fini en environ241s sous ce profil ; le cap600 ne promet pas cette durée. Le pilote exact doit publier ses échéances absolues/unité/watchdog et ses chemins frais avant RO/GO distinct. Aucun compteur ancienEND ou réservation actuelle réutilisé.

Exports ELF/archive/header/cache/commandes avant le cas, vérification source/test/dictionnaire, marqueur natif attendu59 dans le header/ELF, XML exact1cas achevé sans skip, logs complets et rehash après quiescence. **Avec le drapeau absent, aucune ligne BUILD_ID dans stdout n'est promise** : le pilote doit annoncer cette absence et sa preuve de provenance par source/header/ELF/chemin d'exécution contrôlé/hash ; ne pas fabriquer une attestation ou modifier le test pour la produire. Un skip, crash, timeout ou résidu est un refus, pas un vert. À tout écart, arrêt/aucun retry et conservation de core/métadonnées déjà produits si disponibles, sans GDB supplémentaire.

## Ce que cette mesure départagerait

- Si la référence crashe avec une pile comparable : le correctif append n'est pas nécessaire à cette famille de symptôme ; il reste à démontrer une cause commune, aucun acquittement général du correctif.
- Si elle rend le même rouge de valeurs : le phénomène peut se produire sans le correctif, sans prouver que le SIGSEGV et les228 contrôles ont la même origine.
- Si elle passe : une référence verte unique n'attribue pas la cause au correctif, surtout avec une course possible. Pas de clôture C1 ni déclaration de régression causale.
- Si le pilote refuse : cela ne mesure rien du moteur ; ticket/refus avant tout correctif de harnais.

Le prochain travail autorisable est donc borné et ne change qu'une source moteur. Préparation du pilote exact possible après arbitrage C7 sur cette proposition ; aucune exécution déduite de la RO précédente ou de la fin du core.
