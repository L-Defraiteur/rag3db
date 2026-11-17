# Mémoire — passation du 17 novembre, STOP quota

**Ce rapport du 17 novembre prime les passations des 13 et 16 novembre.**
Ordre Lucie transmis par C7 : arrêt des travaux, sauvegarde et publication de
la passation seulement. Tous les coupons non consommés sont RÉVOQUÉS ; aucune
ancienne échéance ne se réactive. Aucun test, GO, health/provider, master/tag,
nettoyage ou qualification ne doit partir automatiquement à la reprise.

## État autonome et limites des constats

La session Mémoire a principalement réalisé des revues RO coordonnées pour
Archi, CP et TTY. Aucun module revu n'a été importé, parsé ou testé par Mémoire.
Les lectures de sources Git, les hachages de fichiers et les lectures SSH des
archives ne sont pas des qualifications OS. Les avis ont été transmis aux
propriétaires et à C7 ; ils ne sont jamais un GO d'exécution ou de master.

**Aucune qualification, aucun enfant/scope, poste ou verrou détenu par cette
session n'est en cours.** Aucun service n'a été démarré par elle pendant les RO
du 17 novembre. Ce constat porte sur ses propres actions ; aucun scan global
proc, healthcheck ou arrêt de ressources étrangères n'a été fait sous STOP.
Les processus/services des autres sessions restent du ressort de leur passation.

Aucun changement produit ni WIP non poussé dans les cinq worktrees Mémoire
inspectés. Cette passation est le seul nouveau changement documentaire.
L'arbre principal de Lucie n'a été ni modifié, ni commité, ni basculé.

## Travail livré : fixtures scripts, et nettoyage gelé

Le correctif historique des fixtures scripts est atterri sur master par
`b0a845bb4877328eada37951c174e3d208986630`, après le véritable ÀTOI C7 et
les contrôles d'intégration. Source historique :
`734087a4e99d54331aac2f6321df9ef6da3b6d9c`. Ne pas rejouer ses qualifications.
Le détail du retrait partiel et de ses preuves est au commit
`492745eab5c34cfdbeb7429e1bda7f7633762438`, fichier
`docs/memoire/16-11-2026/13-retrait-garde-des-fixtures.md`.

Deux tickets Mémoire restent OUVERTS, accessibles à ce même commit :

- `docs/memoire/tickets/2026-11-16-retirer-les-ressources-privees-des-fixtures-apres-atterrissage.md` ;
- `docs/memoire/tickets/2026-11-16-une-copie-de-garde-ecrase-un-chemin-preexistant-sans-empreinte-avant.md`.

**Incident à ne pas réparer par intuition :** une copie scp a écrasé le fichier
PC préexistant `~/.cache/rag3weaver-build/garde_worktree.py` AVANT empreinte de
l'ancienne version. Ancien SHA INCONNU ; aucune égalité rétroactive n'est acquise.
Dernier SHA consigné :
`6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c`.
Les outils de nettoyage utilisaient leur copie privée, pas cette copie commune.
C7 a interdit toute écriture dans le cache commun. Aucune restauration,
suppression, recopie ou nettoyage supplémentaire sans décision neuve.

Retraits déjà réalisés historiquement : 14 WT locaux, 2 WT PC, target privé PC.
Les archives sources, administrations Git et rouges sont conservés aux deux
hôtes sous `~/.cache/rag3weaver-build/archives/` :

- `cx-memoire-script-fixtures-cleanup-16nov-v1/` et manifeste extérieur
  `cx-memoire-script-fixtures-cleanup-16nov-v1-manifest.json`, SHA
  `b7d35c4e2bbf2dd67c9a6804800f612a0b681256791d15285fef2c0d3dcae3d4` ;
- `cx-memoire-cache-copy-incident-16nov-r1/`, manifeste consigné SHA
  `0a15e39b8052cb7cb5584d2073e9dadd81cfdf8b52d4ca6b5d7d29f95aa0d61f`.

Ces empreintes de nettoyage sont historiques, PAS réhachées sous STOP.

## Métadonnées Git effectivement relues pour cette passation

Les cinq WT locaux ci-dessous sont propres (`GIT_OPTIONAL_LOCKS=0`). Leurs HEAD
égalent les branches distantes effectivement relues par `git ls-remote` pendant
la passation. Tous les chemins sont sous `~/.cache/rag3weaver-build/`.

