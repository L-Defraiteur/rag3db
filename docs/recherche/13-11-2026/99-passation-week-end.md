# Passation Recherche — reprise du week-end

État arrêté le13 novembre2026 sur instruction Lucie relayée par C7 : limite
d'usage, PASSATION seulement. Aucun nouveau GO/calcul/runtime, aucun push
master ni nettoyage après cette instruction. Tout le WIP possédé est publié
sur branches non master ; sources de Lucie et autres lots intacts.

**Les anciens délais et GO ne se réactivent PAS.** Les END historiques cités
ci-dessous identifient des preuves, jamais une autorisation de reprise. Au
week-end, nouveau créneau/nouveau budget/nouveau couple exact arbitrés par C7.
Ne lancer aucun des anciens lancer.py/admettre.py depuis cette passation.

## Livré : garde de provenance du target

Produit intégré master exact `9feb7cdbadef6a60568dd4fc447bafddf7986942`.
Source qualifiée `9ec421069320b2f0cca4099fa27f83059e944de9`, pilote final
`65ad0810ca3f4f1496539d5e2945e5c959b94a07`. Le landing a gardé6783entrées
horsdocs strictement identiques à cette source (modes/type/OID compris).
Poste push réel, dernier fetch attenduD7, verdict composé65ad VERT lu et
push sous ifgrepVERT ; trois contrôles lus DEUX postes aprèsfetch : status
propre, aucun rebase, origin/master==HEAD9feb. Aucune CI ancienne rejouée.

Contrat livré : cargo_magasin/F8/e2e passent par l'entrée garde commune ;
non-vide sans provenance et marqueur invalide refusés ; résolution workspace/
manifest/target réelle et restrictions offline/locked/frozen transmises aux
deux métadonnées ; reprise connue enlève empreintes locales et garde les
externes. Cargo brut externe hors de ces points d'entrée reste une limite.
Témoin HTTP cfg(test) seulement, aucune nouvelle API/service ni pose d'ELF.

Qualification SDK29 : F8canonique8/8 ; lib2308/0/66 sans filtre ; NAV40/0/0 ;
deux corps HTTPapp.js/app.css contre blobs Git (UN juge, pas deux fonctions) ;
223gabarits Git/disque/include/ELF pour les deux ELF avant/après runtime.
ELFlarge closure13 et NAV14, default inclus ; mêmes originaux/copies rehashés.
Principal SDK7exec/main PID1232946/start91647819, detachTracerPid0 puis
vrai waitstatus0 du parent1232943. Final2076.554637075s depuis START ORIGINAL
914593.915525602<4800 ; END EFFECTIF919390.748839204 sous original919393.915525602,
pause et rouge d'entrée compris, sans reset. **Ces bornes sont historiques.**

Preuves finales publiées `cdcc032dfde186f9b33fb8d9804ca50fcf2c1a4b`,
[rapport13](13-preuves-finales-union-garde-target.md).558fichiers deux postes
réhashés, manifest `cece3588719044bf461eb6f7f322eb8f9b0db524e2b63e979eb07f782522cb3d`.
RO Archi/Mémoire/ED/C7 favorables, aucun résultat CIstatique réparée déduit.
Pic15069622272/events0,5observations deviceFD,UNKNOWNowned0 ; pas absenceGPU
ou calculGPU inférés. Fin de scope et scan indépendant frais vides prouvés.

## WIP documentaire : classement final des références conservées

Bilan landing/nettoyage déjà sur master : `4df9bd95ad51cdec430c162ba599bb70af492371`,
[rapport16](16-bilan-landing-garde-et-ressources.md). Après arbitrage C7, les
3refs PC/origin698/bdff/a5e sont des ARCHIVES HISTORIQUES CONSERVÉES et leur
suppression est ABANDONNÉE DANS CE LOT, aucun-D/rename/rebase/force.

**Clôture préparée, publiée mais NON atterrie à l'arrêt** :
`a6c23af93fa60cf064f1491ac90babb45e1055c3`, branche
`cx-recherche-garde-target-reliquat-cloture-r1`, base fraîche à sa préparation
`a568adb2941dba8cc5782391a4ffb5781c3415db`.
Trois Markdown seulement : gitmv du ticket reliquat versclosed/État résolu,
bilan16/lien mis à jour, rapport C7 historique « reste ouvert » daté et
corrigé pour préciser garde9febclose.410ticketsVERT, un seul fermé ; produit
inchangé/diffcheckpropre, aucun nouveau test ni runtime. Soumis ROdocs C7,
landing COORDONNÉ encore à faire. Ne pas dire ce classement documentaire
publié sur master tant que C7 n'a pas atterri ce commit.

