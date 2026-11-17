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
