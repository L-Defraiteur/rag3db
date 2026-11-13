# Passation lucivy — 13 novembre 2026

Passation à la demande de Lucie pour reprise le week-end. **Aucun nouveau calcul
ni GO ne doit être déduit de cette page. Tous les anciens délais sont terminés :
ils ne se réactivent pas.** Aucun nettoyage effectué pour cette passation.

## Livré et en cours

La version publiée amont est4.3.3, tag `663975adf11fc4a9d6cadf90e70ee68f6c9db075`.
Son close attendait un acquittement Shutdown précédant la destruction effective.
Un témoin CPU l’a prouvé rouge ; correctif de barrière B préparé/poussé, pas
fusionné ni publié. Barrière : fin de vie après actor+queue détruits, admission
fermée, sends engagés terminés, cleanup résistant aux paniques, callbackRAII
worker/coop/pinned et wrapper avant on_start. Close attend aussi le routeur.

B est fonctionnellement qualifié (179luciole+6core), mais sa fusion est
**retenue après mesure CPU** : débit diminué dans7/8cas. C est l’alternative
atomique destinée à réduire le coût : bitclosing+compteur mêmeAtomicUsizeCAS,
garde empruntée sanscloneArc parsend, mutex/notification uniquementclosing.
ED l’a relue favorable statiquement, pas exécutée. Aucun bénéfice de C présumé.
Aucun détenteur du writer du rouge CI historique n’a été identifié avec certitude.

## Dépôts, branches et worktrees

Remote lucivy : `origin` = `git@github.com:L-Defraiteur/lucivy.git` ; toutes les
branches ci-dessous ont été confirmées poussées (ou poussées lors de passation).
Préfixes branches ET worktrees `cx-`. Ne pas modifier les clones principaux.

| Branche lucivy | SHA complet | Worktree détenu |
|---|---|---|
| cx-lucivy-shutdown-destruction | fbd3f0a4179917e3674fa52a56ffde027831a354 | pas de worktree sur cette branche ; B préservé |
| cx-lucivy-core-barriere-temoins | 3b1661446c573ee794b918ec016c0cbff748e4ce | mêmes productionB + tests core/pinned/réponse perdue ; préservé |
| cx-lucivy-barriere-admission-atomique | fa1d0273c468b997163f3cf9d5f56bae052f522a | portable : /home/lucied/git_workspaces/cx-lucivy-shutdown-destruction (nom historique) |
| cx-lucivy-shutdown-cpu | 3b1661446c573ee794b918ec016c0cbff748e4ce | luciepc : /home/lucied/git_workspaces/cx-lucivy-shutdown-cpu |
| cx-lucivy-mesure-ancien | 663975adf11fc4a9d6cadf90e70ee68f6c9db075 | luciepc : /home/lucied/git_workspaces/cx-lucivy-mesure-ancien |
| cx-lucivy-mesure-nouveau | fbd3f0a4179917e3674fa52a56ffde027831a354 | luciepc : /home/lucied/git_workspaces/cx-lucivy-mesure-nouveau |

Ces quatre worktrees détenus sont propres au dernier contrôle. Les clones
principaux lucivy des deux postes sont sur une ancienne main ae71a0c5e2dbb6fd33edc45bf77263c8a6651d79 ;
ne pas les assimiler à la version publiée. Les modifications du principal
portable appartiennent à Lucie. Worktrees lucivy-figement-G et lucivy-close
sur luciepc sont historiques/autres porteurs : ne pas nettoyer ni modifier.

Remote docs : `origin` = `git@github.com:L-Defraiteur/rag3db.git`.
Worktree documentaire détenu portable : `/home/lucied/git_workspaces/cx-lucivy-plan-shutdown`.
Derniers docs déjà sur master : `3a5080d93bdaeba05fd333b25bdd0565b49ad18b`,
branche de travail `cx-lucivy-shutdown-preuves-4`. La présente passation et
l’arbitrage avantC sont poussés **uniquement** sur `cx-lucivy-passation-weekend` ;
son SHA est fourni au message de remise (pas de hash auto-référentiel inventé).
Les autres anciennes branches docs restent sur origin, aucun force-push.
L’arbre principal rag3db de Lucie a du travail codeparsers et lucie-tmp étranger
à cette session : rien n’y a été ajouté/commité pour la passation.

