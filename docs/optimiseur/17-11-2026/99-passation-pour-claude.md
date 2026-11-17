# Passation autonome à Claude — G / optimiseur, 17 novembre 2026

**ARRÊT Lucie pour quota, transmis par C7. Le 17 novembre prime les historiques.**
Aucun calcul, test, health, provider, nouveau GO, master/tag ou nettoyage à
reprendre. Tous coupons non consommés sont RÉVOQUÉS ; tous anciens END/leases
sont non réactivables. Ne pas reprendre sur un rappel automatique. Cette
passation sauvegarde l'état ; elle n'autorise aucune exécution. Une nouvelle
consigne et des GO distincts seront nécessaires après reprise par Claude.

## Où reprendre sans dépendre de la conversation

- Session G : `01a1cb5c-e3e0-7762-a600-544203879316` ; coordination C7 :
  `01a1cb5c-62b9-7ea0-8b7e-cf38398ece09` ; relecture R :
  `01a1cb5c-d8c7-7182-9712-a78a9c43d24d` ; Arbre :
  `01a1cb5d-2191-7112-82be-648e153fe4cd`.
- Worktree courant portable :
  `/home/lucied/git_workspaces/cx-optimiseur-psql-runner-source-20261117-r1`.
  Branche homonyme `cx-optimiseur-psql-runner-source-20261117-r1`.
- HEAD/source documentaire avant cette passation :
  `af6b1558f41c0def090ef92337030b06094d7128`, égal à la branche distante
  **réellement relue par git ls-remote**. Status propre avant rapport.
  Aucun code WIP non poussé ; seul ce rapport et sa mention au ticket sont
  ajoutés au commit de passation. La SHA finale est celle du commit qui ajoute
  ce fichier, communiquée à C7 après PUSH et égalité HEAD/origin vérifiée.
- Ticket PARENT OUVERT :
  `docs/recherche/tickets/2026-11-16-la-garde-des-comptes-en-blobs-reste-a-302-documents-pour-301-lignes.md`.
  Chercher les sections chronologiques du 17 novembre ; ne pas créer de doublon.
- Lire `docs/README.md`, règles/procédure de push, suivi C7 du **17 novembre**,
  puis `docs/optimiseur/17-11-2026/psql-host-runner/` :
  `plan.md`, `runner19-plan.md`, `native-contact-plan.md`, `runner.py`,
  `capture_evidence.py`, tests et recettes. Ces sources vivent sous docs,
  sont de l'outillage PRIVÉ de qualification, pas du produit intégré.
- Aucun fichier produit, manifeste de Lucie, lib/SDK, service ou originalDB
  modifié par les lots psql. Aucun push master ni tag dans ces lots.

## Livré, acquis et frontière qui reste fermée

1. **Bundle psql HOST privé**, source exécutée
   `08d436ed2c8f75b10923508d7265c2cb7c6032ac`, rapport branche bundle
   `5a9588b90e9749b1a53a36554a3c8d198a3175ae`.
   Une seule extraction depuis image locale postgres17-alpine immuable
   `sha256:b0f9560a2de083e2cc7382e75f808c7381a32852a7ec49117deedb300e552b24` :
   container STOPPÉ, network none, jamais start/PG. Container attribué supprimé
   avant exécution CLI. `--list`, `--version`, `--help` rc0 ; PostgreSQL17.11.
   Programme psql SHA256
   `c4e48eba81b1b9e2a8ab11347afdce0ffc9eb8df41c87e4e694dce3ab79cadb5`.
   Entrée noyau **chargeur privé**, distincte du programme, SHA256
   `38d022ce7425ff105ccfb53598f606e6e5f5f0a34bfbc793d65e6f34c9d72806`.
   Ledger fsync AVANT les trois exécutions :
   `824b5913054898e735d88980cd9eb9505dbb0364d8384bfdcadf49c2d076c4a6`.
   299fichiers/12574694octets ; 356preuves deux hôtes et RO R indépendante
   portable. ISA/ABI/CLI seulement : ni transportSQL, ni SSL/auth/dlopen/locale.
   Les15libs sont étayées par ELF/--list/SHA, PAS15maps physiques observés.
   Le label musl PT_INTERP peut désigner le chargeur lui-même, pas un mappinghost.

