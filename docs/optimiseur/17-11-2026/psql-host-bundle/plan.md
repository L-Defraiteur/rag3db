# Psql hôte privé — outillage sans contact, GO C7-PSQL-HOST-BUNDLE-17nov90

Arbitrageb9139349d2/FTS302. RustV2 gelé/horscritique, aucun test/fix/compilation
Rust supplémentaire. Image locale linuxamd64PG17 ID+digest exact
sha256:b0f9560a2de083e2cc7382e75f808c7381a32852a7ec49117deedb300e552b24,
aucunpull/latest, nouvelle racineprivée/nonce/labelsG.

Une extraction : Dockercreate STOPPÉ (PID0, Runningfalse), networknone,
entrypoint/bin/true, rootfsreadonly/tmpfsPGDATA pour éviter volumeanonyme.
JAMAIS dockerstart/exec/PG/SQL/port/credential. Image originale intouchée.
Copier psql et répertoires lib/usr-lib/usr-local-lib/sharePG de CEfilesystem
privé par DockerCP ; aucunconfigEnv.inspect. Capture bornée aux cheminsimage,
parserELF Python64LEmachine62/PT_INTERP/DT_NEEDED sans exécutionelf/readelf/ldd.
Les symlinksabsolus sont remappés à la racinecapture, jamais suivis surhost ;
référence noncapturée/dangling/ambiguïtéhash→refus, pas defallbackhost.
ABIattendu muslAlpine PT_INTERP/lib/ld-musl-x86_64.so.1, autreABI=>STOPproposition.
Closuretransitive .so finales et supportPG copiés en fichiers réguliers privés,
SHA/modes/provenance/symlinksorigine archivés. Cleanup du seulCID/image/nonce
stoppé etsesvolumesattribués, AVANTexécution du bundle.

Vrai POSTE32G/Common6439 délai75s/OS88TERM+KILL2=90MAX aprèsposte. Travail65s
maximum et réservecleanup dansENDCommonoriginal (aucuneallocationrenouvelée),
CLIcaps5sMINEND. Première erreurSTOP0retry/nouvelleimage/compilateur/Cargo/SDK/
OSinstall/contact. Résidus inconnusrefus, scopesCommon/PID/bus/brutsàconserver.

Ledger fichiers+programELF+kernelentry fsync physiquement AVANT toute exécution
bundle. Le noyau lance LE CHARGEURprivé, lequel charge LE PROGRAMMEpsql,
SHA/pathdistincts explicités ; programmePIE psqldirect noninvocable n'est pas
relabelisé commekernelimage. Releverproc/exe/maps du seul enfant siobservable,
UNKNOWN/exited-raced sinon, pas depreuveconstructive remplacée paraffirmation.

Aprèsledger uniquement : chargeurprivé --library-path bundle/lib --list psql,
puis mêmepréfixe psql--version etpsql--help. Source primaire musl dynlink.c
https://git.musl-libc.org/cgit/musl/plain/ldso/dynlink.c confirme ces flags,
rechercheenv_path d'abord et mode--list qui réécrit SON NOM en PT_INTERP.
Le labeloriginal/lib/ld-musl affiché par--list peut doncêtre un nomdesoi,
pasune bibliothèquehost réellementlue : exceptionuniquement pournomclosure
SHAidentiqueauchargeur, conservéecommePT_INTERP_SELF_LABEL_NOT_HOST_MAPPING.
Toutautre cheminrésolu doitêtre régulier DANSbundle et avoirSHAledger attendu.
Invariants etchoix figurent AVANTpremieressai, pas relaxation aprèsrouge.

EnvironnementCLIfixe : HOME/TMPprivés, LANG/LC_ALL=C, aucunLD_PRELOAD/envsecret,
PGHOST cheminprivé sanssocket/PGUSERbundle_no_contact/PGDATABASEbundle_no_contact
failclosed si une connexion inattendueétait tentée. --version/--help sontles
seuls modes psql, pas requête/identifiants/pgpass. DockerCLI lui-même reçoit
HOME/DOCKER_CONFIGprivés et DOCKER_HOSTunixexplicite pournepaslireconfigauthuser.