Cette branche de passation part de a6c23af93 et contient donc cette clôture
WIP ; **ne pas la fusionner automatiquement**. C7 rassemble les rapports.
Pour le ticket reliquat, master avant landing conserve encore le chemin
recherche/tickets ; cette branche porte sa proposition dans tickets/closed.
Le défaut produit garde est déjà clos indépendamment.

Prototype auparavant sale : exactement5scripts docs de Recherche, sauvegardés
fidèlement deuxpostes, puis commit INCHANGÉ archivé sur branche isolée
`cx-recherche-garde-target-prototype-archive-r1`,
`ccb3d90c416bbde37ee8f65176c017207de5f265`. **NON FUSIONNÉE / NON DESTINÉE MASTER**.
Aucun script de ce prototype exécuté. Propreté obtenue par archivage, sans
reset/clean/force ; W retirée après nouvelle archive+garde fraîcheVERT.

## Branches, remotes et worktrees

Remote origin conservé ; [instantané RO des références garde](passation-week-end/remotes-garde.txt)
avec SHA complets, pris pour cette passation. Toutes les branches de travail
et de preuves ont été poussées, rien à sauver depuis une W supprimée.
Le master du snapshot a pu avancer ensuite : refaire un fetch RO au retour,
sans rebase/landing automatique.

| Branche clé publiée sur origin | SHA complet | W actuel |
|---|---|---|
| cx-recherche-garde-target-integration-r2 | 9feb7cdbadef6a60568dd4fc447bafddf7986942 | retiré deux postes |
| cx-recherche-garde-target-union-preuves-finales-r1 | cdcc032dfde186f9b33fb8d9804ca50fcf2c1a4b | retiré local |
| cx-recherche-garde-target-reliquat-cloture-r1 | a6c23af93fa60cf064f1491ac90babb45e1055c3 | local conservé propre |
| cx-recherche-garde-target-prototype-archive-r1 | ccb3d90c416bbde37ee8f65176c017207de5f265 | retiré local, archive NONMERGE |
| cx-recherche-garde-target | 698b952ab89fd48ac112645650000ebae34e6742 | aucune W, ref historique PC/origin |
| cx-recherche-garde-target-pilote-r1 | bdffcef070d78f5a97cc7d2a1edbf60baef7b58e | aucune W, ref historique PC/origin |
| cx-recherche-garde-target-reprise-r1 | a5e678b05b3b139ac71432589271485b25e8bc25 | aucune W, ref historique PC/origin |

Deux W DOCS locales désormais conservées, racine
`/home/lucied/.cache/rag3weaver-build/` :
- cx-wt-recherche-garde-target-reliquat-cloture-r1 : branche de clôture ci-dessus ;
- cx-wt-recherche-passation-weekend-13nov-r1 : branche
  cx-recherche-passation-weekend-13nov-r1, présent rapport et instantané.

Pas de W garde sur luciepc au contrôle RO de passation. Aucun target garde :
`cx-target-garde-target-qualification-v1` retiré PC, absent local. Au total,
17Wlocales+7WPC retirées avant cette instruction, puis nouvelle W de clôture
et W de passation laissées en place. **Ne pas poursuivre le nettoyage.**
Trois références locales PC conservées (non-ancêtres du master rebasé pour
Git, refus-d). Les références correspondantes de l'autre poste avaient été
retirées par-d normal ; ne rien recréer/supprimer pour harmoniser.

## Preuves, rouges et archives à conserver

Racine commune R=`/home/lucied/.cache/rag3weaver-build`.

Local :
- R/cx-preuves-garde-target-v1-local et v2-local : ciblé698, rouge de profil
  puis correctif ; v1module8/1, v2module9/0. ELF688fe230...1392f préservé ;
- R/cx-preuves-garde-target-union-v1-local : rouge4d38 AVANTCargo (option-B
  du lanceur incompatible argv strict),37fichiers manifest6e1f7b6a...bf666 ;
- R/cx-preuves-garde-target-union-v2-avant-runtime : snapshotFIGÉ100fichiers,
  manifest33b8863c...b7a8, ledger6937bd93...29b7 et deux ELF, sur DEUX postes ;
- R/cx-preuves-garde-target-union-v2-final :558preuves, manifest complet plus haut.

PC : P/Q/S ciblés garde-v1/v2 et union-v1/v2 conservés ; noms exacts dans
les source.json et manifestes. Les dossiers cx-validation-* ne sont PAS les
copies locales : les copies locales ont le préfixe cx-preuves-*.
Contrôle RO passation : autorités723255/812186/1085860/1125874 toutes TERMINE,
leurs PID absents. Composés ciblév1ROUGE/v2VERT,unionv1ROUGE/v2VERT.
Aucun de ces reçus n'est réutilisable.