## Ticket unique et preuves

Ticket ouvert, partagé avec ED/coeur-cpp et coordination C7 :
`docs/orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md`.
Porteur lucivy pour barrière/coût ; ED cycle Rust/fixture, coeur-cpp garde native.
Ne pas créer de doublon ni fermer parce que le travail est transféré.

- RougeCPU `645b6e662c75e6d13dbb3ba5862226a2c4957866` :1test exécuté, échec attendu
  `shutdown returned before worker destruction: Some(Ok(3))`. Avant cela : refus
  scope systemd puis erreur E0046 du témoin ; conservés comme autres rouges,
  jamais réétiquetés TDD. `preuves/shutdown-rouge-*`, compilation et entrée.
- B FBD :10exactsCPU verts. `preuves/barriere-fbd3f0a/`,20SHA relus ED/C7.
- Source3b / pilote9b2be2fc2d100497f81aeb6edb7fa72e6041651d :
  **libluciole entière179/0/0**, sixcoreexacts1/0/0 chacun. `preuves/plein-3b16614/`,
 32SHA relus ED/C7, confineurVERT0/restes[]/quietfresh/sourcepropre.
  Limite : trois noms non appariables strictement aux lignesok à cause de
  nocapture entrelacé (local_execute_and_take,test_join_resume_fires_on_last,
  port_value_take_panics_on_fanout) ; résumé179/0/0 confirmé, pas trois échecs.
  Pas de libcore entière99, SDK ni CI physique qualifiés par cette passe.
- MesureA/B : `03-mesure-cout-envois.md`, `preuves/mesure-envois-fffe/`.
  96/96,210SHA relus ED, mêmesharnais/locks, ABBA, affinité20-23, vraiMESURE
  exclusif,8G/HIGHmax/j4, confineurVERT0/restes[]/quiet/sourcepropres.
  Worker :−30,9/−17,5/−40,1/+80,0% seloncas ; pinned :−20,3/−20,9/−9,3/−11,0%.
  Coût DELTAglobal (HandlerGuard aussi), aucun seuil inventé ni ingestioninférée.
  Diagnostics pinned versdevnull restent un biais (formatage/syscalls inclus).
- Simulateur du pilote streaming :4testsPython factices verts, pas Cargo.
- **C fa1d027 : NON compilé, NON joué, NON mesuré.** Juges comptageatomic et
  trois sendsadmis/cap1 ajoutés, anciens tests conservés avec thread::scope pour
  garde empruntée. ROED statique favorable, pas GO de qualificationC reçu.
  Pilotes distincts `outils/qualifier-barriere-atomique.py` et
  `outils/mesurer-envois-ac.py`, plan `04-alternative-admission-atomique.md`.

A/B et179+6 restent acquis sur leurs sources ; aucun résultat C ne les remplace.
Les proofs relatives ci-dessus sont sous `docs/lucivy/13-11-2026/`.

## Ressources, processus et délais

Aucun test/build/bench de cette session en cours. Les confineurs terminés ont
attesté PID/start sans vivant ; le contrôle poste à la passation indique
**libre,0/4lourds,aucuneMESURE**. C’est un constat daté, pas un verrou futur.
Aucun serviceGPU utilisé ; pas d’appel7970 à préserver. Aucun verrou détenu.

Cachés conservés sur luciepc (pas de purge) :
- `~/.cache/rag3weaver-build/cx-lucivy-barriere-target-13nov` :2,2G, sourcesFBD/3b,
  quiescent ; target qualification fonctionnelle.
- `cx-lucivy-shutdown-target-13nov` :129M, témoinrouge/compilation historiques.
- `cx-target-mesure-lucivy-envois-v1` :23M, mesureA/B, targetprivé.
- Logs : `cx-lucivy-shutdown-13nov`, `cx-lucivy-barriere-qualif-13nov`,
  `cx-lucivy-plein-13nov-v1`, `cx-mesure-lucivy-envois-v1`,
  `cx-mesure-lucivy-pilote-v1`, `cx-confinement-mesure-lucivy-envois-v1`.
