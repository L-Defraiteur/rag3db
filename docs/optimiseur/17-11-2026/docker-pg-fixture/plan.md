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
internal nouveaux. Aucun volume/bind de données existantes : PGDATA tmpfs512M,
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
poll pg_isready0/1/2 est attente bornée60s sur CETTEinstance, pas une nouvelle
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