2. **Runner fixture SOURCE + fix3**, source de fix
   `d787d6e69850e7f86ea1f324de97be26a8528aea`, recette CPU
   `691eea025b7465f27b1d221ae6b09e9aeb058ee3` : huit raccords fake/FS verts.
   Première cause et cleanup distincts ; tout refus de capture garde record ;
   consumer exige sidecar/completion ET TERMINAL EXTERNE admis après Common.
   Pièce writer seule ou absence de refusal ne suffit pas. Tests vrais FS,
   capture fake, objets synthétiques ; pas un producteur terminal réel qualifié.

3. **Helper d'archives corrigé**, source
   `3a6fb3a0c986fdbee00d28c77383f4080fb8ef00`, helper SHA256
   `76d72ff39f38ba4e39b31e75ff970b3d12e8e642c30a973e63e1e8bbe4a34daa`.
   Quatre rouges fakeIO puis quatre verts acquis : CaptureArchiveRefused garde
   first_error/record OBJETS initiaux, archive_error secondaire et chaîne ;
   archive_complete=False, aucun RAW inventé quand le FS refuse.

4. **Les19 du runner**, source exécutée
   `7d0a8af15af17d14ee975478fdfa96a4d980584d` : 19/19 verts, noms verbose,
   0FAIL/ERROR/skip.12sans enfant et7VRAISPython privés +1détenteur EOF borné350ms.
   PID/start/exe/Pythonpin/FDparents/reap/caps/RAW contrôlés, TERM_READY observé
   puis SIGKILL=-9, floods64KiB refusés, EOFhérité partiel. Holder Z explicitement
   conservé au test ; pas déclaré absent dans le test. Les7captures ont leurs
   propres contrôles contemporains ; Common atteste sa terminaison/quiescence.
   Le relevé OWNER ultérieur de8originaux absents est APRÈS CommonEND
   (1287546.151 >1287381.424), PAS une preuve rétroactive dansEND. R le borne ainsi.

5. **Contrat parent d'attestation natif PUREMENT SÉMANTIQUE**, source
   `65447d0aaaa7e7edb1fc6950c9271fb19e3a6f46`, runner SHA256
   `aec264c03df3c864d311ba006aca783e75f03a661db5b91c0aab491d074c0b2a`.
   Quatre rouges avant fix, base valide prépubliée, puis23fakeCPU verts :
   2positifs identité/postcheck,16mutants isolés,1canariPUBLIC et4anciens
   contre-ex inchangés. Ces4anciens refusent maintenant pour schéma absent ;
   les2positifs/mutants portent la discrimination fine. PID/start/exe, stage
   held/alive typé, cgroup, env exact, mapsSHA/dev/inode, programme ET chargeur,
   liaison RO et temps bornés sont contrôlés comme DONNÉES SYNTHÉTIQUES.
   Aucun constructeur PsqlSession réel, aucune bibliothèque/secret/PG ouverts.
   Cinq corps profile/session/capture/observe/consumer sont inchangés et listés
   dans `native-carry-functions.json` : pas de rejeu19 par réflexe.
   R a relu source/paquet13checks et preuves LOCAL21/21,120006octets conformes.

**NON LIVRÉS / NON QUALIFIÉS, à ne pas rebaptiser « finis » :**
collecteur OS pendant la pause vivante, handshake stdin progressif après RO et
avant identité, comparaison psql/env réels, outerwhole5 avant imports/pins et
jusqu'au terminal, vrai producteur terminal Common externe, factory/lease
opérationnelles, SSL/auth/dlopen et transportPG. Le `capture` actuel envoie le
SQL d'un bloc et ne produit PAS native_attestation : le chemin réel reste HOLD.
`PsqlSession` requiert désormais common_cgroup stable et preuve complète ;
`native_attestation_checked` ne signifie que contrôle sémantique, pas preuveOS.
Le script `runner.py` se termine SOURCE ONLY, pas une entrée live.

Le validateur/publisher **v1** est gelé :
`docs/optimiseur/16-11-2026/pg-ro-attestation/pg_receipt.py`, SHA256
`de69c498b1ac38d80d4976144fea79ddbfa7d3a34ea8ce7dae1d9e4a457aeed6`.
Il impose5433/rag3weaver_test/user=owner rag3weaver. La fixture distincte est
15433/g_pg_probe_<nonce12>/g_pg_observer NONowner. Pas d'alias ni de grant v1
obtenu en changeant des noms. SQL constants/primitives sont réutilisés figés.

