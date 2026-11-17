# Passation codeparsers pour Claude — 17 novembre 2026

## Instruction prioritaire : STOP

Ce document du 17 novembre PRIME les historiques. Arrêt Lucie transmis par C7
(thread 01a1cb5c-62b9-7ea0-8b7e-cf38398ece09) : quota, passation autonome,
aucun nouveau calcul/test/GO/health/provider/master/tag/nettoyage. Tous coupons
non consommés sont RÉVOQUÉS ; anciens END NON RÉACTIVABLES. Ne pas reprendre
sur rappel automatique. Seules sauvegarde WIP, commit/push et métadonnées RO
nécessaires ont été faits après ce message.

## Livré, en cours et non qualifié

Branche de reprise : cx-codeparsers-publication-snapshot-cause-fix, worktree
/home/lucied/git_workspaces/cx-codeparsers-publication-snapshot-cause-fix.
WIP source sauvegardé et poussé : d5b4d9764c1a5fb28d84985ffe3a5f4592cccb3e.
Rapport12 dans publication-core décrit les modifications. Son ancien renvoi aux
nouveaux pins est explicitement corrigé : publication-snapshot-cause-pins.json
N’EXISTE PAS. Préparation de pins interrompue avant création, revue totale non
sollicitée, GO63 absent, aucun import/parser/test du nouveau candidat.

Le moteur commun reste extension/rag3weaver/scripts/publication_relay.py ; loader
commun publication_loader.py, wrappers relay/gate du 17nov, pilote du 16nov.
Le WIP unifie la lecture prepared : un FD, SHA et JSON des mêmes octets, pin
indépendant conservé par tous les consommateurs. Capture de première exception
avant IO ; journals protégés, abort/stop encore tentés, secondaires conservés,
SSHTransport.stop conserve le premier reap même si close échoue ensuite.
Ces garanties sont des INTENTIONS SOURCE NON VALIDÉES. failure_record mémoire
n’est ni archive durable ni preuve de quiet distant ni reçu CP.

Le bundle test_publication_candidate.py sélectionne désormais 63 cas
(36+3+8+10+1+5). Les cinq corps résiduels sont inchangés depuis6f47 ; aucune
assertion affaiblie. 63 NONJOUÉS. Ancien candidat e15 : 58 NONJOUÉS aussi, HOLD
après RO M/TTY sur snapshot prepared et erreurs IO masquant première cause.
Le loader a de nouveaux SHA littéraux dans les trois entrées ; leur cohérence
reste à relire et épingler lors d’une reprise autorisée.

Ticket ouvert (défauts ajoutés avant diff) :
../../tickets/2026-11-17-la-publication-du-ledger-de-sonde-attend-un-ack-sans-relais-autonome.md
(chemin depuis ce dossier : ../tickets/…). Rapports détaillés : publication-core/05
à12. d09be96b44 est le rapport11 AVANT correction WIP.

## Preuves, rouges et réserves

Archives ~/.cache/codeparsers-notes/ conservées localement ET sur luciepc.
Les deux copies ont été rehashées avant le fix ; pas de nouvelle certification
à l’arrêt. Chaque archive garde commande/résultat, stdout/stderr bruts, UTC et
durée monotone, source/pins/GO. Le rc0 du driver d’archivage n’est pas le rc test.

- publication-residual-red-6f47/ : GO C7-CP-RESIDUAL-RED-6F47-17nov60 consommé,
  source6f471fe97f71b2348dc6ce72fca27439bc68063c ; 5FAIL précis,0ERROR/skip,rc1,
  .23567038401961327s. Quatre assertIs (trois journaux Relay et close après reap)
  + JSON B depuis seconde lecture alors que A vérifié. 24payloads+manifest=25.
  Manifest68d76af23699734d51df594d9cd71343dfbdb9d6a47f598c141d2e5fc3d1e42a.
  Command7177c8447291bdaf30345a67f436d0bdd52240c31493ef1c5d2807c448befd46,
  result778c1434d1c421511536bf0188c97310dc60ca4a40ea30a769fe66566786b2a7,
  stderr85d8a5ef054af69cf2a7f91955bc6a662295f5db10e2ad0b9b9cf5fb57223357.
- publication-rpc-first-red-f3d0/ : GO C7-CP-RPC-FIRST-RED-F3D0-17nov60 consommé,
  1FAIL assertIs TimeoutExpired reap != communicate,0ERROR/skip,rc1,.175979993s.
  19payloads+manifest=20 ; manifest8c2fa5bc8e3b174ee98a827f665c68a3b181208624db29af4b4a82916be7f956.
