# Le cycle démarre avant l’armement effectif du producteur de grant

- **État** : ouvert, 17 novembre 2026 ; cycle privé3c2 refusé avant health et writer, défaut de coordination à corriger avant toute reprise.
- **Famille** : produit
- **Gravité** : P1 de qualification et coordination ; aucune dette de donnée originale attribuée à cet incident.
- **Atteignable en service** : non, contrôleur de qualification sur copie privée et son producteur de grant.
- **Touche rag3weaver** : oui, indirectement par les pilotes et preuves ; aucun correctif Rust demandé ici.
- **Pour** : orchestration, Arbre et Embar.
- **Signalé par** : orchestration, retours concordants Arbre et Embar du cycle privé3c2.

## Faits, sur la même horloge PC

Allocation C7-LIFECYCLE-3C2-17nov900, source3c2a93ca38ba5729b4414a5d813467f916cbf905/Qv4prepared69c5480da4bc212f4e435a4419c0a56ebb3b26b041e3ca7ade47dd37336aa68f, nouveauPcopy-v4. Aucun ancien délai n’a été réactivé.

READYnoncea93c7255b4d945148b6d91c89be3fb51/SHA946e93c74f2750e5e4b6e6d9f84d01c205d6851e65767e5175be4f8a68b475b4 : observation1246927.830043659, grantdue1247047.830043659, END1247698.195278915. Marge owner30 : dernière entrée possible1247017.830043659.

WatcherPID2000831 armé seulement1247046.19743902, après cette entrée maximale. Il refuse avant publish_observed_grant et avant GET ; resultREFUSED_NO_RETRY/grant_presentfalse. Son annonce « veille active » était incorrecte : poll=1 était déjà observé. Embar la corrige explicitement et préserve ses traces.

Pilote : firstred1247047.917505952 fresh lifecycle provider coordination deadline expired. Grant, provider-before.transport et writer-process absents ; composéRED, scope arrêté/nettoyage à réattester. Seuls les précontrôles sur la nouvelle copie avaient tourné, aucun writer ni contact provider des deux acteurs. Aucun retard réseau ou transport n’est attribué à partir des messages.

L’orchestration avait reçu une disponibilité annoncée, puis lancé avant preuve de veille effectivement armée. Cette admission était insuffisante ; les rappels de message n’ont pas remplacé un processus vivant. Le contrôle de phase a correctement empêché le grant tardif et le writer.

## Conservation

Arbre conserve P/Q/Sv4, copie et sorties readonly, firstred immuable, scopes et quiescence/sourcehash avant toute correction. Embar conserve ~/.cache/embarquement-notes/lifecycle-3c2-watch-v4/{armed,result,expected-pins,expected-provenance}.json, watch.py et watch.log. Copies et manifestes sur deux postes avant tout fix. SourceFIN immuable et originaux restent hors écriture.

## Correction à préparer avant une nouvelle admission

Un contrôleur coordonné sur PC arme le watcher à la politique Embar préépinglée AVANT les précontrôles copie/DB et AVANT READY. La disponibilité d’une session ne compte pas comme armement. Un descripteur ARMED complet est publié après le vrai exec, puis lié au PID/start/UID/exe/argv/scriptSHA/cgroup et à l’allocation/source attendus. Le consommateur vérifie le processus encore vivant, pas uniquement un fichier ou Popen sans contrôle de poll.

La publication et consommation du grant n’ont plus besoin d’un ping humain. Avant READY puis chaque action, la liveness/phase est recontrôlée ; disparition/refus du watcher déclenche immédiatement un premierrouge, sans attendre120s. Le helper owner strict reste identique et s’exécute seulement après READY valide, sans health préalable. Le même watchdog physique borne attente, watcher, worker santé et contrôleur, avec cleanup attribué et borné. Pas de nouveau mode d’acceptation tardive ou de délai prolongé.

Témoins aux vrais raccords à publier avant code : watcher jamais armé, vivant annoncé mais exited, retard avant exec/descripteur avec mauvais argv, watcher terminé après ARMED, READY arrivé avant ou après refus ; aucune action provider/writer avant admission complète. Les contrôles deadline/nonce/pins/typedhealth/firstred et noembedding restent stricts. Aucun process réel, réseau, DB ou CLI dans ces témoins hermétiques.

