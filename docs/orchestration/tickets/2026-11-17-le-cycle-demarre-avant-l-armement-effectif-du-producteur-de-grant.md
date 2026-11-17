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

## Partage et contrat de correction avant source acteur

Base de préparation e65cbd34a582c3bbb82773bfd9fd6f9b8abff695, archives3c2
publiées et figées. Arbre porte le contrôleur/supervision et les hooks cycle dans
lifecycle_health_policy.py ; Embar porte le mécanisme commun
`docs/orchestration/17-11-2026/coordination/grant_watch_actor.py` et son test.
Recherche reprendra les mêmes octets avec sa politique globale ; aucun acteur
PG fictif tant que l'outil réel est en HOLD. Le mécanisme ne remplace ni les
politiques Embar ni le helper strict, n'émet aucun GET avant READY valide.

CLI : `--watch CONFIG_PATH CONFIG_SHA256`, hash externe. La configuration runtime
est créée une fois par le contrôleur dans P, à partir des sources/profils
préalablement pinés et de son autorité common active. Elle ne se contient pas
elle-même : le préfixe argv exclut son SHA final ; argv attendu ajoute le SHA
externe. Actor_nonce et ready_nonce sont choisis avant le spawn, jamais depuis
une notification tardive. Aucun jeton d'autorité ni environnement complet dans
config, descripteurs ou journaux publics.

Après vrai exec, l'acteur publie watcher-armed.json ; le parent compare à sa
mesure OS indépendante PID/start/UID/CG/exeSHA/argv/scriptSHA, source/allocation,
prepared/configSHA/nonces et poll vivant AVANT tout contrôle copie/DB. La lecture
OS est partagée ; une transition R/S n'est pas un changement d'identité, deux
états doivent rester vivants. Un fichier ARMED seul ou une session disponible
ne constitue pas cette preuve. Un READY/grant/refus déjà présent interdit de
simuler un armement préalable.

L'acteur attend sous END ORIGINAL du même cgroup/watchdog common, sans créer un
TTL start+900 ni un scope sœur. Refus/disparition/unexpected exit0 déclenche le
premier rouge immuable du contrôleur immédiatement. Après le grant, pas de GET
supplémentaire : acteur encore vivant jusqu'au watcher-release.json authentifié
DRIVER_FINISHED ; seul le parent rend sa sortie0 attendue après driver0 et reap
borné MIN2/END. Résultat intermédiaire watcher-grant.json et résultat terminal
watcher-result.json sont distincts et créés exclusivement, pas réécrits.

Source/fakes de ces vrais raccords à publier avant exécution CPU60 hermétique,
puis rouge→fix→vert et revue totale. Cette préparation ne lance aucun acteur,
processus de test natif, lecture /proc, health, DB, CLI ou nouveau cycle. Les
preuves OS manquantes du3c2 ne sont pas reconstituées a posteriori.

## Préparation acteur commun publiée (Embar, 17 novembre)

Source des deux fichiers `grant_watch_actor.py` et
`test_grant_watch_actor.py` : ff69dcbd1fcbd7b807c4880a7a556b539c8069d2,
branche cx-embar-grant-watch-actor-20261117-r1. Code acteur strictement identique
à 96f3a2a121. SHA256 acteur
`789058a8a8d15815e1a1b4083c1201d6ae390a79e9471740c11ab38af31f5175`,
tests `cd0f3f4691148ac8f9ed4dfc65ddc20a88e46a20350c535fd11c664f82264d11`.

Témoins publiés 5a0cbc5997 avant exécution : premier témoin d'admission
physique vu rouge, ValueError « actor admission not implemented », 1 test,
0.004s sous timeout60/failfast. Après correction publiée96f3 : 12/12 verts
0.061s. Extension de témoins publiéeff69 : 17/17 verts0.015s, aucun skip.
Commandes : `timeout --signal=TERM --kill-after=2s 60s python3 -B -m unittest
-f test_grant_watch_actor`, depuis le dossier coordination. Aucun acteur OS
ni sous-processus de test lancé ; faux proc, horloges, autorité et politique.
Les fichiers temporaires des témoins ne sont pas des ressources du cycle.

ARMED est créé exclusivement dans la première boucle après les contrôles,
avec process complet, loop_counter et loop_observed_monotonic. Le parent
réutilise process_snapshot/validate_armed et doit vérifier son propre poll ;
le reçu seul ne suffit pas. Heartbeat remplaçable borné à10s,
phases WAIT_READY/PUBLISHING/WAIT_RELEASE. L'ACK intermédiaire et le résultat
terminal restent distincts. Après publication du grant, aucun second contact :
attente du release DRIVER_FINISHED lié à configSHA/nonces/pilot/allocation
jusqu'à END original. Release anticipé, firstred, changement de processus ou
sources entraînent refus, sans écrasement du premier rouge.

Tous les fichiers Q sont vérifiés avant import de la politique et chaque
contrôle d'action ; prepared et config restent liés par SHA externe.
`paths.grant` reste dans Q ; les traces acteur sont dans P. La politique
préépinglée porte les contrats services, lease, GET et flags ; le mécanisme
commun ne transpose pas no_embedding du cycle vers la politique globale.
Les hooks sont preflight(config) et publish_ready(config,path,sha,nonce).

Qualification limitée à la préparation : raccord contrôleur/protocole/politique
porté par Arbre, qualification totale et RO du nouveau couple restent requis.
Aucun RUN, health, lecture /proc réelle, DB, modèle, slot ou nouveau GO.
L'ancien 3c2 demeure ROUGE et ses archives restent inchangées.

### Raccord des rôles Python avant intégration

Le contrôleur a besoin d'observer aussi --controller et --driver, que le sampler
--watch refusait correctement. Témoins publiés1dddf58545, rouge observé
AttributeError basic_process absent, 1 test0.002s avant correction. Source
finale des deux fichiers db04b43ac3f3c4bcbac208953c86784c8b55fb3f ;
19/19 CPU verts0.010s, zéro skip, même commande CPU60 après publication.

`basic_process(pid,*,clock=...)` expose la double lecture minimale stable
PID/start/UID/cgroup avec deux états vivants ;
`python_role_snapshot(pid,expected_script_path,*,proc_root=...,clock=...)`
partage la double lecture complète et exige le chemin du script attendu AVANT
son hachage. Seules les formes Python script et Python -B script sont acceptées,
pas -c/-m/env/autres options. Aucun environnement n'est lu ou retourné ; le
contrôleur conserve la politique d'environnement de lancement.
`process_snapshot` garde la spécialisation --watch du même mécanisme.

SHA256 finaux : acteur
`db1838bef339ad06f3c58be98b062e81c7c039a8905864c62b38d6334a900914`,
tests `015091dfd05d235cd189ca5769cb9b209d7b1a961154918eaa0350877acfe568`.
Ownerbcfd inchangé ; qualification totale du couple et RO restent requises,
aucun acteur réel, /proc réel, health, slot ou nouveau GO exécuté.
