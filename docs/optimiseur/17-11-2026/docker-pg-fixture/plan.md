# Allocation privée Docker — C7-PG-DOCKER-FIXTURE-17nov

Autorisation explicite Lucie + C7 986d0f8cff : mise en service TEST autorisée
après publication de cette source/recette, sans nouvelle attente humaine.
Même ticketFTS302. Ce n'est PAS le GO du client Rust C ni une qualification PG.

Inventaire RO PC : Docker29.9.2, endpoint unix:///var/run/docker.sock,
image locale linux/amd64 `postgres:17-alpine`, imageID ET RepoDigest exacts
`sha256:b0f9560a2de083e2cc7382e75f808c7381a32852a7ec49117deedb300e552b24`.
Pas de pull/latest, imagepgvector existante/autrescontainers intouchés.
Bindtest127.0.0.1:15433 libre ; recontrôle avant entrée. Le port5433 partagé
reste intact. Inventaire brut sélectif sansEnv dans cacheprivée
`/home/lucied/.cache/optimiseur-notes/pg-v2-fixture-20261117-r1/inventory-01`.

## Une initialisation, bornes et TTL avant start

Nonce32hex frais, noms/labels ownerG et nonce pour container ET réseau bridge
normal nouveaux. Aucun volume/bind de données existantes : PGDATA tmpfs512M,
run16M/tmp32M/shm64M, rootfsreadonly. Binds uniquement sourceRO/credentialsRO
nouveaux et minusculejournalLifecycleRW privé. Docker2GiB+swap2GiB/2CPU/
pids128/logsjson1MiB×2, aucun privileged ni socketDocker monté dans le container.
Les bornesDocker sont propres : le Common client ne borne pas le daemon.

Avant start : timer transientuser NOMNONCE-cleanup, collect, on-active1750s,
MemoryMax128M/CPU10%/RuntimeMax30s/TimeoutStop2s ; sourcecleanup+seedSHA pinés,
aucunservice original modifié. Installé depuis l'appelant hors Common (pas de
transplantation du busLucie dans son busprivé), reçu unitsActive collecté.
Le nettoyage ne cible que ID/name/image/labelsnonceownervérifiés, container
puis réseau puis deuxnouveauxfichierscredentials, et conserve son reçu.
Backup configuré dans entrypoint AVANTpostgres : TERM à1740s puis KILL dixs
après au seul PID1 du namespace privé ; Docker--rm retire container/tmpfs.
La marge tient sous1800sMAX, pas de daemonoublié. Le timer nettoie réseau et
archiveactions même si l'opérateur a fini. Les ancienscontainers sont exclus.

Setup : vraiPOSTE32G→Common6439 délai180s→timeoutOS185sTERM+KILL2 (187MAX
aprèsPOSTE). Provisionneur réserve au moins10sEND et captravail150s ; cleanup
attribué20s au premierrefus, aucun retry/recréation/pull/substitution. Readiness
poll PID1=postgres puis pg_isready0/1/2 est attente bornée60s sur CETTEinstance, pas une nouvelle
initialisation ; toute erreur inattendueSTOP. Timer+backup restent en sécurité.

## Cible/credentials/dataset et vérification indépendante

Bootstrap admin `g_pg_fixture_admin`, DB `g_pg_probe_<nonce12>`, observer
`g_pg_observer` nonowner/NOSUPERUSER/NOCREATEDB/NOCREATEROLE/NOREPLICATION/
NOBYPASSRLS/NOINHERIT/sansmembership, defaulttransactionreadonlyon. PUBLIC
CONNECT/TEMP/CREATE DB et privilègespublicschema révoqués ; observerCONNECT,
USAGEfixture/SELECTsamples seulement. Fixture publique3lignes : red2/blue3/
green5, count3/sum10, contrôles indépendants de l'outilC avant sa connexion.
AdminDDL et juges psql fournis par l'image sont autorisés UNIQUEMENT pour
l'allocation privée ; aucun clientRust/transportC exécuté implicitement.

Deux motsdepasse neufs aléatoires, fichiers0600 sousprivate700, POSTGRES_PASSWORD_FILE
référence montéeRO ; SQLstaticpsql lit le secretobserver depuis son fichier,
pasde valeur en argv/manifest/Envinspect/runnerB/stdin archivé. Aucun credential
original/.env/.vault lu/copié. Sorties CLI bornées et valeurs générées filtrées
avant archivage ; DockerCLI whitelist HOME/DOCKER_CONFIG privés et endpoint
unixfixé, pas lectureconfigauthoriginale. Les secrets ne sont pas répliqués.

Vérifier adminmetadata droits effectifs/grants/membership/OID/image/PID/port/
limites/mounts/labels, dataset exact ; second juge psql sous rôleobserver sur
socketprivé vérifie READONLY/count3/sum10. Ce n'est pas une admission de clientC
ni une preuve TCP du futur transport. Dockerinspect formats ciblés sansEnv.
Identités et logs sanitisés/publicallocation/proofs copiés aux deuxhôtes avant
fin. Credential_reference pointe seulement le fichier privé de ce propriétaire.
AllocationREADY jusqu'àTTL est distincte de transport_qualified=false/leasefalse.
Au TTL/cleanup : uniquement ressourcesnonce, logs/reçus conservés, secrets
inactivés/supprimés. Tout premierrefus garde causebruts et termine la tentative.

## Tentative01 close ROUGE, correction à relire avant nouvelle entrée

