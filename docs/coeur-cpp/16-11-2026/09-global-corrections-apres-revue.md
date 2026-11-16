# Superviseur global : trois réserves traitées, revue nouvelle avant admission

Cette page remplace les promesses d’exécution f319/doc08, **restés non exécutés**. C7/HOTFIXS ont demandé ces corrections avant tout GOexact. Source native888/base685 inchangées, branche livraison propre15ab inchangée ; scripts/preuves surcx-coeur-dictionary-append-atomic. Aucun staging distant, buildglobal, appel7970 ni place PC supplémentaire pendant la fenêtre froide/CLIArbre.

## 1. Borne extérieure et finalisation

La commande donnée au vrai poste est maintenant timeout --signal=TERM --kill-after=10s 10790s python3 entry.py J : watchdog externe couvrant entry, appels systemd, scan/proc, rehash/inventaire et reçu, démarré après acquisition de poste. TERM10790 puis KILL10 forment10800 max pour cette commande ; il ne s’agit plus du seul RuntimeMax de l’enfant. Les contrôles de quota par place/32G/swap0 restent identiques, pas800%fixe.

Réserves avancées sous ce watchdog : controllerdeadline entrée+10710 ; uniténative RuntimeMax≤10720 plusstop10 et temps de préparation déjà déduit, finnative≤entrée+10730. Finalisation supervisée jusqu’à+10780, avec marge avantTERM. Les caps individuels restent120/1800/30/2400/1800/600 et e2ecommun1800, chaqueMINsolde. Aucun cap n’est additionné à10800 pour augmenter l’enveloppe.

entry englobe préflight/exécution/quiescence/finalisation en try/finally. Premier refus conservé sans l’écraser ; aucune ancienne unité ne peut être arrêtée par erreur : nom privéfixe et LoadStatenot-found exigé avant invocation. systemctlshow/stop/wait ont des timeoutsMINreste ; lectures/proc/hashchunks1Mio vérifient la deadline. Un refus de préflight avant unité écrit invokefalse/étatnotstarted/ROUGE. Si erreur tardive, reçuincomplet/ROUGE sans inventer quiescence. Si le watchdogKILL empêche toute dernière écriture, preuvefinale manquante ne donne jamaisVERT ; on ne garantit pas une publication réussie face à un arrêt forcé, on garantit que ce défaut ne peut autoriser le push. L’uniténative possède sa borne indépendante finissant avantwatchdog même si entry ne peut plus la commander.

## 2. Premier verdict et descendants

Identités examinées après **chaque phase native avant toute phase suivante**, plus export/contrôle des artefacts avant les tests. Mauvaise identité transaction ne peut plus laisser démarrer known_red/strings/e2e. Offset est unique ; transaction et strings restent fail_fast.

Arbitrage C7 explicite : known_red contient des rouges attendus et nécessite son comparateur canonique complet ; STOPaupremierVERDICTdephasecomplet, pas de prétention d’arrêt instantané au premierassert physique de cette phase. Une phasecanonique rouge arrête toute suite suivante. Aucun watcher ambigu ou fail_fast aveugle dans le banc.

Pour e2e : inventaire natif --gtest_list_tests --gtest_filter=* (aucun cas exécuté), puis deux invocations **disjointes** :

```
e2e_test --gtest_filter=*-transaction~ddl~ddl_empty.DropNodeTableReclaim --gtest_fail_fast ...
e2e_test --gtest_filter=transaction~ddl~ddl_empty.DropNodeTableReclaim ...
```

Leur temps cumulé, inventaire inclus, est borné au même1800MINreste, aucun délai ajouté. Les positifs doivent correspondre exactement à inventaire−DROP, sansdoublon/skip/disabled ; leur premier rouge arrête avant DROP. DROP exact est joué ensuite et son éventuel rouge doit satisfaire l’assertion/query historiques. UniondesnomsXML=inventairenatifcomplet, intersectionvide, DROPexactementunefois ; pas de retrait d’un test pour cacher son échec. Les noms existants peuvent contenir espaces/&/parenthèses : le parseur les préserve plutôt qu’une grammaire ASCII restrictive inventée.

finish_child traite le groupe attribué même si le leader est déjà sorti ; TERM/grâce3/KILL/récolte3, pas seulement pollNone. Le leader est pollé/récolté dans la boucle pour ne pas prendre son zombie pour un groupe vivant. Une présence de descendants du groupe après sortie du leader est une anomalie conservée. Les PID du cgroup sont comparés avant/après chaque commande : résidu horsgroupe, y comprissetsid, refuse la phase et ne lance rien ensuite ; terminaison globaleKillModecontrol-group et vérification d’état final sont requises. Aucun killparnom niPIDtiers. Gitauto-maintenance désactivée dans le staging privé pour ne pas lancer de tâche d’arrière-plan non attendue.

## 3. Exception DROP stricte

Le texte avant Expected equality est désormais vide OU seulement la localisation source connue test/test_runner/test_runner.cpp:198 et préfixe absolu de source. Tout texte arbitraire, dont poolfull suivi de l’assertionhistorique, est rejeté. AssertionFalse/True/tuple0 et querypageusage demeurent exactes, autresfailure/skip/error refusés. Source888 et fichier/ligne connues, pas une tolérance générique par nom.

## Témoins CPU joués, aucun natif global

- [Contre-exemples réels sur l’ancien code](preuves/global-receipt/review-counterexamples-cpu.json) : oldf319 accepte poolfull+assertionDROP ; nouveau parseur le refuse. Oldentrypreflightfichiermanquant quitte sans verdictfinalROUGE ; nouveauentry, même faute avant toutsystemctl, conservepremierrefus+ROUGE. Aucune unité systemd créée.
- [Premier rouge CPU de supervision](preuves/global-receipt/supervision-cpu-red.json) conservé :5/6, timeoutdupetitenfant récolté mais boucle3s inutile,3,155s pour borne attendue<2. Récolte du leader dans la boucle corrigée ensuite, pas de seuil affaibli.
- [Résultat CPU actuel et SHA](preuves/global-receipt/review-delta-cpu.json) : **13parse+6supervision verts**. Vraies fonctionscontroller extraites AST pour tee et timeout d’un petitPython ; vrai entry pourpréflightrefusé ; vraiGNUtimeout pour un enfantprivé ignorantTERM ; groupeleaderdéjàsorti contrôlé par doubleos/proc, et extrait réel de la phase transaction prouvant refus d’identité avant suite suivante. Durées inférieures à quelques secondes, pas de compiler/GPU/service/basevivante/libmagasin.
- ast.parse des scripts et bash-n du lanceur verts. Scripts/SHAsactualisés dans recipe.json. Les 10 anciensvertsCPU restent leur preuvehistorique, pas réétiquetés13/6.

Ces vérifications ne prouvent pas une passe native ni une borne physique en cas d’I/O noyau ininterruptible ; un reçu manquant/incomplet resteROUGE. Ni l’ancienGO ni les caps d’une tentative précédente ne sont réactivés. Nouvelle revueHOTFIXS+C7 et pointcapacité aprèsCLIArbre avant admissionexacte ; **aucun lancement par cette publication**. 45exclusdistincts/2helperssautés/1mesuredisablednommée restent la portée arbitrée ; C1parentcrash nonfermé.
