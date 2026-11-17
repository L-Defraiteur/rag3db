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
