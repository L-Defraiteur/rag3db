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