- publication-guards-red-8792/ : GO C7-CP-GUARDS-RED-8792-17nov60 consommé,
  10FAIL causaux,0ERROR/skip,rc1,.268373769s ;17payloads+manifest=18.
  Manifestf7c622667ff7552402843e35a96fb79337defab907b68eb66c29f781cc86d074.
- publication-guards-red-ca1c/ : STOP historique,10FAIL mais seulement9 causes
  prévues+1 erreur de montage pin (FAIL, pas ERROR),0skip. JAMAIS requalifier
  ce dixième comme causal. Correction fixture seule8792 après nouveau GO.
  Manifest41c73731458fd15c1003c378257a38f8a8c326adfb64c02a6c79df2a29874b93.
- publication-core-c9-cpu36/ :36PASS anciens modèles seulement ; manifest
  6fc717365b4fcf1d01476fd1e3a6ec458e4420a1d06decd9a5408609e3ee560d.
- publication-core-c9-red3/ :3FAIL causaux ; manifest
  ce154c521ded16e3fe1e9a8999a0486769f89b84dd2da35b427b76396e63573d.
- publication-window-red-7ee/ :8FAIL API absente avant helpers,0ERROR/skip ;
  manifest74fdc8cabd5d4cf96b565f19fd672ed8c631d440c85e64be624ca5b5e510c27d.
- rust-relay-green-d45aefbdf8/ :36PASS anciens modèles, aucun natif admis.

Les GO ci-dessus sont consommés : aucun retry. Les GO non consommés révoqués.
Aucun vert ne prouve la qualification globale six rôles, Q, watchdogs physiques,
SDK, origine des images ou port CP LIVE. Archi possède execution_entry_runner /
image ENTRY et sa préparation FIXTURE_ONLY ; CP ne reprend pas ce chantier.

## Processus, verrous et ressources

Aucune exécution test/native ou phase active au STOP ; aucun contrôleur OWNED à
arrêter, aucun PID/scope détenu en cours par cette session. Derniers tests terminés
avec archives avant fix. Pas de poste lourd acquis pour ce WIP, pas de service
lancé. Aucun health global, scan processus ou nettoyage réalisé à l’arrêt ;
l’absence de restes globaux n’est donc pas certifiée. Garder sources et preuves.

Native P02 historique : compile BASE rc0, Cargo33.04s, publication ACK absente,
fin ROUGE ; jamais --list/test natif/CAND/REPORT. Scope validation-rvm1cxyv,
PID1813013/start120429499 historique terminé ROUGE1, restes[] nettoyé true ;
contrôle postmortem antérieur ownedAlive[] unknown[], ne pas réactiver ce scope.
BASEfec35198b64b7e9da0496de6b01a7232fa7a8390 et CANDc1cd48750f8178e68d24a3570bc320b458cad7d1
figés deux hôtes lors des derniers relevés. Target luciepc
/home/lucied/.cache/rag3weaver-build/target-import-pointe-pile : propriétaire BASE.
Ancien target portable31GiB conservé, aucun reset. Les targets et verrous n’ont
pas été réinspectés au STOP. P05/P06 jamais armés, Q globaux jamais qualifiés.
Sources et preuves deux hôtes préservées ; ce WIP nouveau n’est PAS copié sur
luciepc, il est disponible sur origin. Pas de miroir distant inventé.

## Trois prochains pas — INACTIFS

1. Attendre instruction explicite de reprise de Lucie/C7. Relire docs/README.md,
   orchestration courante et procédure push ; examiner rapport12/ticket/diff :
   `git show d5b4d9764c1a5fb28d84985ffe3a5f4592cccb3e` (lecture seulement).
2. Si reprise autorisée, terminer pins SHA source/tests/interpréteur/cwd, relire
   tous consommateurs prepared et capture première cause, solliciter RO TOTALE
   M/TTY/C7 du pin EXACT. Aucun ancien END/coupon réutilisable.
3. Après RO et GO CPU FRAIS seulement, argv PROPOSÉ NON AUTORISÉ à ce stade :
   cwd WT/docs/codeparsers/17-11-2026/publication-core ;
   `timeout --signal=TERM --kill-after=2s 58s /usr/bin/python3.14 -B test_publication_candidate.py -f -v`.
   Préconditions : source propre et épinglée,63 attendus, archive brute exacte,
   STOP sans retry au premier motif/count inattendu. Conserver deux copies
   rehashées. Même un vert63 ne vaut aucun GO natif/global. Toute étape physique
   nécessite son propre dossier complet et un autre GO, non demandé ici.

## État Git local à la sauvegarde source