ELFlarge467285648octets SHA
f28a3f993af3b7d45f79d40ff94d6b86e928c60a65fe3973123ccb605d4664b3,
NAV467193136octets SHA
71d41f8b8c9a675e4c40757e5aaf40f96eebe6a060092464c72320455140c266 :
copies horsT dans preuves locales et PC conservées. Les originaux sousT
ont été retirés après archivage et qualification, ne pas les attendre.

Archives administratives fidèles sur DEUX postes :
- R/cx-archives-garde-target-integration-13nov,320fichiers manifest-final
  709537307bd67a9d3f6beead251b9c7c989185faf1c1ffb0f6b5e317f1d55fec ;
- R/cx-archives-garde-target-reliquat-13nov : nouvelles archives et rapports
  après archivage prototype, bilan97 et suivi4df ;
- R/cx-archive-prototype-garde-13nov : sauvegarde5fichiers/modesUIDGIDmtimeSHA ;
- R/journaux-cx-recherche-garde-target-13nov : rougesCPU03896, vertsCPU,
  intégration9feb, nettoyage, garde6f3copiée, scripts administratifs.

Retraits passés uniquement sur garde COURANTE SHA
6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c,
rapport neuf VERT LU avant CHAQUE W/T, quiescence fraîche, archive deuxpostes.
Deux refus Git STRUCTURELS sousmodules PC ont reçu nouvellesarchives et
nouveauxVERT puis un--forceunique chacun AUTORISÉ, aucun refusgardecontourné.
Aucun-D de référence, suppression remote, logs tiers, SDK ou service touché.

## Accompagnement RO de l'admission2b-A, lot possédé par Arbre

Ces sources/pilotes/targets sont à Arbre, pas à Recherche : ne pas les
modifier/nettoyer/rejouer depuis cette passation.

- Source ACPU affa6d03b5a57dd99810b6853783dc1a267f76b9,
  pilote c75ad3ae4b437495fb4544045894a986ac23bebb.
  Preuves33a64da41c0d68a4fd0653c71167fc8887e41f7d RORecherche favorable :
  315fichiers réhash0écart, manifest59bf3e14da0adfc99f1788ff431a6ae4d707602e32f860bb51b654019682d6ba,
  15exacts+lib1662/0/6,1662distincts/1677raw ; ACPU seulement, pas B/C/natif.
- Union source193d77cd71fa4cc1608e961ea4c8a8d3d445412e sur garde9feb,
  3patchesidentiques/8074protégés0écart. Rouge F8 dead_code horscfgtest,
  preuvese6e6b8677742c6b10efde5cbb28b27bdf19bb540,48fichiers préservés.
- Source correctif d9aa4be86c6f21665cf85c22e47134c87ab6aab8 :8cfg(test)
  ciblés champs/observateurs fixture, corps/juges inchangés ; RO favorable.
  Pilote9625RETIRÉ avant run (APPassert resté193), remplacé pard7d00dcc.
- Rouge v2 observationCargo : preuves237614f8343a196f384d2b01361a8f83e7a0b490,
  59fichiers réhashésRecherche0écart,
  manifestfd4e8bb02f79559458341a178e4c7ae740bc992d24a7e95abf773465cc572aab.
  Cache193→d9VERT,3jeuxF8 seulement ; jeu4PID1463583/start91873444,
  7champsNone/rawlenmanquant, cause NONATTRIBUÉE ; premierrouge immuable,
  aucun rcjeu4avantSTOP, aucunELF/juge/runtimeunion qualifié. Aucune lecture
  ultérieure ne peut repeindre cette tentative en vert.
- Pilote776RETIRÉ/nonlancé après2P1 : présenceCPU6absents+LD requise et
  refusOSError absorbable par outerobserve. Mon avis776 n'avait pas détecté
  ces2défauts ; remplacé par revue final4fca corrigé.
- DERNIER couple RORecherche favorable, **PAS GO d'exécution** :
  APPd9aa4be86c6f21665cf85c22e47134c87ab6aab8 / PILOTE
  4fca0dd38378d5bb31269dab76713d6bc0060ebc.
  Branche cx-arbre-couches-2b-union-observe-pilote-r1,
  Wlocal cx-wt-couches-2b-union-observe-pilote, PCGitonly annoncé par owner.
  Rapport40 et scripts couches-2b-union-observe. Neuf hashes relus,
  24fonctions hermétiques (26cas) annoncéesowner vertes, nonrejouéesRecherche.
  pidfdSTOP tous threads/capturestrictphase/presence+rawlen/CONTfinally,
  refusRuntimeErrorfatalcausepréservée et firstred immuable. NONCargo n'atteste
  aucun Cargo. Pas d'effet causal ni coûtSTOPréel qualifié.