Résultat éventuel = ISA/ABI/CLI initiale et fermetureDT_NEEDED vérifiée dansCE
profil, PAStransportPG/lease/whole5. SupportSSL/auth/dlopen/locale interactive
etSQLrestent àqualifier sous future recetteprofile/-X/PGPASSFILE0600/ROmesuré/
postcheckwhole5 ; nepasfairepasserversion/help pourunrunnerSQL. PGv1 etsesQ
restentfigés ; ciblefixture nécessitera profil explicite, pas noms/portrelabelisés.
Manifeste, ELF/bibliothèques/supports, stdout/stderr/rc/argv/tempsetCommonfinal
conservés deuxhôtes, nonsecretseulement. Aucun nouveauPGfixture tantqueclientSQL
n'estpas prêt, aucuneprolongationancienne02(expirée/restituée).

## Pins de l'entrée autorisée01

Source08d436ed2c8f75b10923508d7265c2cb7c6032ac, branche
cx-optimiseur-psql-host-bundle-20261117-r1. PCrootneuf
`/home/lucied/.cache/optimiseur-notes/psql-host-bundle-20261117-r1/bundle-b630342a7eb1`.
RecipeSHA2562e4b62917c3f683ebc704e28c3f8ec1caeb07ac9665565622331cd44aea3a64b,
commandSHA273deeb581ed1ef890e6caa1a861b3c4a2026915e9de366e9beb449a801fb458,
nonceb630342a7eb1f4ec9475cccc4360c18b. Source/copiepinspréparées sanscontainer
ni exécutionELF. ABI déclaréavantentrée : muslELF64LEx86_64, autreABIrefus.
GO b9139349d2 autorise UNEentrée Common75/OS88kill2 après cettepublication,
aucune nouvelle permission attendue, aucunPG/credential/admission autorisé.


## Résultat acquis — entrée unique, 17 novembre

`HOST_CLI_PREFLIGHT_PASS`, Common `validation-m9zyc9yy` VERT, état terminé,
nettoyé, restes `[]`. Entrée monotone 1274231.8861648 → 1274233.066962824,
soit 1.180798024 s. Aucun PostgreSQL démarré, SQL exécuté ou credential lu ;
`transport_qualified=false`. Rust v2 reste gelé.

Programme `bundle/bin/psql` : PostgreSQL 17.11, SHA256
`c4e48eba81b1b9e2a8ab11347afdce0ffc9eb8df41c87e4e694dce3ab79cadb5`.
Entrée noyau et `/proc/exe` observé des trois enfants : chargeur privé
`bundle/loader/ld-musl-x86_64.so.1`, SHA256
`38d022ce7425ff105ccfb53598f606e6e5f5f0a34bfbc793d65e6f34c9d72806`.
Résolution, version et aide : rc0, sans timeout ni troncature. Ledger scellé
à 1274232.93790296 avant les trois exécutions, SHA256
`824b5913054898e735d88980cd9eb9505dbb0364d8384bfdcadf49c2d076c4a6`.
Bundle : 299 fichiers, 12 574 694 octets. Le label PT_INTERP musl reste
explicitement distingué d'un mapping hôte.

Le premier audit de copie a refusé le texte Docker minuscule `no such object`
car il attendait `No such`. Son brut initial n'a pas été conservé ; cette
limitation et le nouveau diagnostic brut sont archivés dans
`audit-01-refusal.json` et `audit-cleanup-diagnostic.*`. L'audit02 valide les
hashes, l'ordre ledger/exécutions, Common terminé et le diagnostic d'absence
CID. Aucun rejeu d'extraction ni d'ELF. La première extraction locale des preuves
a aussi refusé un répertoire déjà existant ; constaté vide, il a ensuite reçu
l'archive vérifiée sans écrasement de fichiers.

Preuves PC : racine d'entrée ci-dessus. Copie portable :
`/home/lucied/.cache/optimiseur-notes/psql-host-bundle-20261117-r1/bundle-b630342a7eb1-proof`.
356 fichiers vérifiés par taille et SHA sur les deux postes ; manifeste SHA256
`c3f52706ab8488ea3e62edfd12dc4327aaed8ede7ff767988521977b0bcb7023`.
Archive privée sur les deux postes, SHA256
`e334a7c78cf85ca3fc04b335637dc660ee10decc2f52fca75efb8324d34e4f35`.
Reçu de scope SHA256
`2b82b6cfaf90974da69ad674250c31c3891f581edbf320f7132b7979cbad0189`.
Qualification limitée à ce préflight ISA/ABI/CLI ; transport, SQL, RO mesuré,
lease et whole5 restent à réaliser sous recette distincte. Aucun service,
original, restauration ou ancienne fenêtre de migration réactivé.