Inventaire RO des worktrees locaux cx-codeparsers-* ci-dessous ; status vide
partout et HEAD==origin observé. origin est la référence locale de suivi après
push ; aucun fetch global. Pour cette branche, le commit de passation succède au
WIP d5b4 et est poussé immédiatement ; son SHA final est donné à C7 après push.
Arbre principal Lucie non modifié. Pas de WIP non poussé attendu après clôture.

| Branche | HEAD complet = origin | Worktree |
|---|---|---|
| cx-codeparsers-language-corpus | 7c27c723e31efff5d289012c210a86eec34c916c | /home/lucied/git_workspaces/cx-codeparsers-language-corpus |
| cx-codeparsers-passation-weekend | 2753df2ceaed92b1ba01fa58b0d52195c1756dc4 | /home/lucied/git_workspaces/cx-codeparsers-passation-weekend |
| cx-codeparsers-protocol-report | 1be7dc3d9736b2f65bc8d81e8eaf7ff9cb34b3f4 | /home/lucied/git_workspaces/cx-codeparsers-protocol-report |
| cx-codeparsers-publication-capacity | 558959a8defc3a92c29e056ef697a63a7c0f9423 | /home/lucied/git_workspaces/cx-codeparsers-publication-capacity |
| cx-codeparsers-publication-cause-report | 6f471fe97f71b2348dc6ce72fca27439bc68063c | /home/lucied/git_workspaces/cx-codeparsers-publication-cause-report |
| cx-codeparsers-publication-contract | 2c8d1df84ae1ea0af6b2da58892bc3a57c7d89a0 | /home/lucied/git_workspaces/cx-codeparsers-publication-contract |
| cx-codeparsers-publication-core | c9b84168e83caaf2aa083a53ad14b93da6c4330e | /home/lucied/git_workspaces/cx-codeparsers-publication-core |
| cx-codeparsers-publication-guard-red | 8421500d9ad7b70ad79ace7674d2bc19b8510333 | /home/lucied/git_workspaces/cx-codeparsers-publication-guard-red |
| cx-codeparsers-publication-guards-correction | e15db5a4a87220e2de0b38c3721e4461844c7f36 | /home/lucied/git_workspaces/cx-codeparsers-publication-guards-correction |
| cx-codeparsers-publication-guards-fix | f3d049e4f9f3dd13aaf1fc7c05a2f6205e314caf | /home/lucied/git_workspaces/cx-codeparsers-publication-guards-fix |
| cx-codeparsers-publication-guards-fixture-pin | 8792769e55844113ac2a33ad48da94bebd072541 | /home/lucied/git_workspaces/cx-codeparsers-publication-guards-fixture-pin |
| cx-codeparsers-publication-guards-grouped | ca1c9f6e25aa3906719a31946c146758566f0758 | /home/lucied/git_workspaces/cx-codeparsers-publication-guards-grouped |
| cx-codeparsers-publication-guards-report | 33cfbed8863ffa765e0976d31181a9799275af20 | /home/lucied/git_workspaces/cx-codeparsers-publication-guards-report |
| cx-codeparsers-publication-snapshot-cause-fix | d5b4d9764c1a5fb28d84985ffe3a5f4592cccb3e | /home/lucied/git_workspaces/cx-codeparsers-publication-snapshot-cause-fix |
| cx-codeparsers-publication-window-red | 7ee63776bbbd07f16bc854054281157ba2caae53 | /home/lucied/git_workspaces/cx-codeparsers-publication-window-red |
| cx-codeparsers-rust-fix-1 | c1cd48750f8178e68d24a3570bc320b458cad7d1 | /home/lucied/git_workspaces/cx-codeparsers-rust-fix |
| cx-codeparsers-rust-probe-baseline | fec35198b64b7e9da0496de6b01a7232fa7a8390 | /home/lucied/git_workspaces/cx-codeparsers-rust-probe-baseline |
| cx-codeparsers-rust-red-1 | 056f552c122353a25c37f2feaf1d4e5006c712f3 | /home/lucied/git_workspaces/cx-codeparsers-rust-red |
| cx-codeparsers-rust-relay | d45aefbdf8a0cd4c1f2fc0a6547cc450215aae40 | /home/lucied/git_workspaces/cx-codeparsers-rust-relay |
| cx-codeparsers-rust-relay-report | 4e560b216ce148d79c82e67e50525fbec6c12d83 | /home/lucied/git_workspaces/cx-codeparsers-rust-relay-report |
| cx-codeparsers-rust-stop-words-3 | c36b3dfb3e22e1c270f9a407cf56f5e5b8c7918b | /home/lucied/git_workspaces/cx-codeparsers-rust-stop-words |