| WT | Branche | HEAD = origin |
|---|---|---|
|cx-wt-96-query|cx-memoire-segment-pile|f5a59210ac797511d97b56a1fa1dfa35f2429a77|
|cx-wt-memoire-compaction|cx-memoire-usage|1c85fa06e7def8b0bfc979de32b599e8ae4d0240|
|cx-wt-memoire-passation-weekend|cx-memoire-passation-weekend-r1|8ff6bba85927e33f761a4c8ee18c9857fd0d68e4|
|cx-wt-memoire-script-fixtures-cleanup-r1|cx-memoire-script-fixtures-cleanup-r1|492745eab5c34cfdbeb7429e1bda7f7633762438|
|cx-wt-memoire-script-fixtures-r4|cx-memoire-script-fixtures-r4|734087a4e99d54331aac2f6321df9ef6da3b6d9c|

Ces branches anciennes sont conservées, pas proposées à intégration.

PC : sous `/home/lucied/git_workspaces/`, les quatre WT gelés suivants ont été
relus par métadonnées Git seulement : HEAD détachées, status suivi/non ignoré
vide. Un status vide NE résout pas le refus historique des fichiers ignorés.

| WT PC | HEAD | Reste historique à conserver |
|---|---|---|
|cx-memoire-script-fixtures-r4|e2722d521192b3c25f9dd13e0e9aed47a7d7487f|extension/rag3weaver/target/ ignoré|
|cx-memoire-script-fixtures-resume-pilot-r1|f7ef1b0153dc5c337a9a9e3cb2e234c96a84e789|script-fixtures-resume/__pycache__/|
|cx-memoire-script-fixtures-second-only-pilot-r1|f812f1fd8024aac0ab1a70e2633d75e326c5263b|script-fixtures-second-only/__pycache__/|
|cx-memoire-script-fixtures-union-pilot-r1|01f3cf6ea9ffbf418508e8dd07b25e8233afaab6|script-fixtures-union/__pycache__/|

L'archive Git indépendante PC `cx-memoire-script-fixtures-734-proof-r1` est
également présente, HEAD détachée734087 ci-dessus, status vide. Le target PC
`~/.cache/rag3weaver-build/target-cx-memoire-script-fixtures` est ABSENT, vérifié
par existence seulement. Aucun nouvel inventaire, retrait ou contrôle de garde.

WT du présent rapport :
`~/.cache/rag3weaver-build/cx-wt-memoire-passation-claude-17nov-r1`, branche
`cx-memoire-passation-claude-17nov-r1`, base
`927941a67a5805450d49f1e04b39fdd542a6059b` (HEAD master locale observée).
La référence locale origin/master était99fef92ce6ee9a0007eaf99c7c3a6a3789061c57,
donc périmée relativement à cette HEAD ; aucun fetch ni mouvement master fait.
Le SHA final du rapport est donné au message de passation après commit/push ;
son unique diff est ce fichier. Aucun target n'est créé pour cette passation.

## File active Archi : pure acquis, runner MODELED non qualifié

Ticket propriétaire OUVERT :
`docs/orchestration/tickets/2026-11-17-les-images-executees-manquent-d-un-producteur-de-preuves-commun.md`.
Branche de travail propriétaire connue : `cx-archi-pas-va-au-bout-global-pilot-r2`,
WT local `~/.cache/rag3weaver-build/cx-wt-archi-pas-va-au-bout-global-pilot-r2`.
Les pointes de ce WT peuvent avancer pour leur propre passation : les avis
ci-dessous restent attachés UNIQUEMENT aux SHA nommés, pas au dernier HEAD.

Arbitrage C7 : résultat de l'INVOCATION avec image d'entrée confirmée, pas
garantie d'image A immuable, de PID écrivain par octet, ni inventaire exhaustif
des images/enfants futurs. P17–P20 UNKNOWN, pas condition implicite du lot anglais.
SDK chargé reste une attestation physique distincte.

### Validateur pur, preuves relues et suffisance bornée