Les caches contiennent les reçus privés terminés : ne pas publier leurs jetons.
Sources/pilotes/bruts utiles sont déjà dans les branches/proofs poussées.
Les scripts RO auxiliaires de constat/cache ne sont pas des WIP de code à
qualifier ; ne pas prendre un ancien fichier de reçu comme nouvelle autorité.

**Tous délais600/900/END de ce fil sont terminés.** Réemploi éventuel d’un target
exige provenance+quiescence fraîches et GO du couple, sans purger les preuves.

## Contraintes et décisions à préserver

Pas de nouveau calcul pendant cette passation ; pas de push master de ce rapport.
La fusion de B est retenue à cause de son coût ; C est préparé seulement.
Une fusion ordinaire du correctif autorisé requiert revue+qualification puis GO
coordination ; tag/release/crates/npm/PyPI demandent décision explicite de Lucie.
Aucun patch .cargo/registry, SDK épinglé ou dépendance rag3weaver pour maquiller
le juge. Pas de DB/index réel, pas de rerun CI historique. Le holderD17 demeure
inconnu. Ne pas supprimer le juge, affaiblir la garde writer ou ajouter des sleeps.
Pas de modèle ou outil cité dans messages de commit ; identité personnelle de
Lucie Gmail. Aucun force-push, suppression distante ou nettoyage forcé.
Avant tout retrait de worktree : gardecommune et archives d’administrations,
contrôle dépendances sous-modules, ne pas improviser prune/GC/restauration.

## Trois prochains pas (après reprise, pas maintenant)

1. Relire ticket, plan04 et ROED sur C ; contrôler branches et propreté :
   ```bash
   git -C /home/lucied/git_workspaces/cx-lucivy-shutdown-destruction fetch origin
   git -C /home/lucied/git_workspaces/cx-lucivy-shutdown-destruction status --short --branch
   git -C /home/lucied/git_workspaces/cx-lucivy-shutdown-destruction show fa1d027 --stat
   ```
   Reprendre C depuis sa branche, pas main ; relire mécanismeCAS/no-lostwake
   et les témoins de fermeture avant toute nouvelle demande de qualification.
2. Après **nouveau GO C7 exact** sourceC/piloteC, préparer reçu/logs neufs et
   cibleprivée à provenance/quiescence contrôlées ; nouveau budget aprèsposte.
   Commande intérieure candidate (ne pas exécuter seule ni avec ancien reçu) :
   ```bash
   python3 CHEMIN_PILOTE_FIGE/qualifier-barriere-atomique.py --target TARGET_PRIVE_ADMIS --logs LOGS_NEUFS
   ```
   Elle s’exécute sous helpercommun/vrai poste lourd/8G/HIGHmax/j4 ; pleinlib
   inventorié +6coreexacts, premierFAILEDSTOP, firstredimmuable et quietfinal.
3. Seulement si C est vert et après **RO/GO MESUREA/C distinct**, coordonner4CPU
   réellement libres, nouveauxP/T/S, mêmesharness/locks A/C, sans replayA/B :
   ```bash
   python3 CHEMIN_PILOTE_FIGE/mesurer-envois-ac.py --ancien WT_A_663 --nouveau WT_C_fa1 --zone ZONE_NEUVE --target TARGET_NEUF_ADMIS --cpus CPU1,CPU2,CPU3,CPU4
   ```
   Sous vrai posteMESURE exclusif et nouveau600validé, bruts avantwait/contrôle,
   ABBA96/Fence/Drop/quiet ; arbitrer coût avant intégration, aucun tag automatique.

## Restes non poussés

Aucun code ni WIP documentaire détenu non poussé à la remise. Les trois branches
miroir de mesure/qualification de luciepc ont été poussées lors de passation.
Seuls caches/targets/reçus privés et exécutables reproductibles restent horsGit,
conservés volontairement ; leur preuve utile est archivée dans les docs.
Le travail de Lucie et des autres sessions n’a pas été inclus.
