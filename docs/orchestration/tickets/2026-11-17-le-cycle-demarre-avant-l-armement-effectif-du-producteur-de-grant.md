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

## Réserve finale sur la publication du composé — 17 novembre

ED612c778aca relève sur source2dccc070f1564d8ca9d22a59fcb2b73222f415e5/Qv5e3741c89 : lifecycle.compose contrôle END puis appelle qualify.save, qui ne contrôle END qu'avant JSON/flush/fsync. C7 confirme en lecture ces deux vrais raccords. Une publication franchissant END peut donc laisser un composé favorable et rc0 après la borne Common830 ; l'enveloppe OS900 ne prouve pas le respect de cette borne interne. Aucun franchissement runtime n'est déclaré : le couple n'a pas été lancé.

HOLD de ce couple avant tout GO. Même ticket et ancien paquet conservés ; publier un témoin aux vraies instructions finales avant correction, puis rendre cohérents publication, horloge finale et code de retour. Nouveau source/paquet et revue totale avant nouvelle admission. Les contrôles d'armement, liveness et action sont favorables en lecture mais ne lèvent pas cette réserve finale. Aucune DB/provider/CLI/autorité supplémentaire admise.

## Harnais final précisé avant CPU — réserves ED1320e637c8 / C7 8b941b379b

Le cas END déjà passé capture désormais le ValueError réel hors try final,
le retour et les octets/JSON éventuels avant de juger explicitement le contrat
retour non nul + composé RED. Une exception avec JSON vide est ainsi un FAIL
discriminant du contrat, jamais un refus qualifié ni une erreur du harnais.
Le cas late-link spécifie la future publication atomique : q.save actuel
n'appelle pas link, donc aucune causalité de lien tardif ancien n'est revendiquée.
Le cas late-fsync reste le contre-exemple causal prévu au vrai raccord actuel.
Ces cinq fonctions restent NON JOUÉES ; runtime2dccc inchangé. Nouvelle RO puis
GO CPU distinct requis avant toute exécution ; aucun correctif runtime encore.

## Rouge final observé puis correctif local prépublié

GO C7-CYCLE-FINAL-RED-D31-17nov60 consommé sur d31d3e57496945fdd13637bd6b933270240dc823 propre : une invocation du seul témoin fsync tardif,
1 FAIL (`0 == 0`), 0 ERROR, 0.006s unittest ; processus rc1 en 0.124370s.
Les quatre autres fonctions restent NON JOUÉES. Bruts/reçu/manifeste hors WT
sur les deux hôtes : ~/.cache/rag3weaver-build/cycle-final-red-d31-20261117-01,
trois fichiers size/SHA conformes ; SHA manifeste
d098d0a93cf366e8c0a6c70c3ad054afe9efaa7237d31bc6b5558482231fd846.

Correctif local lifecycle.finalize_composed réutilise write_frame existant,
vérifie deadline effective/premier rouge avant publication et après tous les
fsync/nettoyages juste avant retour. Un dépassement reclassifie atomiquement
son propre composé RED et retourne 1 ; premier rouge existant conservé.
Un résultat étranger n'est pas remplacé. Aucun changement de qualify.save
ni primitive générique nouvelle. P/Q/S et base manifest passent ensemble v6,
anciens v5 figés. Correctif NON JOUÉ : nouvelle RO/GO cinq verts distinct,
puis paquet neuf/RO totale avant toute décision native. Ticket reste OUVERT.

## P1 raccord argv Common avant ARM — revue totale Recherche

Sur source6715/Qv6, compose passe python -B confiner_validation.py lancer.
La vraie signature_superviseur Common exige args[2] == lancer et script en
args[1] : refus certain avant ARM/DB. Défaut hérité de2d, non détecté par
l'ancien test d'entrée qui vérifiait seulement --controller. Qv6 reste figé.
Avant correction : publication d'un témoin CPU extrayant le vrai argv compose
et la vraie fonction signature Common, avec faux fichiers proc temporaires
et contrôle positif sans -B. Aucun import/exécution du confineur, aucun proc
réel ni subprocess dans ce témoin. NON JOUÉ, nouvelle RO/GO CPU distinct requis.
Correction proposée après rouge : retirer -B uniquement devant Common,
PYTHONDONTWRITEBYTECODE=1 ; conserver les -B des rôles exacts. Aucun GO natif.

