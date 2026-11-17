# Alternative client Rust : inventaire de préparation, aucun bâti

Même ticket FTS302/301, autorisation C7 de lecture d'archives immuables.
Aucun Cargo/rustc/client natif/PG/DockerAPI/service n'est exécuté ; aucun
lecteur G sur target/deps vivant et aucune réservation. Source PGv1 pure8ddb
et trois pins de69/f949/2907 restent inchangés.

## Sources et API déjà disponibles

Cargo.lock APP636 : SHA090cc4e1330dc694127c0f08330d25acc656699763079400b502ba6607093fa7.
Versions : tokio-postgres0.7.16, tokio1.53.1, serde_json1.0.151,
postgres-protocol0.6.10 et postgres-types0.2.12. Sources locales des trois
premiers comparées OCTET POUR OCTET aux `.crate` déjà en cache, SHA des tarballs
identiques aux checksums du lock ; aucun téléchargement. Le fichier individuel
.cargo-checksum.json n'est pas présent : aucune preuve n'en est déduite.
Rapport privé `~/.cache/optimiseur-notes/pg-ro-attestation-20261116/rust-source-dependencies-ro.json`,
SHA c7eacc2eb4d4ccb943987b7257d3cd1d90984046569b843c059a794af4358c9b.

API lue dans tokio-postgres/src/config.rs : options319, application_name331,
hostaddr413, port423, connect_timeout437, connect729 ; Client.simple_query576
rend les champs texte du protocole simple et permet les deux SQL constantes
sans implémenter wire/auth/crypto. Le produit utilise déjà tokio-postgres et
NoTls dans postgres_connection.rs. Un client standalone éventuel peut rester
hors rag3weaver/SDK/C++, avec runtimeTokio et serde_json. Ces signatures lues
ne sont PAS un typecheck, un link réussi ou une qualification de probe.

## Archives candidates et limites constatées

Archive G PC `~/.cache/optimiseur-notes/bascule-16nov-927/release-archive` :
manifest12:13 VERT4806 avec copie fidèle à cette date ; AUCUN artefact postgres,
ni tokio_postgres rlib/rmeta dans la copie. Tokio27f9a018 est présent,
SHA455239746933d42071def254c3d2bd25975906738788597ddf19a1d23e5fea15 ;
serde_json051dd28e SHA c5cc039553c35bd0ade3f9c32b42d5f3698cc2de7fb88f310ec416400a882806
et4224bc8d SHA820c72aa22ad353845c19562892da95637c34396ff778877076270529054a9d5.
Ce ne sont que des candidats : deux variantes serde ne doivent pas être choisies
au hasard, fermeture transitive/compilerfeatures non attestés pour ce client.

Fichier rustc PC1.98.1 rehashé SANS exécution :
`~/.rustup/toolchains/1.98.1-x86_64-unknown-linux-gnu/bin/rustc`,
SHA859254978c0a0402c32f949f6de0d99aee73be8d15f45aac00ae1448aac51e74.
Un SHA actuel n'est pas une attestation de compiler avant le bâti historique.

Recherche confirme ses archives133ROOT/94GEN/62DOUBLE/65canonRED : ELF/source/
ledger/bruts, zéro rlib/rmeta et pas de stamp compiler prébuild qualifié.
Pas de lecture de sa target ni de copie par Recherche.

ED indique T b472 quiescente et propose une copie PRIVÉE HORS_T des dépendances
existantes, fingerprints, lock, rustc_info et compiler SHA observé. C7 l'autorise.
Paquet demandé parce que G927 est incomplet ; G attend finalisation/manifest
et ne lit PAS le T direct ni un lien vivant. Ce paquet n'est pas encore une
preuve de compatibilité ABI, de fermeture transitive ou de link réussi.

## Suite conditionnelle avant toute recette client

Exiger copie immuable complète avec types/SHA, source/lock/features/compiler
attribués et fermeture transitive cohérente ; conserver deux variantes ou un
manque comme refus, jamais lancer un build pour découvrir un candidat au hasard.
Après audit seulement, publier fakes de contrôle et source standalone/recette
rustc pinée ; toute compilation et toute exécution exigent un NOUVEAU GO.

Le futur transport se nommerait `standalone-tokio-postgres` : il n'est PAS
accepté par PGv1 actuellement. Ne jamais le déguiser en private-host-psql ;
contrat/owner/consumer devront être revus ensemble avant admission réelle.
Borne5 unique, startup RO+timeouts/connect/lock, application_name nonce,
reap du seul client hôte/TCP et postcheck attribué restent à qualifier.