Après source/tests publiés et RO totale du couple/pin, nouveau GO correctif distinct uniquement. L’ancien3c2 reste ROUGE ; aucune reprise n’est autorisée par ce ticket. Pas de migration LIVE, redémarrage de service ou actor de remplacement non attribué.

## Préparation indépendante après conservation complète

Preuves natives rouges publiées e65cbd34a582c3bbb82773bfd9fd6f9b8abff695,
84 fichiers PC/local réempreintés0écart, manifeste8087ab5db59b8f48bdfb98579f0743acecb1d7831197781a981c0fde814e55de.
RO preuves ED+Recherche favorables bornées au refus et à la conservation ;
aucune qualification du cycle. Nouveau WT de préparation distinct
cx-arbre-cycle-producer-controller-20261117-r1, aucun code dans les anciens
P/Q/S ni dans la source PC3c2.

19dd4ec680 publie six contre-exemples du VRAI `lifecycle.publish_ready` puis
ils échouent tous en0.004s : jamais armé, annoncé vivant mais exited, argv
incorrect/exec retardé, boucle non engagée, disparition après ARMED, premier
rouge présent. Le code courant ignore les états simulés et publie READY.
Ces témoins constatent l'absence de garde à ce raccord ; ils ne constituent
pas encore une qualification des futurs validateurs PID/ARMED ni du futur
contrôleur. Des fixtures complètes et un parcours positif seront nécessaires,
ainsi que les vrais fakes du parent/superviseur. Aucun process/proc/service,
réseau, DB, CLI ou signal réel dans ces témoins.

Interface commune Arbre/Embar convenue avant code : acteur versionné
grant_watch_actor.py, politique cycle dans lifecycle_health_policy.py et
politique globale distincte. Config SHA externe en argv (non circulaire),
source/allocation/prepared et deux nonces prédéclarés. ARMED après vrai exec
et première itération vivante, processus OS indépendant et poll contrôlés,
heartbeat borné sous END original ; grant ACK intermédiaire puis acteur
reste vivant jusqu'à RELEASE surveillé. Même common cgroup/watchdog ;
aucun TTL900 propre au watcher, aucun nouveau GO ni runtime correctif.

## Groupe correctif publié, CPU seulement

84a61e6934 : contrôleur commun supervisant acteur avant driver/copie/DB,
ARMED réel + heartbeat/process OS aux actions, nouveau descripteur obligatoire
dans READY/phase, deux nonces préchoisis et lease distincte, release terminal
sous même END. Acteur4f54 strict, helperowner83139/transportedf45 inchangés.
Après publication :124 fonctions CPU vertes0.283s,0skip/rc0, timeout59+kill1.
Les six rouges initiaux sont désormais verts, avec chemin positif, vrais
fakes parent/entrée/policy et les20 de l'acteur. Aucune action OS/proc/health/
DB/CLI ou service réelle dans ces tests.

Voir [préparation02](../../arbre-principal/17-11-2026/02-preparation-controleur-et-producteur-supervise.md).
Qv5/22 sources et Python à préparer en métadonnées, puis RO totale avant
tout nouveau GO C7. Ancien3c2/PQS4/FIN restent figés et ROUGE conservé.
Le ticket reste OUVERT : aucune qualification native du cycle/shutdown/LIVE.

## RO ED totale source2dccc/Qv5, réserve finale avant nouveau GO