## Preuves et rouges — localisation exacte sur les deux postes

Sous portable ET luciepc :
`/home/lucied/.cache/optimiseur-notes/psql-host-runner-20261117-r1/`.
Les racines exécutées ci-dessous sont PC ; au portable, copies fidèles suffixées
`-proof`, archives `<racine>-proof.tar`. Le manifeste SHA couvre taille+SHA
fichier par fichier ; bruts autorité Common originaux restent PC PRIVÉS,
les copies publiques sont sans jeton. Ne jamais copier/afficher les jetons.

| Racine PC | Scope | Résultat réellement joué | Fichiers du manifeste | SHA256 manifeste identique deux hôtes |
|---|---|---|---:|---|
| red-baseline-02 | validation-cniyids7 | 1FAIL métier attendu,0ERROR,rc1 |16|e7c4dd95f00a95040c593ab90f95bca3d513a29e13cc50238cd3f3da17d403dc|
| regression-red-03 | validation-3xm6hlp5 | 4FAIL causaux attendus,rc1 |19|d42e78e677012e4cee5e6abdbda20d600bff74f03c5c2b33b7c1cc41b50df86a|
| fix-cpu-04 | validation-x2mj1p0o | 8fakeFS PASS,rc0 |19|86ced1c90c10c83e51121e2772815b82a0344160cb3d1d802188c3b0fd8b3350|
| archive-fakeio-red-06 | validation-2ag211uv | 4FAIL cause perdue,rc1 |20|13996ea3dd97b346c26cc36a9c8909129b99580e96a016de9bc31ca5d7818e2e|
| archive-fakeio-fix-07 | validation-aif6ktio | 4fakeIO PASS,rc0 |20|10fe1584b2a35502c73fbded5b442b26c2564ecee95f43b07a36730daf548732|
| runner19-fixed-cpu-08 | validation-falzqtsw | 19PASS dont vraisPython,rc0 |56|a5b29479956b57ddb1ba44e6074fe111d480a41c5e87618550f24e68f1707cc5|
| native-attestation-red-09 | validation-ej17nv6e | 4FAIL frontière manquante,rc1 |20|2e668f9b5a2b66197e50dc0bb81586638d3b3ee3f4b2ce0acd3a79ae7d0b008c|
| native-contract-cpu-10 | validation-a5rkgdd5 | 23fakeCPU PASS,rc0 |21|d02ae81efec60f7c948e7ff5ba0be6de1e6495c098759c015eed5a09977a21ee|

Dernière archive native-contract-cpu-10-proof.tar SHA256
`6186ac5776991a2f36a087691a33e255d95197908438d8105b7db0adc61efd2f`.
Chaque entrée a result/entry-start/entry-end, stdout/stderr, recette, sources,
common-public et manifeste. Le worker19 ajoute child-proofs/7cas et
ownership-after.json. R a réhashé les preuves verts8/4/19/23 sur portable,
source/paquets d'entrée sur PC. Sa preuve indépendante du rougenatif c90 était
encore pending au dernier échange ; ne pas l'inventer. G a vérifié ses deux copies.

Bundle : PC
`~/.cache/optimiseur-notes/psql-host-bundle-20261117-r1/bundle-b630342a7eb1/`,
portable même parent `/bundle-b630342a7eb1-proof/`, 356fichiers,
manifestSHA256 `c3f52706ab8488ea3e62edfd12dc4327aaed8ede7ff767988521977b0bcb7023`.
L'audit copie01 avait refusé `No such`/`no such` Docker ; brut initial MANQUANT
explicitement conservé comme limite, diagnostic suivant et audit02 distincts.
Aucun ELF/extraction rejoué pour « réparer » cette preuve.

**NON JOUÉS** : importrouge c604/red-01 (refusé avant entrée car erreurinfra),
ancien paquet runner19-cpu-05 (helper réservé, jamais exécuté), tout contactPG,
tout vrai constructeur PsqlSession/PGPASSFILE, collecteur/handshake/terminal réel,
nouveaux tests ou calculs après STOP. Aucun fichier held-capture/launcher nouveau
écrit : la réflexion sur ce raccord n'était pas encore une implémentation.
Les commandes dans les anciennes recettes sont HISTORIQUES, pas des ordres.