Source `68eab84d95c8346c6dfe51ec4f3a7f0d79a093c3`, module
`execution_entry.py` SHA
`565b7f39494911b65ef121f19565450c70803c93b9ef6e62599d570707dbf218`.
Preuve `892bc4271fec9b387142954358c4110a58ff2bdd`, rapport53 : **51 PASS**, sans
FAIL/ERROR/skip, rc0, une invocation, 51 noms distincts. Réempreinte personnelle
des neuf fichiers LOCAL+PC, 0 écart ; manifeste
`003ca193ffc17c0d54eb6366aee549ce54b5a07f5e5ddc5d7d9fb1d015a671b3`.
Racine aux deux hôtes :
`~/.cache/rag3weaver-build/cx-archi-entry-pure-green-68ea-17nov-r1`.
Rouges antérieurs conservés : baseline nominale f74e7f2cb580567d0ce22c20d8ea1c9d8748e52c,
topologie2a715fd7b971a009fcf666c27ac50c5d87f344cf. Trois commandes, 55 invocations
de méthodes pour51 distinctes ; PAS55 verts. Références SHA opaques, faits de
modèle seulement ; aucun reçu OS/CP/Common réel acquis.

### Protocole OS : documentaire et inactif

Dernier delta protocole relu favorable DOC/API :
`69db61c102a0987fad43b86fa3082e03cca071be`, rapport55, dossier
`docs/rag3weaver-archi/17-11-2026/execution-entry-native-protocol/`.
Origine MONOTONIC émise AVANT watchdog ; trois pièces ledger/ELF/faits canoniques
sans cycle ; FD A après renommage du chemin est positif, vraie entrée B négative ;
ETXTBSY à qualifier sur objet privé writable avant/après, pas EACCES/EROFS ;
OwnedLaunch avant fork, gate et cleanup avant HeldEntry. Common a sa racine réelle
dbus-run-session et son vrai wait, distinct du wait enfant du runner. Liens de
clôture/Common et port CP terminal encore HOLD. PDEATHSIG/EXITKILL, stabilité,
ABI/linker, coût et ressources physiques restent à qualifier. Pas de GO natif.

### Runner d'effets : où la file s'arrête

Module `execution_entry_runner.py`, uniquement **FIXTURE_ONLY/MODELED** ; LIVE,
CLI natif et collect_common_closure réels restent unsupported. Toujours
cp_ack=false/global_admission=false/imagesUNKNOWN/SDKNOT_ATTESTED.

| Étape | Source ou preuve exacte | Résultat/avis |
|---|---|---|
|Baseline runner|f7ec23843801b1972244f493aa242977c19db593|Refus avant effets ; nominal seul proposé|
|Preuve baseline|b40a819d23ebc21a99a9fd20f9ec648fb950931e|1 FAIL exact, 23 autres non joués, neuf fichiers deux hôtes relus|
|Corps initial|5e3b2006dd20ad2f03e300b002a4132eafcd6567|Réserve M : contrôle END avant hashes finaux ; réserve snapshot attendus groupée|
|Preuve quatre rouges|b3ab49f8b7fe1b3ce5b41b727aaa8d2f988ae92c|4 FAIL exacts, 24 autres non joués ; deux hôtes relus|
|Correctif request/hash|10f7534f24e1334634801a8c924a918dec44b553|Réserve M : observations facts/RAW conservées par alias mutable|
|Preuve deux observations|b5113587e7d9dd5a2c07e8ca2a047b9756ab32c6|2 FAIL d'égalité exacts, 28 autres non joués ; deux hôtes relus|
|Correctif observations|5877852b4f6697f2da82a7b97369af7404d78e61|Avis M favorable borné source/pins ; annulé comme suffisance globale par réserve TTY suivante|
|Témoin rc wait|2df73d1dc523beecc8cfe67d07e837767150fd92|Avis M favorable témoin/pins ; UN rouge proposé, NON observé par M au STOP|

**Réserve courante TTY, à ne pas perdre :** le snapshot complet de wait peut
refuser RAW bytearray malformés AVANT décodage de kind exit/code7 valides et
effacer rc7. Témoin31 attend `observation-types` MAIS invocation_returncode=7 ;
ancienne valeurNone. Les RAW restent invalides ; ne pas réduire le contrat pour
réussir. Avis M précédent favorable587 n'annule PAS cette réserve. HOLD30 ;
aucune preuve30/31 verte reçue ou relue par M. Dernier test source connu :
`d9d61264000941cb312bdc9f72bcd944bb364bc7b6c97cde888f10eb7880adf9`, runner
`cf3e7a41fc30eabd0ef5ff1b377efbf102ac14e3443d9796e3b3f19099cb4a0e`.
Tout résultat ultérieur doit être repris depuis la passation Archi, sans
réactiver son coupon. Les callbacks sont postvalidés, PAS interrompus sous une
borne OS démontrée ; aucun backend natif livré par cette couche.

