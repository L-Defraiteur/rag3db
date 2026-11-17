# Suite des19 : source préparée, zéro exécution autorisée ici

Audit C7 8b54d1bbac, même ticket FTS302. Le rouge nominal, quatre rouges de
relecture et huit fake/FS verts restent immuables ; aucun résultat transféré
à cette nouvelle source de tests. Runner produit6148 INCHANGÉ. Ce lot prépare
les19 autres uniquement ; pas de contact PG ni de nouveau test exécuté.

## Sélection et nature des témoins

| Groupe | Nombre | Ce qui serait réellement exécuté sous futur GO |
| --- | --- | --- |
| Profiles | 8 | JSON/profils/SQL typé en mémoire, aucun processus |
| WholeJob | 3 | Transport injecté, vrai publisher/fsync/FS privé, aucun processus |
| RealCapture expired | 1 | Popen mocké et assert_not_called, aucun processus |
| RealCapture pipes/exit7/floodstdout/floodstderr | 4 | Quatre vrais enfants Python privés via capture |
| RealCapture timeout/TERM ignoré | 2 | Deux vrais enfants Python privés, pidfd/TERM/KILL/reap bornés |
| RealCapture EOF retenu | 1 | Un vrai enfant Python plus UN petit-enfant détenteur EOF, durée bornée350ms |

Total19 :12sans enfant réel et7avec, soit7enfants directs +1descendant fixture
si la suite arrive jusqu'à ces cas. Ce ne sont pas de faux processus. Le fork
est celui de LA FIXTURE EOF, pas un observateur Common ; aucun nouvel observer
forké, aucun changement de Common. Les8fakeFS déjà verts ne sont pas rejoués
et leur module n'est pas copié au paquet19.

## Source d'évidence et invariants proposés

`capture_evidence.py` s'exécute dans LE processus unittest, jamais au chargement
du module. Racine child-proofs privée/UID/mode700 et Python réel/SHA imposés par
la recette. Fichiers exclusifs600/fsync, dossier propre par nom de test, aucune
écriture dans source/caches partagés ni découverte de manifestes/identifiants.
Bruts stdout/stderr, record cause/cleanup/flags, argv/env explicitement PUBLICS,
stdinSHA et PID/start/exécutable/mapprefix archivés AVANT repropagation du refus.
Pas de swallow des erreurs non CaptureRefused ; aucune erreur ne devient succès.

La fenêtre de chaque capture couvre préparation d'évidence/capture/reap/dernier
fsync dans le cap original (0.5/0.8/1s selon test), sans reset. Retard d'archivage
ou identité inconnue : rouge. L'instant inscrit archived.json est pré-fsync ;
un contrôle de l'horloge après derniers fsync refuse tout dépassement. Ces caps
sont propositions à juger, pas des mesures garanties. La portée globale20s
reste l'ultime borne avec quiescence obligatoire ; aucun timeout sans FINI.

PID/start sont liés au pidfd de l'enfant de capture et observés dans /proc par
la primitive. Le helper compare l'exécutable observé au Python demandé/pinné et
exige identité connue, puis absence du processus ORIGINAL (reuse PID distingué).
Snapshots de FDs du SEUL parent unittest avant/après détectent les tuyaux ou
pidfd/selector laissés ouverts ; pas de scan des FDs des autres sessions.
Maps préfixées ne sont ni inventaire complet ni preuve de toutes bibliothèques.
Ces contrôles sont une proposition de preuve Python, pas une qualification psql.

Le détenteur EOF imprime PID/start/pgrp/session et signale readiness au parent
avant sortie0 de celui-ci ; identité brute conservée. Son pgrp/session doivent
être ceux de l'enfant directement possédé. Après EOF le test relève son état
par PID+start sans signaler un PID réutilisé. Un zombie est conservé comme Z,
pas appelé absent ; seul absent/original remplacé ou Z sans exécution est
acceptable au témoin, la quiescence Common globale reste obligatoire.
Les descendants inconnus ne sont pas déclarés tués par le reap du seul parent.
Le cas TERM exige HANDLER_READY et returncode=-SIGKILL, pas un handler supposé.
Les refus de flood/EOF/timeout doivent rester CaptureRefused avec bruts archivés.

## Recette proposée et gates

`runner19-cpu-recipe.json` : source7blobs+Common6439, Python3.14a382/timeout9c3e/
poste79e3, root runner19-cpu-05 neuf. Vrai POSTE32G puis OS28TERM+KILL2/Common20,
Python-B -m unittest -v -f test_runner. Attendu19/0FAIL/0ERROR/0skip/rc0, Common
VERT/TERMINE/nettoyé/restes[] et bus privé absent. Verbose donne les19noms ;
failfast impose STOP au premier inattendu,0retry, pas d'allongement de cap.
Aucun skip n'est un vert, aucun retry ou reprise des tests suivants après rouge.

La recette est encore NON AUTORISÉE : paquet source privé à matérialiser/hash
vérifier seulement, R lecture TOTALE du chemin (runner,tests,helper,frozen-v1,
Common+outils) avant éventuel GO C7 DISTINCT. Aucun import/parse/test de cette
préparation, aucun enfant RealCapture lancé. Métadonnées de copie seules
permises ; aucun accès aux bases, aux services ou aux secrets.

## Ce que ces19 ne qualifieront pas

