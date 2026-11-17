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