Archives runner aux deux hôtes sous `~/.cache/rag3weaver-build/` :

| Racine | SHA manifeste personnellement rehashé LOCAL/PC |
|---|---|
|cx-archi-runner-baseline-red-f7ec-17nov-r1|db4805bc3165f0a617a083121f82adea82bea3bf2033d52d07b0583c86067528|
|cx-archi-runner-snapshot-hash-red-deee-17nov-r1|60e1b1e219030c49d5980f6643f133512a4d02bcf2b5379d1d547fa9a8fef866|
|cx-archi-runner-observations-red-4117-17nov-r1|c13bb055e8ceaa1507f466e5fdf9ee701e745874223d6e49eb4d8ccd8aa43e12|

## File CP : moteur publication non qualifié, cinq rouges annoncés

Ticket propriétaire OUVERT :
`docs/codeparsers/tickets/2026-11-17-la-publication-du-ledger-de-sonde-attend-un-ack-sans-relais-autonome.md`.
Moteur unique `extension/rag3weaver/scripts/publication_relay.py`, loader commun
`publication_loader.py`, wrappers Rust ; aucun adaptateur global/Q/LIVE armable.
Les six rôles globaux et le raccord Archi ne sont pas qualifiés par les fakes.

Réserves M initiales558 : budget Common ignore le restant du watchdog de phase,
verdict final ignore hard_end local, import moteur avant fermeture. TTY a ajouté
BOOTTIME vs MONOTONIC, imports Gate/legacy/guard et grâces RPC/reap hors cap.
Rouge CA1C : neuf causes +UN FAIL de fixture (gate.pin resté ancien), PAS dix
rouges causaux. Ma RO CA1C avait manqué ce raccord de pin ; avis corrigé et
incident gardé. Correctif fixture8792769e55844113ac2a33ad48da94bebd072541 seul,
puis dix rouges causaux réellement relus, manifeste
`f7c622667ff7552402843e35a96fb79337defab907b68eb66c29f781cc86d074`.
**17 fichiers manifestés + manifeste =18 fichiers**, pas17total.
Racine aux deux hôtes `~/.cache/codeparsers-notes/publication-guards-red-8792`.

Témoin causal RPC prépublié au commit
`f3d049e4f9f3dd13aaf1fc7c05a2f6205e314caf` : compare l'OBJET erreur communicate
avec celui finalement levé, et conserve erreur reap secondaire/UNKNOWN.
Son rouge est ensuite consigné par CP, manifeste8c2fa5bc8e3b174ee98a827f665c68a3b181208624db29af4b4a82916be7f956 ;
ne pas l'appeler réempreinte personnelle M des deux hôtes (non faite pour ce lot).

Correctif groupé SOURCE
`e15db5a4a87220e2de0b38c3721e4461844c7f36`, rapport09, pins16 vérifiés Git/disque.
MIN des deux watchdogs et finalMINhard corrigés, origines MONOTONIC avant exec,
snapshots contrôleurs avant import. **RO totale M : deux réserves résiduelles** :

1. Loader hashe prepared puis rouvre read_text pour JSON : bytes B peuvent choisir
   la fermeture après validation du SHA A. UNE lecture bytes/hash/JSON nécessaire,
   primitive à réutiliser dans TOUS les lecteurs prepared, pas seulement préimport.
2. Première cause encore masquable : stop garde erreur reap puis close la remplace ;
   Relay.run peut perdre l'erreur initiale sur first-red, cleanup-refused ou
   local-stop-refused si leur atomic_json échoue hors FileExistsError. Objet
   premier à préserver, erreurs secondaires/UNKNOWN et record de secours mémoire
   à distinguer d'une publication disque réussie.

Prépublication résiduelle exacte
`6f471fe97f71b2348dc6ce72fca27439bc68063c`, branche/WT
`cx-codeparsers-publication-cause-report`, fichier
`docs/codeparsers/17-11-2026/publication-core/test_publication_residual_counterexamples.py`.
Avis M favorable borné aux CINQ témoins/pins18 : 1 prepared, 3 journaux, 1 reap→close.
Moteur2f8e7d5a5e892402efb632657b69b9a662a1386d767b6630f85d0c062d4d42bf,
loader5ee566f69c978f1444bd3b9d685145e2cfc68cc263f9ef48553780c54f593d4b inchangés.