Pas d'instanciation PsqlSession ni d'exécution psql/chargeur/Docker/PG/provider.
La factory réelle, exe/env natifs psql, auth/SSL/dlopen, outer whole5 y compris
import/pins/fsync tardif, producteur terminal externe réel/non falsifiable et
tout grant v1 restent NON QUALIFIÉS. Les fixtures fake du publisher gardent des
horloges/pièces synthétiques. Aucune allocation PG tant que client réellement
prêt et recette contact distincte admise ; pas de fixture idle pour attendre.
Aucun service7970/original/migration/retour/lib/SDK/buildCLI99 ou slot touché.


## HOLD19 R/C7 et quatre contre-exemples fakeIO prépubliés

RO totale R7b6 réservée sur le seul helperarchive ; C7HOLD36490b1217 confirme
AVANT fix. Réserve consignée FTS302 AVANT testdiff. Runner6148/helper0a2b
INCHANGÉS : aucun fix ni qualification19. Les11 contrôles du paquet19 de R/C7
conformes ne lèvent pas ce défaut source et n'autorisent aucun RealCapture.

test_capture_archive_red.py appelle le VRAI archived_capture avec FAUSSE
capture/CaptureRefused contenant record original. Quatre erreurs OSError
injectées APRÈS capture : write stdout, sync, proc_state, fd_snapshot. Aucun
child, pidfd, fork ou scan processus tiers. Uniquement FS/temp privé et
métadonnées de SON unittest ; sources des fixtures capturées publiques.
Les quatre juges exigent cause originale identité objet, record original,
erreurarchive SECONDAIRE et __cause__ explicite. OSError sans ces pièces doit
être4FAIL assertion (pas absorbé comme succès), import/autreexception=ERROR.

archive-fakeio-red-recipe.json PROPOSE4tests seuls/-v sansfailfast, attendu
4FAIL/0ERRORskip/rc1, Common20/OS28+2/POSTE32G, root archive-fakeio-red-06 neuf,
neuf sources dont Common + trois outils. Source/hash/paquet/Rlecture puis
GO C7rougeDISTINCT avant entrée. Aucun test/import/parse/enfant/PG exécuté ici.
Les19 et8anciens sont exclus, module test_runner importé pour type du runner
seulement, aucune classe19 sélectionnée. Après rouge conservé deux hôtes
seulement : fix premièrecause/record+archiveerror, sans bruts inventés si
l'écriture est impossible, puis nouvelle RO/GO fakeIOvert distinct avant19.


## Quatre rouges fakeIO acquis — GO57f223ab29, avant fix helper

UNE entrée sourceb6586cbc1a/recipe2bf184348d035759a5b6298b83a6a2998462df7f94cbce3e4c5b8c5ae9d73b00.
Scope validation-2ag211uv CommonROUGE/code1/erreur null, terminé/nettoyé/restes[],
bus privé absent.4tests/4FAIL first_error absent précisément/0ERRORskip/rc1,
elapsed0.3040391611866653s. Write/sync/proc/fd étaient FAUX refus sur capture
FAUSSE, aucun enfant/Psql/PG.19+8autres NON JOUÉS, aucun retry.

20preuves sources/bruts/pins/recettes/temps/Common sans jeton vérifiées SHA/
taille aux deux postes AVANTfix : PCpsql-host-runner-20261117-r1/archive-fakeio-red-06,
portable même parent/archive-fakeio-red-06-proof. Manifeste
13996ea3dd97b346c26cc36a9c8909129b99580e96a016de9bc31ca5d7818e2e ; archive
a54dc9a20b4cb32cd138100784d535efa1ce06ef0f17d1d3808295a9d6b1c443.
Fix source helper désormais autorisé : cause/record inchangés et archiveerror
secondaire explicite sans bruts inventés si FS impossible. Publication/RO/GO
fakeIOvert distinct avant toute entrée19. Tous anciens rouges/verts conservés.


## Fix SOURCE minimal helper après quatre rouges fakeIO — NON JOUÉ

Après conservation5019ad71f4 deux hôtes, seul capture_evidence.py change :
CaptureArchiveRefused(OSError) garde first_error OBJET, record OBJET inchangé,
archive_error distinct et archive_complete=False. Bloc d'archivage post-capture
protégé ; raise explicitement chaîné sur le refus initial quand présent, sinon
sur l'erreur d'archive. Aucune tentative d'inventer ou remplacer des bruts quand
l'écriture échoue, aucune autorisation/vert converti. Hors archivage, deadline/
identity/FD refusent toujours ; runner6148 et les quatre juges344df inchangés.

archive-fakeio-fix-recipe.json PROPOSE les quatre mêmes seuls juges, attendu
4PASS/0FAILERRORskip/rc0, -v -f, POSTE32G/Common20/OS28+2, racine NEUVE
archive-fakeio-fix-07. Source/paquet privé/R total puis GO C7 distinct requis,
zéro test/import/parse/RealCapture/PG pour cette préparation. Même si ces
quatre passent ensuite, les19 restent NON JOUÉS : nouveau helper oblige à
refaire pins/RO/GO19 après fakeverts, pas modifier ancien paquet19-05 gelé.
L'ancien helper rouge et les anciens8fakeverts restent immuables. Nativeexe/
env/outerwhole5/terminalréel/PG non qualifiés, aucun grant v1.
