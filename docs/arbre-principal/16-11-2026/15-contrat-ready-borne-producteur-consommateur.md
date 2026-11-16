# READY borné : couple préparé, aucun nouveau GO

Ticket commun C7 publié avant correction : [grant tardif](../../orchestration/tickets/2026-11-16-un-grant-est-publie-apres-la-pause-ready-faute-de-borne-explicite.md),
repris octets identiques depuis be3a4d260e. RougeB5 figé
2e38a947650d639b2b37aecd11f8514f427421ea, branche
cx-arbre-cycle-b5-rouge-preuves-20261116-r1 ;65preuvesPC/local conformes
cddb26214c6c0176c96566a091f09102b8415926691ffbe8f01060556221ec03.
Grant tardif révoqué, aucun writer ou contactprovider du pilote B5.

## Contrat commun

Arbre porte READY/consommateur, Embar le producteur. Les deux importent
lifecycle_protocol.py exact98ab2be6259e02052cdd46f982539c030347a51e,
SHA c67f072ba0134f97be6150547d80b8a23174937f2c964b0bdc30bec5e0309ec0.
Le producteur vérifie ce SHA à l'entrée et avant link.

ready_bounds(observation,END) calcule UNE fois grant_deadline_monotonic =
MIN(observation+120,END−500). ready_observed_monotonic est prélevé sur
CLOCK_MONOTONIC@luciepc **avant** écriture/fsync/publication, pas une heure
d'achèvement autosignée. READY contient aussi ENDscope, nonce, SHAmanifest/
graphe/digest et phase(job/pilot/allocation, autoritépath/PID/start/CG/UID/
scriptSHA, firstredpath). Admissiongrant, leasevaliduntil et capappel/END
sont trois bornes distinctes. Aucun délai depuis le temps d'un message.

read_phase_state lit deux fois autorité/processus et lstatfirstred, refuse
incohérences/UIDétranger/erreurs opaques ; retourne un snapshot daté SANITISÉ,
jamais le token d'autorité. Lire ce snapshot D'ABORD, prendre clock() ENSUITE.
Contrat pur : tempsfinite/typePC, domaineidentique, PIDstart/CGUIDscriptSHA,
autoritéactive/snapshotstable/firstredabsent. validate_ready_phase ajoute
maintenant<grantdeadline ; validate_grant_phase ajoute nonce/SHA/noembedding,
observationgrant dans fenêtre et leasefuture<=END. validate_phase_authority
maintient le core pendant une lease ADMISE à temps, sans rouvrir l'admission.

## Raccords réels

READY est écrit en pending neuf via q.save(openx/flush/fsync), borne vérifiée,
puis os.linkEXCLUSIF vers nom public ; pending conservé, aucun fichier public
vide/partiel ni écrasement. Si fsync consomme la pause, pas de nom public.
Consommateur : vérification après apparition/ouverture/validation du grant,
puis nouvelle lecture de phase/borne avant premier health. Healthinitial
MIN(END,leaseuntil,grantdeadline), marge15 existante ; trop tard=>aucunHTTP.
Avant writer, autorité vivante et caps de LEASE distincts restent nécessaires.

Embar9052ebbf20 puis3aa76a7a6009931bae5f41e5cc30961f6f3ef230 importés
strictement (ports5dcd049e2b/316d94c6b6), deux fichiers sans delta local.
publish_grant contrôle Ready exact/owner/nonce/protocolSHA/phase/borne AVANT
callback, APRÈS observation et APRÈS stagefsync. RelitReadysha/nonce,
protocolSHA et phase/clock immédiatement avant os.linkEXCLUSIF ; stage
supprimé enfinally, anciengrant/READY/firstred jamais réécrits.

Le module producteur ne fait AUCUNHTTP : callbackowner à borner et chargé de
valider les9champsprovider+PID/start/exeSHA/usage/lease et absence de calcul
autorisé. Il n'est pas rendu correct par la seule validation de phase.
Futurappelowner health_margin=30 proposé (défaut générique5) pour laisser
marge15consommateur. Aucun slot/health/warmup avant nouveau GO+READYvalide.
Checks et link sont séquentiels, pas atomiques avec firstred/finphase ;
consommateur revalide. Pas de fsyncrépertoire ni durabilitécrash revendiqués.

## Témoins et chronologie

df6104e5f4 publie les témoins avant run sur anciencode :3fonctions,
2rouges/1verte, logs ready-clock-cpu-red-01. VRAIEpublication sans timestamp
et VRAIconsommateur avec grantprésent+tardif atteignant provider FICTIF.
98ab fixe le contrat,3b66c67de0 les raccords. Les préchecks locaux ne sont
pas appelés jugement du couple figé. Après publication effective de
316d94c6b69e8a727c4faf60ba6a26537721898c :29fonctionsCPU vertes0.029s
(9cycle+4contrat+5READY+11producteur), logs ready-clock-cpu-green-316d-01.
Deuxpushes concurrents avaient été rejetés ; publication normale finalisée
AVANT ce jugement figé, aucun forcepush.

Embar a publié9052 avant ses11tests rouges (5fail/6pass), puis3aa avant
11verts+4contrat=15verts0.004s. ROED+Mémoire favorables au producteurseul,
preuves8fichiers réempreintées parMémoire. Tous ces tests utilisent fichiers/
linktemporaires réels mais horloges/autorités/proc/provider/Popen fictifs.
Aucun DB/CLI/native/health/service/signal réel.

## Couple à relire

Branche/WTneufs cx-arbre-cycle-ready-borne-20261116-r1, nouveaux P/Q/S
lifecycle-copy-v2/lifecycle-control-v2/lifecycle-scopes-v2 ; manifeste vise
seulement la NOUVELLEfixtureFIN.16sources contrôlées, dont producteur et
schémaReady. AncienB5/Qgrantinvalidé/sources/preuves conservés.

Protocole13 inchangé horscoordination : debt0/flagsFTSvides/FTS5avant,
digestbitTOUS97247avant/après, noncequery→metaquery/run/PIDstart/ELF/mapsCG,
publicationPUISshutdownACKexactPUISrc0mêmeenfant ; snapshotPOSTunefois/FTS5/
finished/sourcesstrictes. 900PROPOSÉS/outer898+kill2/common830/32G/HIGHmax/
swap0/quota<=8, READY120MAX MINEND−500, callMIN180/END−500/lease−20 et
réserveRO500, finhash/quiet/prépublication sous mêmeEND. AucunB5réanimé.

Publicationsource+paquetPC puis ROcouple avantGOdistinct. Seul un futur
cycle observé peut qualifier le shutdown de cettefixture, pas durabilité
générale, vieuxshutdown, LIVE, RETOUR ou bascule. Tickets restentOUVERTS.
