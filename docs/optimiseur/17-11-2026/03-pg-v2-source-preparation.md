# PG v2 — source proposée et rouge causal à acquérir

Même FTS302, arbitrageprépa5b838bda7a. WT/branche distincts
`/home/lucied/git_workspaces/cx-optimiseur-pg-v2-preparation-20261117-r1`.
Ancien B09ba/sourcef4e, validateurv1de69 et preuvesrouges sont inchangés.
Aucun nouveau code ci-dessous n'a été importé/testé/compilé/exécuté en PG.
Les jugesimagepsql de l'allocationDocker sont une autre portée, pas ces tests.

## Ce qui est préparé, pas qualifié

Sources sous docs/optimiseur/16-11-2026/pg-ro-attestation :

- pg_v2.py : allocationsv2 strictes/attentes indépendantes, portloopbackprivé
  !=5433, noncefixture distinctnonceprobe, rôleROnonowner, ordreframes/sessionRO,
  identité/postchecktyped, premierrouge avantcleanup/mêmeEND/reapquiet, rapport
  pg-tool-probe-v2 nonadmissible commegrant/lease. Des traces canoniques validées
  remplacent les octets wire : ce n'est PAS le contratrawv1 déguisé.
- SecretInput masqué et payloadmémoire : whitelisttraces, canaris surbytes et
  strings JSONdécodées, jamais stdin/DSN dans request.json/argv/env. Exceptions
  externes ne sont pas recopiées ; leurs messages peuvent contenir les secrets.
  ProviderID et traces sont contrôlés avantpublication. Markerfirstred interdit
  d'interpréter un probe.json tardif déjàpublié comme une réussite.
- read_allocated_secret API : callbackscope/GO doit autoriser avant toutelecture,
  coredumpssoft/hard=0 requis, cache/rootnonce/credentialsdir700, fichier0600UID/
  nlink1/O_NOFOLLOW+stat/fstat. Formathex64 des secrets NOUVEAUXde cettefixture
  seulement. Aucun cheminoriginal/.env/.vault ni fichiersecretlu pendantprépa.
  Pas de claim d'effacement garanti des copies immuables en mémoire Python.
- pg_v2_native.py : adaptateurprocesssource, groupe supervisé dédié exigé,
  ELFpin/UID/exe/PIDstartCG avantstdin, envwhitelist sansCommon/lockdir, pipe
  borné/selectors/framing, pas d'archive du payload, stderrnonreconnu nonarchivé,
  finish unique/stop-reap/quiet du groupehorsworker. Aucun .capture appelé ici.
- pg_ro_client_v2.rs : nouveau candidat no-unsafe, JSONuniquekeys via serde_core,
  ciblefixtureallouée explicite et sslmodedisable, BEGIN READ ONLY + constante
  readonly-session-v2.sql dans CHAQUEsession, identité/postcheckconstants,
  Owner/CancelToken puis finisherpartagé cancel-si-token-et-temps/drop/join.
  Échecconnectsansowner/pid/token resteUNKNOWN ; erreurfuture n'est jamais
  transformée en succès. Un erreurstopcheck n'emprunte pas le PIDidentité.

Le finishergénérique SessionClose est utilisé par l'Owner réel et les doubles
Rust, pas un modèle séparé. Sur échec query/shape/émission, cleanup mêmeEND,
sourcefailureavec session/code/flags/PIDouNULL et backend_absentNULL. Aucun
SQLadditionnel de nettoyage aprèsrouge : arrêt/cancel/drop/join seulement,
absence serveur UNKNOWN etREFUSED. La preuvepostcheck SQL est celle du parcours
nominal, dans la même borne originale, pas un nouvelEND ni une autoabsence.

La bibliothèque Python principale et l'adaptateur refusent leur entréeCLI
PREPARATION_ONLY. Il manque volontairement le binding exécutable de vraiGO C/
packet/allocation/READY/Common/credentialprovider et publication d'admissionv2
côtéconsommateur/owner. Un callbackinjecté n'est pas un grant autoritatif.
Aucun de ces morceaux ne peut être déclaré runtimeadmis sur les seuls faux.
Tout futur binding doit mettre RLIMIT_CORE=0 DANS son worker AVANT credentials.

## Rouge causal concret, publié avant nouvelles sources

Source test_pg_v2_legacy_secret_red.py publiée4b770391f5 : appelle le vrai
NativeBackend._run de B et son vrai controller_environment/request.json,
avec process/horloge/Common/proc entièrement simulés par NativeBoundaryFakes.
Le stdin est un canari littéral fictif, aucune credential réelle. Le fichier
archivé contient ce canari : une ASSERTIONFAIL attendue (pas un ERROR/import/
compilation) démontre le raccord fautif historique. Module rouge dédié, jamais
inclus dans une suite verte par découverte globale.

CPU ROUGE proposé, NON JOUÉ, cap58TERM+KILL2 :
`python3 -B -m unittest -v -f test_pg_v2_legacy_secret_red.LegacySecretBoundaryRed.test_legacy_b_artifacts_must_not_contain_stdin_canary`.
Attendu1FAIL/0ERROR/0SKIP/rc1, message legacy B archives stdin. Tout ERROR/PASS
inattenduSTOP, aucun retry/fauxrougeinventé. Le codeB ne sera pas corrigé sur
place : l'artefactrouge et la source figée restent référence.

## Filets nouveaux, tous NON JOUÉS