## Worktrees, branches, diff et targets à conserver

Contrôle de passation RO : les dix worktrees portables suivants sont propres,
HEAD=origin-tracking et refs distantes effectivement relues par ls-remote.
Préfixe worktree : `/home/lucied/git_workspaces/`.

| Suffixe worktree | Branche distante origin/ | HEAD complète avant rapport |
|---|---|---|
|cx-optimiseur-psql-runner-source-20261117-r1|cx-optimiseur-psql-runner-source-20261117-r1|af6b1558f41c0def090ef92337030b06094d7128|
|cx-optimiseur-psql-host-bundle-20261117-r1|cx-optimiseur-psql-host-bundle-20261117-r1|5a9588b90e9749b1a53a36554a3c8d198a3175ae|
|cx-optimiseur-pg-v2-preparation-20261117-r1|cx-optimiseur-pg-v2-preparation-20261117-r1|a28f7a4855d160a189de7a4711813bfabf3c12f3|
|cx-optimiseur-pg-ro-attestation-20261116-r1|cx-optimiseur-pg-ro-attestation-20261116-r1|99b11dee22ff10547cb80e0dc9385d5e1410d072|
|cx-optimiseur-bascule-16|cx-optimiseur-bascule-16|fe02bf89206daa57e2bc55490bbb5b6a6fdfb75a|
|cx-optimiseur-return-generator-16|cx-optimiseur-return-generator-16-r2|57fbd9aa56f4b7caeacb891388f3f10cbbb39d71|
|cx-optimiseur-stack-guard-16|cx-optimiseur-stack-guard-16|d5f8182dac9dd15be44117539ffde1dd7ae2684c|
|cx-optimiseur-stack-guard-16-candidate|cx-optimiseur-stack-guard-16-r2|df2a5789888d16247f0dad42dcdaa3c4ceea2094|
|cx-optimiseur-passation-weekend|cx-optimiseur-passation-weekend|13e69050a46528aedc21ade49c0025600a655168|
|cx-optimiseur-banc-7882|cx-optimiseur-banc-7882-2|96d80615d554079cd192f6a8d043d60753bd35db|

La ligne courante avance seulement du commit DOC de passation ; SHA finale et
égalité distante sont vérifiées après push, données à C7. Pour la retrouver :
`git log -1 --format=%H -- docs/optimiseur/17-11-2026/99-passation-pour-claude.md`.
Diff du lot : exclusivement docs/outillage privé/ticket, aucun produit. Aucun
rebase en cours à conserver, pas de source code non poussée à rattraper.

PC worktrees G relus propres (détachés, pas à faire passer pour les sources
récentes des paquets psql) :

| Chemin PC | HEAD |
|---|---|
|~/.cache/rag3weaver-build/wt-lib-arbre|3177d2489ef39769e1ed259b5e4bc9ca33f1cedc|
|~/git_workspaces/rag3db-controle-G|3177d2489ef39769e1ed259b5e4bc9ca33f1cedc|
|~/git_workspaces/cx-optimiseur-bascule-16|68590270e21d00285b29800bd9cdecf0cb921461|
|~/git_workspaces/cx-optimiseur-stack-guard-16-pc|57fbd9aa56f4b7caeacb891388f3f10cbbb39d71|

Les nouveaux lots psql n'ont PAS de worktree Git PC ni de targetCargo : leurs
COPIES SOURCE/PINS/PROOFS privées PC sont les racines ci-dessus. Ne pas reconstruire
un worktree PC et prétendre qu'il a servi aux preuves. Sur PC, le target historique
`~/.cache/rag3weaver-build/target-cx-optimiseur-bascule-16`, l'archive
`~/.cache/optimiseur-notes/bascule-16nov-927/release-archive` et les inputs privés
`~/.cache/optimiseur-notes/pg-rust-tool-qual-20261117-r1/inputs` EXISTENT UID1000.
Pas de taille/du/recompilation ici. Au portable, target-cx-optimiseur-bascule-16,
target-G/hote-lucie et target-G/hote-lucie-tests sont ABSENTS au contrôle stat.
Ne rien effacer/recréer, ne pas partager/écraser un target ou ses deps.