Pour la topologie, éventuel futur Dockerinspect sous borne séparée : FORMAT
ID/image/binding UNIQUEMENT, jamais Env. Aucun inspect ou programme client ici.
Tant que les archives ne sont pas pinnables : TOOL_UNQUALIFIED/HOLD, sans
client inventé ni wire maison. Sources/API lisibles ne réservent aucun PG.

## Copie ED b472-v1 finalisée et sélection RO — 17 novembre

READY final reçu :14428fichiers/5,526,823,298octets ; manifestSHA
`de797438448c79ae54a1bc313003b0fdead4e37b70b180298ce21690b320c945`.
Audit G UNIQUEMENT dans la copie OUT_T, jamais le T. Graphe des fingerprints
résolu,74artefacts et74fingerprints recoupés aux SHA du manifeste ; aucun
manque/ambiguïté après résolution des alias Cargo postgres-types :
serde_json_1=serde_json ; serde_1=serde_core. La première lecture avant
résolution d'alias est conservée, pas maquillée en paquet manquant définitif.

Directs sélectionnés par le graphe, pas au hasard dans les variantes :
- libtokio_postgres-3179df43d80295e0.rlib :
  02280b8ffdf885a831c878f549fb14d1d9f65de7c991aac6ad457c249431cd28 ;
- libtokio-64cfcb92dcf41713.rlib :
  0dcfafaddd3e85ca419fb8b3944899826b50ec35623d64316a47678b5c6f0882 ;
- libserde_json-452ff4526b3b8a08.rlib :
  94f6a90becfcbae008aeb632b39c39af8ea5d0403e72bb86634a2fe1f3e37ccc.

Tous ces fingerprints portent le même identifiant compiler, lock090 et
cache version1.98.1 recoupés. Rustc859 fichier actuel et copie témoin égaux,
mais aucune preuve prébuild ni compatibilité LINK native déduite. Build/out
natif.a absent ; les scripts de build rencontrés restent listés dans l'audit,
leurs produits éventuels ne sont pas présumés dispensables à tout link.
Rapport RO `~/.cache/optimiseur-notes/pg-ro-attestation-20261116/ed-b472/audit-v2.json`,
SHAaa80f31a1e50550bcae6fd7126c0b3bd267f8360dbebedf43c3d6e63696ad394.
ED conserve le snapshot entier ; G ne maintient aucun lecteur ou hold sur T.

## Source candidate et recette, NON EXÉCUTÉES

Vecteurs rust-client-fakes.json publiés a2c7fd0ac1 AVANT code, huit cas
NONJOUÉS ; aucune qualification native par ces exemples. Source pg_ro_client.rs
et rust-client-plan.json publiées cedf2546b1. Le client emploie les vraies API
Tokio/Postgres/serde_json, pas de wire/auth/crypto maison. Mode explicite,
DSN stdin borné et non journalisé, cible canonique stricte, startupreadonly/
statement/lock/connecttimeouts, deadline intérieure unique<=4s, deux SQL
constantes et fermeture du pilote de liaison avant postcheck. Sans mode,
aucun contact implicite. Ce programme source n'a PAS été compilé ou exécuté.

La deadline intérieure commence après lecture stdin : ELLE NE QUALIFIE PAS
startup/lecture/sortie/reap sous hard5. Le superviseur extérieur hard5/killreap,
les vecteurs fakes et l'attribution backend DOIVENT encore être éprouvés sous
nouveau GO outil. Pas d'affirmation « transport prêt » à partir de la source.

Recette rustc single-file proposée dans le JSON, utilisant uniquement la copie
OUT_T et les trois --extern. Aucun Cargo, exportSDK, C++/build.rs, install ou
changement magasin. Réaudit frais du sysroot/compiler et des pins requis avant
GO, sortie dans un NOUVEAU dossier privé exclusif, refus de réécrire un artefact.
Un échec de link par sortie native manquante sera conservé sans repli automatique
vers Cargo ni rebuild de dépendances. Coût de compile/exécution : NON MESURÉ.
Dockerinspect futur : FORMAT Id/Image/NetworkSettings.Ports uniquement, borne
et quiescence propres encore NONJUGÉES ; aucune lecture Env, aucun dockerexec.

Le module pur/SQLv1 de69/f949/2907 et ses25fakes acquis restent bitidentiques.
Admission de ce nouveau transport Rust reste INCOMPATIBLEv1, à revoir avec
owner/consumer après qualification. Aucune probe, DB, réservation ou GO accordé.