**Dernier statut reçu de CP avant STOP, NON encore revu sur pièces par M :**
GO5 consommé une fois, cinq FAIL aux cinq causes exactes/0ERRORskip/rc1,
0.235670384s externe ; 24 payloads+manifeste=25fichiers, archive
`~/.cache/codeparsers-notes/publication-residual-red-6f47`, manifeste
`68d76af23699734d51df594d9cd71343dfbdb9d6a47f598c141d2e5fc3d1e42a`,
résultat778c1434d1c421511536bf0188c97310dc60ca4a40ea30a769fe66566786b2a7.
CP prépare rapport/ticket puis correctif groupé sur branche neuve. Aucun SHA de
correctif final reçu par M ; **HOLD58/aucun GO63**. Ne pas transformer ce message
en preuve relue, en source corrigée, ni en droit de rejouer. Le bundle historique
58=36+3+8+10+1 n'a PAS été exécuté sur e15 par cette session.

## Autres frontières à transmettre

- TTY : preuves14 modèles vertes sur fe01 validées bornées, producteur physique
  toujours absent et Qv4 INACTIF. Contrat invocation v2 accepté en DOC seulement.
  Voir passation TTY ; aucun GO3600 reporté.
- Hotfix C++ membership : dernier delta DOC e4ea441ac9 corrige ONE_ONE/FWD =
  AU PLUS une ligne, pas présence garantie. Prérequis retention historique/OID,
  MVCC/REL et migration WAL non vide restent ouverts avant code. Voir propriétaire.
- Archi reader pur6e12 et41 témoins hermétiques globalpilot a0d74 : historiques
  distincts ; ni prepared229b ni docs/JSON n'activent le global7200/D6.

## Trois prochains pas, seulement après reprise explicite

1. Lire la synthèse C7 et les passations17nov Archi/CP/TTY, puis rapprocher leurs
   SHA complets/branches réellement poussées. Ne pas faire confiance aux refs
   origin/master locales anciennes. Lire les tickets existants, sans en fermer.
2. Finir les RO en file : preuves CP6f47, éventuelles preuves rouge rcwait et
   correctif runner, puis correctif CP snapshot/firstcause. Vérifier sources et
   pins avant d'émettre un avis groupé ; avis favorable n'est jamais GO.
3. Lucie/C7 doit redonner une autorisation EXACTE et un budget/créneau NEUFS
   pour chaque exécution, master ou retrait. Réexaminer séparation MODELED/OS,
   origine préwatchdog et ressources conservées ; aucun ancien END ou coupon
   n'est utilisable. Le cache commun reste interdit d'écriture.

Commandes **INACTIVES**, exemples de lecture Git pour une reprise autorisée :

```bash
git show 2df73d1dc523beecc8cfe67d07e837767150fd92:docs/rag3weaver-archi/17-11-2026/execution-entry-native-protocol/runner-wait-rc-red-pins.json
git show 6f471fe97f71b2348dc6ce72fca27439bc68063c:docs/codeparsers/17-11-2026/publication-core/publication-residual-pins.json
git show 492745eab5c34cfdbeb7429e1bda7f7633762438:docs/memoire/16-11-2026/13-retrait-garde-des-fixtures.md
GIT_OPTIONAL_LOCKS=0 git -C /home/lucied/.cache/rag3weaver-build/cx-wt-memoire-script-fixtures-cleanup-r1 status --porcelain
```

Pas de commande CPU/native/cleanup laissée prête à exécuter. Préconditions
manquantes : reprise explicite, sources finales/pins/RO, GO exact neuf et budget
original neuf attesté par le propriétaire. Pas de nouveau calcul à la passation.

## Coordination

C7 thread01a1cb5c-62b9-7ea0-8b7e-cf38398ece09 ; Archi01a1cb70-7ec9-7d53-a7d1-2a76313d457d ;
CP01a1cb5d-1698-7ca3-b94b-30fed1adefea ; TTY01a1cb5e-981f-7903-877f-6412ccfcd2b2.
Ces identifiants sont historiques de transmission, pas une autorisation de
reprendre automatiquement. Après publication et retour du SHA à C7 : **STOP**.