## Autres chantiers G et données : préserver les frontières

- **Rust v2 GELÉ**, branche a28f ci-dessus, docs03/04 du17nov. Pas de tests/fix
  supplémentaires sur chemin critique depuis arbitrage C7b9139349d2. Nouveaux
  témoins v2 et test JoinHandle source NON JOUÉS. Le défaut Python status typé
  (1/1/False) et les réserves de raccord/admission restent documentés au parent.
  L'ancien ELF Rust B et ses parser tests ne qualifient pas v2, le fixture profil
 15433 ni le transport ; aucun réemploi sur de vrais identifiants.
- **Fixture PG02 historique EXPIRÉE**, root PC
  `~/.cache/optimiseur-notes/pg-v2-fixture-20261117-r1/allocation-032a992c3cd6/`.
  Nettoyage historique attesté au monotone1270310.630210527,
  avant ancienEND1270341.372028805. CID27cf/network/secrets propres supprimés,
  timer ensuite not-found/inactive/dead dans ses preuves. FinalSHA historique
  `5c050dbebf2e3dc07f0d7e7b503fb90ac63104c6532539b453c8720ab22bb6ec`.
  Aucune réallocation/prolongation/restauration ici, aucun nouveau Dockerinspect.
  État issu des preuves, pas un nouveau health/scan aprèsSTOP.
- **Bascule16** : source927941a67a5805450d49f1e04b39fdd542a6059b,
  host0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288,
  SDK59ac9c86bc2b3529b590a7462ab7051e3ad26b8a,
  libb7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101,
  vector1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561.
  Ce sont des pins HISTORIQUES de pose, pas étatservices actuel. Lire
  41a75ba35eec64af842c2120b73ca83cf103bd14:
  docs/optimiseur/16-11-2026/01-plan-de-la-bascule.md.
  COPY02 ROUGE sur469debt ; aucun CLI original/restore data par G. ABORT/reveil
 7970 d'alors est historique. Aucun GO7970 actuel, aucun ancien handle/END réarmé.
- **Retours** : source posée historique e51b0d0c5bda791a62f39910a8872f43f7912468,
  rapport04 au commit57fbd9aa56f4b7caeacb891388f3f10cbbb39d71. Bundle deuxpostes
  `~/.cache/rag3weaver-notes-A/return-generator-b8d7579797/`, ancien préservé.
  InstantanesSHA0cecea650a9db3dbe9dbe57159f63372339dd99cb35ef2b0d5909cb99eec9437 ;
  générateurba27110ea947697aa432f84ed5c8e9be9d06878717d9c5dd2738cab10c9e8cd4 ;
  restoreV2e1fd96b25c1cc61b70507c59f7f203acb21936e5b676f68c74b88d216f923854.
  Sources/tests posés historiques ne constituent ni retour frais ni GO restore.
  Respecter tous snapshots/RETOUR/états-après/anciennes preuves et72h minimum
  après chaque opération concernée. Aucun nettoyage de copies ou ancien index.
- **CLI99/trois bases Arbre** : choix préparation
  99fef92ce6ee9a0007eaf99c7c3a6a3789061c57/SDK59, pas alias de927.
  G a fourni refs exactes et RO SOURCE326b62544b, pas un build/GOA/pose.
  Sources build sous docs/optimiseur/scripts/ existent mais vieux lanceurs ont
  checkout/targetpartagés/copie/HTTP : NE PAS les lancer. Lire le rapport Arbre
  actuel/C7 pour suites parallèles ; leur état global n'est pas réattesté ici.
  Les acquis ED6add/fichiers de tests ne sont pas une preuve CLI99 runtime parG.
  Aucune fenêtreoriginal, snapshot, migration, service ou oracleMatched autorisé
  par cette passation ; chaque base exige gates et retours frais, pas vieux733.

## Processus, verrous, services et ressources au STOP

Aucune exécution G encore active connue ou à reprendre. Les huit scopes runner
EXÉCUTÉS relus en metadata au STOP sont tous autorité `termine`, nettoyé=True,
restes[] ; les scopes rouges restent rouges, ils ne sont pas maquillés.
Dernier scope : validation-a5rkgdd5 (23fakeCPU), déjà fini AVANTSTOP. Les sessions
exec d'exécution sont rendues. Pas de calcul à tuer, aucun signal envoyé au STOP.
Les7Python+holder19 avaient leur preuve owner d'absence ultérieure archivée,
pas des PID réutilisables à signaler. Aucun verrou G attendu après ces scopes
finis et poste rendu ; **état global poste --qui NON réinterrogé**, ne pas en
déduire absence de travaux d'autres sessions. Pas de nouveau slot acquis.