[Rapport ED33](../../everything-declarative/16-11-2026/33-ro-cycle-supervise-2dccc.md) :
copie22/prepared e3741/HEADpropres/P-S-allocation-grantabsents recoupés PC,
contrôleur→ARMEDindépendant→driver/actions et RELEASE borné relus. Aucun run.
P1final hérité constaté : lifecycle.py239-242 rend positif après q.save,
qualify.py17-20 ne contrôle END qu'AVANTdump/flush/fsync. Si fsync traverse
END, composed positif et rc0 restent possibles après sa deadline ; outer900
n'impose pas la frontière Common830. Porteur Arbre, consigné avant correctif.
Critère : vrai finaliseur refusant un dépassement postfsync/préretour avec
verdict/code concordants et premier rouge conservé, témoin prépublié/rouge puis
fix/couple/paquet/RO neufs avant décision C7. Zéro test/proc/native/DB/health
par ED ; pas de GO3c2 reporté ni de budget remis à zéro. Ticket reste OUVERT.

RO ED statique témoins33ae/2d948 avant CPU : vraie fin compose/vrai q.save bien
exercés, aucun témoin joué. P2harnais lateentry laisse actuellement ValueError
remonter avant assert et composed reste vide => ERROR prévisible, pas rouge
d'assertion ; capturer/juger explicitement ce contrat avant pin CPU si rouge
fonctionnel demandé, sans modifier l'exécutable. Late-link n'est pas appelé
par le tail actuel, contrat futur distinct d'une preuve causale du bug fsync.
Voir complément rapportED33 ; les trois cas ontime/fsynclate/firstred visent
bien le vrai raccord. Pas de CPU60/GO correctif accordé par cette lecture.

RO ED du harnais seul d31d3e5749 favorable : exception lateentry capturée puis
assertion explicite rc non nul/JSON RED sans exception, octets vides nonqualifiés ;
annotation late-link contrat futur exacte. Réserve harnais levée statiquement,
P1 runtime2dccc inchangé et cinq fonctions NONJOUÉES par ED. C7 décide CPU60
distinct ; pas de GO correctif/native/runtime transféré.

RO ED correctif local6715c17ed9 favorable avant CPU : vrai tail→finalize_composed,
garde END/first-red après file/dir fsync/link/staging cleanup et avant retour0,
refus latché puis composéRED/rc1 ; q.save/commun inchangés. Rattrapage positif
limité aux octets canoniques du record attendu, contenu différent refusé ;
pas de preuve d'origine inode contre mêmeUIDhostile déduite. P/Q/S/manifestv6
cohérents, v5figé ; cinq témoins NONJOUÉS par ED, pinQv6 nonvalidé, décision
C7 CPU5/60 et tout GO natif distincts. P1 levé statiquement sur6715 seulement.

RO ED TOTALE source6715/Qv6prepared7bf5 favorable :
[rapport34](../../everything-declarative/16-11-2026/34-ro-totale-cycle-6715-qv6.md).
Chaîne entière ARM/liveness/profil/action/driver/final relue, pas seulement
delta ;22copies==Git==prepared/HEADclean/23entrées et P-S-allocation-grantabsents
recoupés PC. Manifestev6, Python/timeout,19refs+8carry, outils/SDKhash seuls
conformes ; Q5figé. Archives rougeD31unique1FAIL et6715cinqverts rehashées
3+3fichiers surdeuxhôtes sansrejeu. P1levé surcecouple, aucun nouveau blocage
établi ; zéro runtime/proc/DB/health/service/slot/native. Ticket reste OUVERT :
GO900C7 exact distinct et vraie qualification cycle/shutdown restent à décider.

Correction avis ED765 : RÉSERVÉ/HOLD6715/Qv6 après P1 Recherche confirmé.
Vrai argv compose=python,-B,confiner,lancer ; vraie signature_superviseur
exige args[2]=lancer/args[1]=script, donc refus avant ARMED. Point manqué par
ED à la revue totale, désormais recoupé source ; P1 finaliseur levé reste
distinct. Arbre a prépublié témoinb2bd NONJOUÉ avant correctif. Aucun ancien
avis favorable ni GO natif transféré ; voir section corrective rapportED34.

RO ED delta d22bf28c58 favorable AVANTCPUvert : seul-BCommon retiré,
PYTHONDONTWRITEBYTECODE=1 effectif, signatures/rôles-B/finaliseur inchangés,
PQS/manifestv7 cohérents. Témoin vraiargv/signature/envAST relu, rougeB2BD
1FAIL0ERROR après contrôlepositif et3fichiers réhashésdeuxhôtes sansrejeu.
Pas de vertd22, paquetQv7 ni GO900 validé ; décision CPU C7 distincte puis RO
couple neuf. Qv6resteHOLD ; voir complément rapportED34.

