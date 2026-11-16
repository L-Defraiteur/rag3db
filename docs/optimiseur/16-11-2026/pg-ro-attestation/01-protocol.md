# Attestation PG RO future : contrat de préparation

Même ticket Recherche302/301, contrat Embar79cb ; aucun GO, allocation, client
installé ou contact PG/Docker/health à ce stade. Source canonique2150 et
APP84 : bootstrap127.0.0.1:5433/rag3weaver_test/rag3weaver, bases de suites
`rag3weaver_test_<poste>_<pid>` distinctes. Ne pas poser BASE_FIXE.

Le producteur reçoit une allocation G/C7 EXTERNE épinglée avant l'observation.
Il ne transforme jamais des OID/imageID observés en valeurs attendues par défaut.
Owner/consumer reçoivent path+SHA du JSON final ; pas de ping réseau de leur
part. Ils revalident aussi phase/READY/firstred/autorité via98ab ; ce contrat
ne remplace pas cette primitive ni n'autorise des embeddings hors canonique.

## Format exact v1

Clés de premier niveau exclusivement :
`schema_version, kind, app, pilot, scope, c7_reference, g_reference,
reservation_reference, nonce, ready_sha256, observed_monotonic,
valid_until_monotonic, exclusive, private_test, active, topology, sql,
children_policy, evidence`.

- schema_version=1 ; kind=`postgres-read-only-admission`.
- app/pilot SHA40, ready_sha256 SHA64, nonce32hex ; références/scope comparés
  EXACTEMENT à l'allocation externe et au READY observé, pas librement choisis.
- observed_monotonic dans la fenêtre READY et fraîcheur<=120s à publication ;
  valid_until_monotonic>=END commun ORIGINAL. Horloge CLOCK_MONOTONIC LuciePC.
- exclusive/private_test/active doivent être les booléens true, jamais0/1.
- topology : `container_id, image_id, binding`. Identités attendues externes,
  binding exact `{host:"127.0.0.1",host_port:5433,container_port:5432,protocol:"tcp"}`.
  imageID immuable, pas le tag pgvector:pg17. OID seul n'identifie pas le cluster.
- sql : `database,user,database_oid,user_oid,owner,owner_oid,session_user,
  server_address,server_port,server_version_num,postmaster_start,backend_pid,
  application_name`. Bootstrap noms et OID/propriétaire EXACTS attendus ;
  OID entiers positifs, pas booléens. Adresse/port peuvent être NULL pour une
  recette Unix ; ne jamais assimiler5432 interne à5433 hôte.
- children_policy=`per-suite-process-databases` : autorité future sur le cycle
  de vie des seules bases enfants du job, PAS une identité OID inventée avant
  leur création. Aucun CREATE/DROP exécuté par le producteur RO.
- evidence : `query_sha256,observer_source_sha256,transport,client_bundle_sha256,
  started_monotonic,deadline_monotonic,ended_monotonic,returncode,partial,
  truncated,client_reaped,quiet,backend_absent,read_only,stdout,stderr`.
  transport=`private-host-psql`, pin client/observateur externes qualifiés ;
  worker whole<=5s, deadline unique qui inclut hashes/capture/cleanup/postcheck.
  rc entier0, read_only/client_reaped/quiet/backend_absent=true,
  partial/truncated=false. stdout/stderr : `{path,sha256}` de bruts réguliers
  hors données, publiés avant jugement et relus par SHA. stdout JSON une ligne,
  schéma sql exact ; aucune URI secrète ni environnement dans les pièces.

Les faux OID/topologies des filets ne sont jamais des pins d'exécution. Les
valeurs runtime actuelles restent UNKNOWN tant qu'une recette RO distincte
G/C7 ne les a pas recueillies. Zéro clients n'est jamais une réservation.

## Recette concrète et HOLD outil

Préférence : client psql HÔTE privé avec binaire, chargeur et bibliothèques
pinés dans un bundle immuable ; aucun client installé actuellement relevé.
Une autorisation RO outil DISTINCTE doit fournir provenance/container imageID,
binding5433→5432 et client compatible (copie/extraction sans SQL), puis qualifier
son exécution bornée avant toute probe d'admission. Aucun extrait ou install ici.

La future probe utilise ce client DIRECT, sans shell, pager, psqlrc ou prompt,
DSN canonique en mémoire et jamais archivé ; session read_only, connect_timeout
et statement_timeout inférieurs à la borne globale. La requête constante
`identity.sql` n'exécute que SELECT catalogue ; application_name lie le nonce,
pg_backend_pid attribue la session. Le même budget5 englobe observation,
reap du client et vérification indépendante de disparition du backend attribué.
Un timeout, stdout partiel, backend inconnu ou absence de reap reste ROUGE,
conserve les bruts et interdit toute publication de grant.