Les services Lucie/7970/sockets/tunnels/7878/7879/GPU/PGpartagé5433 sont
**NON RÉATTESTÉS ACTUELLEMENT** : aucune commande service/health/proc globale
pour cette passation. Aucun d'eux modifié par ces lots psql. Ne pas les arrêter,
réveiller ni sonder pour « confirmer ». Préserver ressources, fixtures/proofs,
worktrees, targets et magasins. `.env`/`.vault` jamais lus ou copiés ; aucun
secret dans cette passation, aucun passfile réel créé par le runner.

La dernière lecture avantSTOP était documentaire GitHub PostgreSQL REL_17_STABLE :
mainloop.c540–650 blob1a3d0b6b06937a4837d7164a2899f18e94f1fe2c et
common.c1490–1565 blob3323f787c35bf072d92b623774c4760e335c6728.
Elle N'A PAS établi le flush/handshake stdin de psql, ni la correspondance exacte
avec le bundle17.11. Aucun nouveau code HeldInput/collecteur écrit ; ne pas
promouvoir la réflexion de conversation en fonctionnalité livrée.

## Trois prochains pas — commandes INACTIVES, nouvelles préconditions

1. **Prise en charge Claude + consigne actuelle** : lire ce rapport depuis la
   branche, le ticket et le suivi17nov ; confirmer la révocation de tous GO.
   Lecture seule, commandes ci-dessous INACTIVES tant que reprise non donnée :

   ```text
   # INACTIF : git -C /home/lucied/git_workspaces/cx-optimiseur-psql-runner-source-20261117-r1 log -1 --format=%H -- docs/optimiseur/17-11-2026/99-passation-pour-claude.md
   # INACTIF : git -C /home/lucied/git_workspaces/cx-optimiseur-psql-runner-source-20261117-r1 status --short --branch
   ```

2. **Préparer SOURCE handshake/collecteur/horloge/terminal**, sans contacter PG :
   revoir ce qui manque dans native-contact-plan, vérifier le protocole/flush
   primaire17.11 sans le supposer, définir primitives qui gardent psql vivant
   après RO avant identité, vraie observation owned PID/start/pidfd/exe/maps/env,
   outer5avantimports et producteur terminal externe APRÈS Common. Sources,
   positifs/mutants et pins avant tout essai ; garder primitives déjà acquises.
   Pas de nouvel observer Common forké. Commande documentaire INACTIVE :

   ```text
   # INACTIF : git -C /home/lucied/git_workspaces/cx-optimiseur-psql-runner-source-20261117-r1 show 65447d0aaaa7e7edb1fc6950c9271fb19e3a6f46:docs/optimiseur/17-11-2026/psql-host-runner/runner.py
   ```

3. **Nouvelle qualification, seulement après nouvelle RO TOTALE et GO C7** :
   aucun ancien argv/nonce/END repris. Paquet/racine/proofs neufs et outils/pins,
   OS/POSTE32G/Common/caps/cleanup exacts à publier ; tests de vraie capture sur
   Python contrôlé avant clientPG. Puis, SEULEMENT client réellement prêt et
   GO CONTACT distinct, fixture nouvelle juste à temps, nouveaux OID/nonces,
   lease complète et terminal dans originalwhole5. Tout inconnu/rouge STOP0retry,
   aucune augmentation de cap pour cacher un rouge. Template NON EXÉCUTABLE :

   ```text
   # INACTIF : <POSTE32G neuf> <borneOS du NOUVEAU GO> <Common sourcePIN> lancer --delai <cap admis> --preuves <ROOT NEUF> -- <PYTHON PIN> -B <PILOTE NOUVEAU PUBLIÉ ET RELU>
   ```

Aucune de ces lignes n'est un GO, ni un rappel à exécuter. Rapport de13nov et
plans16nov sont historiques ; seul un nouveau périmètre après cette passation
permettra à Claude de continuer. C7 fait la synthèse et décidera la suite.