20 testsPython préparés :17cœur/frames/flags/allocation/pins/canaris/connect
avantPID/CancelToken/postcheck/sessionéchouée/reap/quiet/temps/firstcause et
3référencescredentials privées (pas de réelsecret/proc/OSsignal/network).
Ils passent par le vrai constructeurtrace/publisher v2 avec transportinjecté.
Le testcanari du core ne qualifie PAS le pipe/Popen réel de l'adaptateur.
CPU GREEN proposé aprèsvrairouge+RO/pins/GO distinct :
`python3 -B -m unittest -v -f test_pg_v2`, cap58TERM+KILL2,20PASSattendus,
zéroFAIL/ERROR/SKIP, sans modulelegacyrouge ni anciennes23suites.

9 #[test] Rust SOURCES NONCOMPILÉES :4parsing/doublons/portpartagé/erreursecrets,
5finisher sans-token/cancel-drop-join-sameEND/ENDexpiré/pidinconnu-joinfalse/joinErrorREADYnonrepolled.
La compilationtest/ELF sera une qualificationNATIVE distincte, pas un résultat
CPU Python. Les cheminsréels connect/cancel/query/postcheck/OSkill/reap/EOF/
stderr/fsync et scalaires serveur restent à qualifier sous GOs appropriés.
Les fakes ne qualifient ni transport ni l'invariant physique whole10.

## Dépendances/sourcepin et limites proposées

Aucune Cargo/build.rs/libSDK : compiler privé1.98.1/sourcearchive317files
manifestbf6b72be485227fadfdda40c133c5b52d4a8a31e7dba1855821c2b34f85d8e25,
READY39e9152c204d5c966307eef4586c7ef278031a82ee58e04c0c237eaeeaccadb4,
OUT_TEDde797438448c79ae54a1bc313003b0fdead4e37b70b180298ce21690b320c945.
Les troisexternstokio_postgres/tokio/serde_json B sont conservés, plus
serde_core-d59a1148f6c5dbf1.rlib déjà dans les74archives, SHA256
89e561380dbbf4322462bfdec990fbef2289a40d6cd9c92bb1123c504e051767.
La présence du fichier n'est pas une nouvelle qualificationlink/type Rust.

BornesC restent PROPOSÉES : workerwhole10 (I/O/credentials/startup/SQL/cleanup/
publication inclus), Common30/OS35TERM+KILL2 ; le superviseur externe doit porter
toute la phase, pas seulement communicate. L'adaptateur n'est pas un superviseur
physique autonome. L'entréeCLI restantdésactivée, aucune commandePGadmissible
n'est produite par ce paquet deSOURCE. FutursGOs CPUcausal/CPUvert/natif sans
contact/PGcontact sont DISTINCTS, aucun transfert du GO Docker/B.

La fixture02 a étéprovisionnée/observéeREADY à sonépoque etexpireauplustard
PCmono1270341.372028805. Sa préparation n'attend pas ce code etsonTTL n'est
jamais prolongé pour lui ; preuves/setupconnues ne sont pas un ACKactuel.
AvantCcontact : allocation/identités/leasefresh/domaineclockPC/attentesSQLdont
adressebackendTCP dérivée de sourceindépendante (pas du candidat), packet/
ELF/pinsqueries/RO/GO exact. Ni fixtureexpirée ni PGpartagé5433 en fallback.

## Paquet source01 et recette CPU proposés

Code exact `6fdb9cddfc096a4ba595d244485d6867bd016edd` (sourcecandidate, pas un ELF).
PCroot `/home/lucied/.cache/optimiseur-notes/pg-v2-source-20261117-r1`.
`source-packet-01/source-packet.json` SHA256
`8194fafe5f237235f47d98b4c12a5d59298276db5450f59bf70571d2215af195` ;
19fichiers pinés, deuxCommon6439 réguliers, queries/go/allocation/compileplan
inactifs inclus, aucun secret ou référence active copié de la fixture.
`recipe-01.json` SHA256
`dbf22ff9b733708bcff9791de948cb85b7dc852ad33f65cab1b5ffdd0002de7d`.
Métadonnées source/recette copiées localement, sourcecopiée depuis blobsGit
explicites (jamais depuis credential/privatefixture). Source/go/status tous
PREPARED_NOT_RUN/run/compile/connect/credential_read=false, live_entry=false.

Nouveaushaconstante readonly-session-v2.sql :
`f40eb6ebc867839303aa51229497e7d533c6708854bea9b63bcfe67f2012b8b9`.
Identitéf949/postcheck2907 inchangés. Compileplan inactif utilise quatreexterns
archivés et propose DEUX sorties/bâtis distincts (testsRust puisclientV2),
aucun de ces deux rustc n'est autorisé ni exécuté. Pasréutilisation09ba commev2.

Demande RO totale source/19copies/recette/rougecausal avant GO CPUred exact,
puisGOgreen20 séparé. Aucun import/unittest/rustc/ELF/exécutionC/credential
réelle aprèscréationpaquet. Les9Rust exigent autreGo native ; aucunGOPGcontact.
Les claimsmetadata n'impliquent pas un runner admissionC livré : les bindings
Common/READY/owner/GoC restent à raccorder sousnouveauxpins etrevue avantcontact.


Correction prépa après consignation6f0c604ca7 : join_connection_driver partagé
par l'Owner réel et un nouveau testRust ne repoll pas un handle consommé en
ReadyErr. Timeoutpending seul autoriseabort+join dans le mêmeEND.9Rustsources
restent NONCOMPILÉES,20Python etrougelegacy toujoursNONJOUÉS. Paquet01code6fdb
reste conservé historique ; paquet02 deSOURCE etnouvelleRO àémettre.