Alternative Dockerexec non admissible dans v1 : tuer/reaper dockerCLI ne prouve
pas le reap du client dans le conteneur, ni l'absence d'un exec différé. Aucun
VERT CPU ne qualifiera ce transport. Si le client privé et son postcheck ne
peuvent être fournis, attente RO outil JUSTIFIÉE : aucune probe possible.

Le code de préparation/validation ne contient aucun appel réseau, SQL, Docker,
processus natif ni découverte automatique de pins. Il peut assembler une pièce
seulement depuis résultats d'un observateur qualifié externe ; tant que celui-ci
manque, produire un plan HOLD, pas une attestation. Source observateur/transport
réel et son runner devront être relus et qualifiés AVANT futur GO probe.

### Preuve postcheck dans la même borne5

Ajouter à evidence la clé `postcheck` : schéma exact
`query_sha256,started_monotonic,ended_monotonic,returncode,partial,truncated,stdout,stderr`.
Ses bruts sont eux aussi `{path,sha256}`. SELECT constant `backend-postcheck.sql`
avec paramètres GUC de SESSION `g_pg.expected_backend_pid` (entier positif) et
`g_pg.expected_application_name` (`g-pg-<nonce>`). L'observateur porte
`g-pg-<nonce>:check`, distinct du premier backend. Aucun SQL arbitraire accepté.
Son stdout JSON exact : `backend_pid,application_name,backend_absent,
observer_backend_pid,observer_application_name`. Absence par PID OU nom initial,
observateur distinct ; résultat absent=true obligatoire. Cette seconde lecture
reste dans le budget ORIGINAL5 (pas5+5), rc0, sortie complète et stderr vide.
Le premier stdout doit porter application_name=`g-pg-<nonce>` ; le postcheck
n'atteste pas sa propre absence à sa sortie : quiet/reap porte TOUT le client
hôte, dont cette seconde invocation. Toute quiescence inconnue refuse.

## API stable, source pure à importer

`pg_receipt.validate_receipt(receipt, *, expected, ready, now, read_raw,
admission=True, admitted_at=None)` : version pure, ValueError au refus.
`validate_attestation(path, expected_sha, expected_allocation, ready, now, *,
admission=True, admitted_at=None)` : ajoute lecture bornée/UID/type/SHA du JSON
et de ses bruts DANS le même dossier, aucun lien ni chemin de données externe.
Retour : receipt validée, sans fabriquer un grant.

`ready` est un contexte PG normalisé de CINQ clés exactement :
`issued_monotonic, admission_deadline_monotonic, common_end_monotonic,
active, first_red`. Issu du READY/phase98ab relus par le caller ; aucune nouvelle
vérification de phase kernel/proc n'est inventée par ce module offline.

`expected_allocation` a QUINZE clés exactement :
`app,pilot,scope,c7_reference,g_reference,reservation_reference,nonce,
ready_sha256,topology,children_policy,tool_qualified,allocation_active,
sql_identity,observer_source_sha256,client_bundle_sha256`.
`sql_identity` a les onze clés sql sauf backend_pid/application_name : toutes
proviennent d'une attente G/C7 explicite, jamais apprises par le validateur.

À admission=True, fraîcheur<=120 à now et admission AVANT deadline READY.
Après canonique, admission=False EXIGE admitted_at fiable du grant initial
(lié aux pins.admitted_monotonic et grant.observed par owner/R). La fraîcheur
et deadline READY s'évaluent à CET INSTANT INITIAL ; phase, firstred et lease
restent évalués à now. Aucun nouveau SQL, END ou TTL réinitialisé.

`publish_observation(target, receipt, raw, *, expected, ready, clock, phase_check)`
est un assembleur de pièces capturées par un observateur EXTERNE, pas une probe.
raw a quatre clés bytes : sql-out/sql-err/check-out/check-err. Il crée un dossier
exclusif, conserve les bruts bornés avant jugement, refuse partial/rc/SHA erronés,
revalide phase/budget avant puis APRÈS fsync et avant lien exclusif final.
Retour : (artifactpath,SHA). Refus : captures + refusal.json immuables, pas de
path/SHA admissible fourni au caller. Un échec d'E/S après publication est
explicitement enregistré, jamais effacé ; caller doit exiger le retour réussi.
`phase_check` fournit le contexte courant sans changer les trois bornes initiales.
`prepare_observation()` renvoie HOLD/run_authorized=false : aucun runner natif
réel n'existe ici, sa qualification et ses pins restent un prérequis distinct.