Dernière info propriétaire : P/Q/Sv3 non consommés, pas de poste pendant
mesureLucivy, nouveau GO C7 après RO exclusivement. Ce n'est pas un constat
actuel de processus Arbre : le propriétaire/C7 doit faire son propre état
à la reprise. Son target T=cx-target-contenu-derive est désormais marqué
WTcfgd9 ; same-owner réel/FPidentiques, pasmetadata ni nouvelle invalidation
prétendue. Preuve ELFancienne Aaffa reste distincte de propriétaired9.
INITIAL918138.385510088/END922938.385510088 sont HISTORIQUES, **NE PAS LES
RÉACTIVER**. Un ancien calcul de reste ne donne aucun reçu pour le week-end.

## Tickets et décisions encore à reprendre

- Produitgarde : docs/tickets/closed/2026-11-11-un-target-repris-embarque-les-gabarits-d-un-autre-worktree.md, corrigé/intégré9feb.
- Reliquat : dans master avant landing encore docs/recherche/tickets/2026-11-13-conserver-et-resorber-le-reliquat-de-preparation-garde-target.md ; proposition résolue/closed dans a6c23af93.
- CI : docs/orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md OUVERT ; vertSDK29 ne répare pas CIstatique, pas de rerun aveugle/exclusion/réparationLucivy déduite.
- Vues2b : docs/arbre-principal/tickets/2026-11-13-appliquer-la-visibilite-des-couches-avant-les-requetes-des-vues.md OUVERT, parent ancres ouvert ; Afixture seulement, Bchargeur/snapshotscomposés et Coutils nonqualifiés.

## Ressources et verrous à l'arrêt

Recherche ne tient plus d'autorité de qualification active ni poste lourd ;
postepush9feb est terminé, aucun verrou détenu pour la passation. Ceci ne dit
pas que le poste global est libre : mesureLucivy/autres sessions appartiennent
à leurs owners, ne pas les arrêter. Aucunservice/PG/modèle de ce lot à poser
ou arrêter. Wdocs conservées, aucun nettoyage final à lancer maintenant.
Lire l'état global au retour via les primitives poste, puis coordinationC7.

## Trois prochains pas, avec commandes RO

1. Lire cette passation et l'état publié avant toute action ; vérifier les
   branches/W, sans checkout dans l'arbre de Lucie :

   ```bash
   git -C /home/lucied/git_workspaces/rag3db fetch origin master
   git -C /home/lucied/git_workspaces/rag3db show origin/cx-recherche-garde-target-reliquat-cloture-r1:docs/tickets/closed/2026-11-13-conserver-et-resorber-le-reliquat-de-preparation-garde-target.md
   git -C /home/lucied/git_workspaces/rag3db worktree list --porcelain
   ssh lucied@luciepc 'git -C /home/lucied/git_workspaces/rag3db worktree list --porcelain'
   ```

2. Transmettre à C7 le WIPdocsa6c23af93 pour RO/landing coordonné (ou constater
   son intégration), sans nouvelle fonction ni qualification. Inspecter :

   ```bash
   git -C /home/lucied/git_workspaces/rag3db show --stat a6c23af93fa60cf064f1491ac90babb45e1055c3
   git -C /home/lucied/git_workspaces/rag3db log -5 --oneline origin/master
   ```

   C7 existant :01a1cb5c-62b9-7ea0-8b7e-cf38398ece09. Aucun push master
   automatique de cette branche de passation.

3. Coordonner seulement ensuite la suite2b-A avec Arbre/C7 : relire le
   derniercouple4fca et le rouge237, obtenir état actuel des ressources,
   nouveau budget/créneau/GOexact ; ne réutiliser aucun ancienEND. Lecture :

   ```bash
   git -C /home/lucied/git_workspaces/rag3db show 4fca0dd38378d5bb31269dab76713d6bc0060ebc:docs/arbre-principal/13-11-2026/40-pilote-capture-cargo-coherente.md
   git -C /home/lucied/git_workspaces/rag3db show 237614f8343a196f384d2b01361a8f83e7a0b490:docs/arbre-principal/13-11-2026/39-arret-union-corrective-observation-cargo.md
   ```

   Arbre existant :01a1cb5d-2191-7112-82be-648e153fe4cd. Nouveau produit
   entrant : STOP/revue ; MDseules ne motivent pas rejeu. Pas service/SDK/PG/
   modèle/GPUcalcul ou cleanup d'un autre lot depuis cette reprise.

## Restes non poussés

Aucun WIP Recherche non poussé après publication de cette passation. Wdocs
conservées volontairement ; troisrefs historiques et prototype d'archive
préservés, pas une tâche de suppression cachée. Les changements présents
éventuellement dans l'arbre principal Lucie ne nous appartiennent pas et
n'ont été ni ajoutés, ni commités, ni archivés par cette passation.