Recettef2f30d29/nonce0491f613c35b : rôles/dataset et plafonds vérifiés, puis
NetworkSettings.Ports 5432/tcp:null sous réseauinternal. Le garde publication
refuse ; aucune allocationREADY. Cleanupcontainer+réseau+deuxsecrets acquis,
timerown arrêté, CommonROUGE/restes[]/nettoyé,23preuves sanssecret deuxpostes.
Les logsserveur n'ont pas été capturés avant suppression : lacune conservée,
aucune reconstitution. CorrectionSOURCE à préparer : bridge normal nouveau,
portloopback toujours vérifié strict, logs bornés/sanitisés avantcleanup.
Pas de nouvel essai sous l'ancien GO ; recette/source/nonce nouveaux distincts.

Sourcecorrective : réseau --driverbridge NEUF normal, pas --internal ; même
publication127.0.0.1:15433 et garde NetworkSettings.Ports exact, refus nommé
explicitement effective_loopback_port_binding_mismatch. Aucun portpartagé,
aucun réseau existant modifié. Au rouge, dockerlogs120lignes bornées/sanitisées
capturés AVANT cleanup si container encore disponible, sans modifierpremièrecause.
Cette source seule ne réouvre pas le GO01 ; nouvelle recette/nonce nécessaires.

## Recette corrective02 préparée, pas de nouvelle entrée

Source56850c399619df7cdf0752f9908292e224b14a65, bridge neuf normal et garde
portstrict inchangé/logsrefusavantcleanup. PCcacheprivée allocation-032a992c3cd6,
recipe.json SHA256
f9b74554198a71bb87e0ab09b5f119c1f97accc58fd19e328c08a62341c8e379 ;
seedSHA5f06ec84d99f86850a11a4d990398f56905fb12357ab9b779bbb03755d7ed733,
nonce032a992c3cd683319698a64f37b1d4c7. RéférenceGO en attente/runfalse,
aucun timer/container/réseau ni credentials02 créés. L'initialisation01 est
close, pas réutilisable ; une entrée opérationnelle distincte doit lier ces
pins avant de lancer02. L'autorisation Lucie Docker n'est pas redemandée.

## Allocation02 READY, distincte de la qualification client

GO489bdd7674, source56850c3996/recettef9b74554198a71bb87e0ab09b5f119c1f97accc58fd19e328c08a62341c8e379,
UNEentrée rc0/1.238898207s, Commonvalidation-phzaapyj VERT/restes[]/nettoye/
etat=termine/busretiré. Source/sourcefiles inchangés. Aucun clientRustC exécuté.
Container27cf25b3d531060337cdcf0d605424a74aabab9dd7e59d82a375805a5e1d9525,
réseauc33ba20e08588ca034b581bd74429e593c39996c6fb7623eb4839345bb2936de,
nonce032a992c3cd683319698a64f37b1d4c7, image17pinb0f956...552b24.
Mapping effectif strict127.0.0.1:15433→5432 et Docker2GiB/swap2GiB/2CPU/pids128
confirmés parsnapshot ciblé sansEnv. Données en tmpfs, aucun volumeoriginal.

DBg_pg_probe_032a992c3cd6/OID16384, propriétaireg_pg_fixture_admin/OID10,
observateurg_pg_observer/OID16385 nonowner ; tousflagsadmin/inherit/membership/
CREATE/TEMP/schemaCreate/DML refusés selon contrôlesadminindépendants.
CONNECT+SELECTfixture seulsgrants dédiés ; defaulttransactionreadonlyon.
Dataset3lignes(red2/blue3/green5)/somme10 exact ; jugepsqlimage sous rôleobserver
confirme transaction_read_only=on/count3/sum10. Serverversion170011,
postmaster_start2026-11-17 13:37:45.187523+00. C'est l'identité de CETTEfixture,
pas celle du PGpartagé5433 ni un identifiant global fourni par OIDseul.

Timer nonce actif/waiting vérifié, limiteexpiration PCmono1270341.372028805
auplustard (max1800aprèsstart, cleanupattribuéetbackupPID1). Ne pas prolonger
la fixture ou recycler son allocation aprèsTTL. Réfcredentials protégée
PCallocation-032a992c3cd6/credentials/observer-password, UID1000/mode0600 ;
valeurs nonarchivées/noncopiées. Timercleanup seulowner retirecontainer/réseau/
secretsnonce, autresressources inchangées. Leasegranted=false/transport=false.

27preuves publiques/logssanitisés vérifiésSHA/taille deuxpostes, rootPC
`/home/lucied/.cache/optimiseur-notes/pg-v2-fixture-20261117-r1/allocation-032a992c3cd6`,
local voisin allocation-032a992c3cd6-public-proof. ReceiptSHA256
16e2caf94a9fa95f55b23b668d615cc5f1337ce351b83fa3f2c61099b2d289bc ;
allocationSHA66d974fe37e41d0142a4d2e68b3c6246ff8e416db7789b11306fe8daf4aad193 ;
manifest39b81afa95ec491efc77fa810f8024fffbe73963e0b9f8067ae9cabe777437d9 ;
jugeobservere75ba3c56858381e3fd2ec978e30c30ea896b4430fe1a908a06d35119d061742.
Ancien01ROUGE conservé, aucunthéorèmegénéral surdriverDocker déduit de son
mapping absent. READYallocation ne permet aucune connexion du futurclientC
sans source/tests/rouge/RO/pins/GOcontactdistincts.