RO ED TOTALE neuve D22/Qv7prepared22eb favorable :
[rapport35](../../everything-declarative/16-11-2026/35-ro-totale-cycle-d22-qv7.md).
Chaîne entière Commonargv/ARM/liveness/policy/actions/driver/final relue et
22copies==prepared==Git/HEADclean/P-S-allocation-grantabsents contrôlés PC.
Common1PASSD22 et cinqfinaux6715carried relus sansrejeu ; quatrearchives3fichiers
réhashées LOCAL+PC0écart. Q6/WT6715/Q5 conservés. P1final+argv levés code/CPU
surCEcouple seulement, aucune activité native/DB/health/proc/slot ni GO900.
Ticket reste OUVERT, vraie qualification et décision C7 distinctes à venir.

Avis ED902 corrigé RÉSERVÉ/HOLDD22-Qv7 après P1 heartbeatRecherche consigné
Arbre : now échantillonné avant read_owned(HB), publication concurrente
plus récente faussement future lors de validate_heartbeat(nowancien).
RO statique témoin13ae favorable : vrai superviseur/read_owned, FakeWorld
OS/enfants/clock seulement, injection+0.01 sur première lecture après spawn
fictifdriver, compteur injecté puis assertions rc0/no-firstred/release. Nonjoué
par ED, décision rougeCPU60 distincte à C7 avantfix ; aucune suffisance native
ni GO900 tiré de l'ancien avis. Voir section corrective rapportED35.

RO ED delta6c6b968832 favorable AVANTCPUvert : HB lu avant horloge fraîche,
check_scope avant/après validation, tolérances strictes inchangées ; PQS/manifestv8
cohérents, Q7figé. Rouge13ae1FAIL0ERROR/3fichiers réhashés LOCAL+PC sansrejeu.
Seul témoinHB vert à décider C7 distinctement, pas rejeu sixanciens déduit ;
paquetQ8/ROtotale/GO900 toujours distincts. Suite rapportED35.

RO ED TOTALE neuve6c6b/Qv8prepareda5d8 favorable :
[rapport36](../../everything-declarative/16-11-2026/36-ro-totale-cycle-6c6b-qv8.md).
Chaîne entière Common/ARM/OS/liveness/HB/policy/actions/driver/final relue,
pas seulement delta.22copies==Git==prepared/HEADclean/23entrées et
P-S-allocation-grantabsents recoupés PC ; références/outils/SDKhash conformes,
ancienQ7/D22 conservé. Sixarchives3fichiers CPU réhashées deuxhôtes, HB1vert
acquis6c6b et sixancienscarried sansrejeu. Aucun nouveau blocage établi,
aucunruntime/proc/DB/health/slot/native ni GO900. Ticket reste OUVERT ;
décision C7 et vraie qualification physique distinctes.

RO ED preuves immutables65eb/Q8ROUGE favorable au périmètre/RAW :
[rapport37](../../everything-declarative/16-11-2026/37-ro-preuves-cycle-q8-rouge.md).
103fichiers17346531B/manifest497baaf recoupés LOCAL+PC0écart ; chaîneARM/grant/
nonce/publication/shutdownACKmêmePIDrc0/maps recoupée. Premierrouge stable_readers
confirmé, puis acteurrefusconsécutif ;2252→2250persistés=.ecrivain/.wal retirés,
DBSHAchangé, autresidentiques. before_writerRAM nonpersisté, aucun POSTgraph/
FTS/bitdigest ni sourcesafter/quietindépendantaprèsrouge déduit. ScopeROUGE1/
restes[]/nettoyé/TERMINE/busabsent et composéRED conservés, aucune innocuité/
causalité native ou durabilité/LIVE qualifiée. ZéroDB/health/native/rejeu/fix
par ED ; toute suite proposition/RO et décision C7 distinctes, ticket OUVERT.