GO C7-CYCLE-COMMON-RED-B2BD-17nov60 consommé sur b2bd propre : contrôle
positif passé, unique juge FAIL sur argv réel avec -B, 0 ERROR, 0.016s,
processus rc1/0.168871s. Archive cycle-common-red-b2bd-20261117-01 hors WT
sur deux hôtes, trois fichiers size/SHA conformes ; manifeste SHA256
a181c7e876f82527079d3e4dbe3353f2233aacf6cef00f85528e2e56c224f894.
Puis correctif prépublié : -B retiré uniquement devant Common, environnement
PYTHONDONTWRITEBYTECODE=1 explicite, contrôleur/driver/acteur -B conservés.
Le témoin juge aussi environnement et argv contrôleur réels. Aucun changement
du Common ni du finaliseur6715. P/Q/S et manifest passent ensemble v7,
Qv6 reste figé. Correctif NON JOUÉ : nouvelle RO/GO CPU distinct avant vert,
puis nouveau paquet et revue totale ; aucun lancement natif autorisé.

## P1 ordre heartbeat/horloge — revue totale Recherche d22/Qv7

Dans la boucle driver de grant_watch_controller.supervise, now=check_scope
précède read_owned(heartbeat). Un acteur sain peut publier entre ces deux
instructions : seen devient supérieur à l'ancien now et le validateur refuse
un heartbeat pourtant présent avant sa validation. Défaut hérité2d, consigné
ici AVANT correction. Qv7/sourceD22 figés, revue totale réservée.

Témoin prépublié test_controller_heartbeat_order : vrai superviseur et vrai
read_owned ; FakeWorld existant sans OS/process réels, publication heartbeat
à now+.01 intercalée dans sa lecture seulement après création du faux driver.
Le juge exige l'entrelacement effectivement injecté, retour0/sans premier
rouge/release. NON JOUÉ : nouvelle RO/GO CPU rouge distinct avant test.
Correction attendue après rouge : horloge fraîche APRÈS lecture heartbeat,
avec contrôles END/premier rouge ; aucun assouplissement future/âge.

GO C7-CYCLE-HB-RED-13AE-17nov60 consommé sur13ae propre : injection unique
confirmée puis 1 FAIL rc1!=0, 0 ERROR, 0.007s ; stdout atteste refus heartbeat
stale/future/outside original scope. Processus rc1 en0.194917s. Bruts hors WT
cycle-hb-red-13ae-20261117-01 sur deux hôtes, trois size/SHA conformes ;
manifeste776aa3f6833826f3ec700b3d563739ca314989d91199a2628fa60a673a7cac74.
Correctif ensuite prépublié : lire heartbeat, revérifier scope END/premier
rouge, échantillonner monotonic frais, valider strictement heartbeat puis
revérifier scope. Tolérances inchangées. P/Q/S+manifest v8, Qv7 figé.
Correctif NON JOUÉ, nouvelle RO/GO CPU vert distinct avant test ; aucun rejeu
des six juges acquis sans cause, aucune qualification native/GO900.

## Qv8 natif : rouge conservé après shutdown, avant suite

GO C7-LIFECYCLE-HB6C6-Q8-17nov900 consommé sur6c6b/Qv8a5d861 : ARM/admission,
READY/grant autonomes puis réponse publique et shutdownACK/mêmePID/rc0 observés.
Premier rouge1266055.091886152 stable_readers après shutdown ; scopeROUGE1,
restes[]/nettoyé/autoritéterminée. Aucun retry, aucun assouplissement du juge.
[Rapport06](../../arbre-principal/17-11-2026/06-cycle-q8-rouge-apres-shutdown.md)
et manifeste Git :103 bruts conservés/réempreintés sur deux hôtes avant
diagnostic. Inventaires persistés : retraits .ecrivain/.wal, DB SHA changé
à taille constante, autres entrées identiques ; cause/innocuité NON QUALIFIÉES.
Suites vecteurs/graph/FTS après rouge NON JOUÉES. Aucune DB/health/recovery
nouvelle autorisée ; proposition RO puis revue avant tout éventuel GO distinct.
