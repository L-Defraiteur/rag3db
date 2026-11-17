# PG phase C — contrat proposé, inactif

Même ticket Recherche302/301 ; demande C7 3bbea030cf. Lecture source et
préparation documentaire uniquement. Aucune DB/DDL/contact/connect/SQL,
Docker/service/natif ni lecture de credentials. B-r4 reste une qualification
compile/link/parsing sans contact, transport_qualified=false.

## Point de départ vérifiable

Source approuvée B f4e615b5bd781810fc03358b2d7e3cd6363f2a5d ; Rust SHA256
86b10a135b1c09a859e0abd4c5ab78d126cc624069e143b9e5f22afeae9095ce ; ELF B
09ba7f2e20843e8bc085099a11ff3d4cfe9f7b5a282c380c62ac287e41924e2c.
Preuves B/ledger neufPID/CommonTERMINE : rapport03 du dossierpg-ro-attestation,
commit e3f1674280. Identité SQL source f9496688a506085562a071ce25827a2a683bccd457c9edf6e944e228b7c29328,
postcheck2907a86d971770275b9ff6291e9bffebdf4467d23c7c59ff2015ae6a3fb9c864.
Le présent document ne modifie aucun de ces fichiers ni les pièces historiques.

Le candidat sait parser une seule cible127.0.0.1:5433, database et role
rag3weaver_test/rag3weaver. Seuls [] et --validate-request ont été joués.
--capture-readonly existe en source, jamais exécuté. Il ouvre deux sessions
séquentielles (identité puis postcheck), sans SQL d'écriture intentionnel,
avec options startupdefault_transaction_read_only=on, statement_timeout1000ms,
lock_timeout500ms, connect_timeout1200ms et NoTls sur loopback strict.
Ces options ne prouvent pas les droits du rôle ni l'état transaction_read_only.
L'identité et la version de l'instance réelle restent UNKNOWN.

## Cible isolée choisie pour la proposition C

PC uniquement, domaine CLOCK_MONOTONIC PC. Endpointnominal127.0.0.1:5433,
instance TEST dédiée et réservation exclusive accordées EXTERNEMENT par G/C7
avant une observation ; jamais mémoirelongue/codebases/instance de Lucie.
L'allocation doit nommer containerID+imageID immuables ou identité de service
équivalente explicitement versionnée, bindinghôte5433/interne5432, hôte stable,
cluster/postmaster_start/version, DBnom/OID et propriétaire/OID, rôle/OID et
session_user. Aucune valeur observée ne devient automatiquement sa propre
attente. Aucun inspect Docker ni provisionnement dans la probe C.

Rôle proposé : observateur dédié `g_pg_observer`, LOGIN mais NOSUPERUSER,
NOCREATEDB/NOCREATEROLE/NOREPLICATION/NOBYPASSRLS, non-propriétaire de la DB,
sans héritage/SET ROLE vers un writer, sans CREATE schéma/DB, TEMP, DML/DDL
ou EXECUTE de routines d'écriture. Accès uniquement catalogues/état nécessaires
à ces SELECT constants et au contrôle de ses propres backends attribués.
Le bootstrap de suites a des responsabilités CREATE/DROP : ne pas emprunter
ses credentials en le rebaptisant readonly. Les droits effectifs attendus,
PUBLIC/héritage inclus, nécessitent attestation administrative indépendante ;
la probe ne crée ni ne révoque aucun droit et ne fournit pas cette attestation.

DB de diagnostic proposée `g_pg_probe_<allocation_nonce>` déjà allouée dans
l'instance test réservée, nom/OID exacts externes, propriétaire distinct.
Ce nom est un schéma de réservation, pas une DB existante annoncée. La
provision éventuelle relève d'un autre GO explicite G/C7 ; aucune DDL ici.
L'observation initiale ne produit aucun grant de canonique sur cette DB de
probe. Une future admission du bootstrap rag3weaver_test exige une allocation
et une observation distinctes sur CE bootstrap, pas la réutilisation de C.
Le contrat enfantsper-suite-process-databases reste porté par le propriétaire
du futur job ; ce rôleRO ne peut ni créer ni nettoyer ces bases.

Conséquence source : prepare_config actuel refuserait ce rôle/DB ; il faut un
nouveau candidat piné qui compare exclusivement au nom/role de l'allocation
privée tout en gardant loopback/port/protocole fixes. Pas de fallback vers les
noms actuels, pas de réécriture d'un DSN étranger, pas d'ouverture avec l'ELF B
avant ce changement/revue. C'est un prérequis concret, pas une compatibilité
ou un provisionnement déjà acquis.

## Session RO et requêtes à figer avant nouveau bâti

Nouvelle source C devra utiliser une transaction explicitement READ ONLY
pour l'identité/état et pour le postcheck indépendant. START/COMMIT et les
SELECT sont des constantes compilées, aucun SQL ou identifiant utilisateur
interpolé. Le rôleRO et les options startup bornées restent défensecomplémentaire.
Le contrôle transaction_read_only doit être observé DANS la transaction où
les catalogues sont lus, pas une valeur déduite d'un ancien SHOW ou de l'env.
Les frontières START/COMMIT changent les CommandComplete attendus : le parseur
actuel exige un seul commandement/une ligne, donc doit être revu/testé avant C.

Plan de constantes séparées et SHA à publier : identité14champs existante ;
étatRO (transaction_read_only, default_transaction_read_only, session_user,
current_user, superuser/CREATEDB/CREATEROLE/REPLICATION/BYPASSRLS, rôle/DBOID,
propriété et droits effectifs attendus) ; postcheck existant lié PID+nonce.
Ces nouvelles constantes ne sont pas encore du SQL livré/exécuté ; leurs
formes exactes et limites doivent faire l'objet du lot source et des témoins.
L'identité du propriétaire peut être distincte du rôleRO, comparée à l'attente.
Ne pas muter les deux requêtes/validateurs v1 gelés pour y ajouter des champs.

Le postcheck cherche l'absence du backend identité par PID OU application_name
`g-pg-<nonce>`, observateur distinct `g-pg-<nonce>:check`. Le nonce32hex et le
PID positif typé servent de paramètres GUC bornés ; aucun SQL arbitraire.
Un résultat backend_absent=false ou observation incomplète refuse. La fermeture
et le reap du client portent aussi le driver de postcheck ; pas de revendication
circulaire d'un SELECT qui prouverait sa propre absence après fermeture.

## Timeout, annulation et quiescence à implémenter/qualifier

Constat source : query_closed fait timeout_at(...).await? avant drop(client)
et join(driver). Sur erreur/timeout/shape invalide, ce chemin ne prouve donc
pas le join ni l'absence backend ; pas de CancelToken explicite conservé.
La destruction du runtime/socket n'est pas un postcheck du serveur.

Nouveau chemin requis : obtenir/attribuer PID et application_name dès
connexion, garder driver+CancelToken sous un owner de session. Sur rouge,
conserver d'abord les bruts/premièrecause ; arrêter toute requête métier.
Annulation via primitive tokio-postgres dans le budget ORIGINAL restant,
puis drop/fermeture/join du driver et postcheck d'absence attribuée seulement
si ce nettoyage est explicitement prévu et dans le même END. Ce nettoyage
est marqué cleanup, ne peut jamais convertir un rouge en vert ni lancer une
nouvelle tentative. S'il n'y a plus de temps, outer kill/reap du seul groupe
attribué ; backend_absent reste UNKNOWN, refus sans grant. Pas de nouveau
budget ni de requête après END pour rendre les comptes plus jolis.

Tout descendant natif/driver/socket doit être attribué, tous stderr/stdout
bornés/partial/truncated/PIDstartCG conservés avant verdict ; aucun claimquiet
sur une erreur masquée. Les fautes de bus/lockdir et le contratFD9/devices
corrigé restent des gardes, pas des raisons de prendre un autre poste.

## Borne proposée et évolution explicite du protocole

Proposition à relire, PAS une promesse de durée : travail C whole10s incluant
startup/stdin/chargements/hash/contrôlesRO/connexions/SQL/annulation/fermetures/
postcheck/reap/rawpublication/retour ; sous Common30s et OS35sTERM+KILL2 pour
la portée entière/finalisation. Timeout connexion1200ms, requêtes1000ms,
locks500ms restent plafonds internes MINEND, pas des budgets additionnés ou
renouvelés. Réservecleanup explicite dans les10s, à dimensionner par témoins.
Le superviseur doit englober tout le worker et ses E/S ; une simple horloge
mesurée après un I/O bloqué ne prouve pas une interruption physique10s.
Les limites Common/OS et la quiescence devront être réellement qualifiées.

PGv1 exige private-host-psql et whole5s ; le Rust/whole10s proposé n'est pas
v1. Propositionv2 distincte à concevoir/tester : schema_version2, kind
postgres-read-only-admission-v2, transportstandalone-tokio-postgres, host et
clockdomain explicites, preuves transactions/droits/session et budgetwhole
explicite lié au manifeste, refus des versions/transport inconnus. Ne pas
fabriquer un v1 avec un autre client/borne, ni ramener10 à5 sur le papier.
Une qualificationoutil C initiale produit un rapport de probe, aucun grant
v2 automatique. Validateur, producteur, owner et consommateur futurs doivent
accepter v2 par un lot distinct relu/piné ; Q gelés existants restent inchangés.
Le validateur v1 de69c498 et ses requêtes gardent leur contrat d'origine.

## Allocation, credentials et lease

Owner de l'allocation/reservation : G, arbitrageC7 ; owner services/modèles et
acteur commun : Embar/Arbre selon contrat, consommateur futur : job épinglé.
Le manifeste inactif ne remplace ni réservation d'une cible connue ni actorARM.
Avant runtime : source/pilot/Q/recette/observer/ELF/SQLpins, GO précis, scope
Common original, nonce/READY/firstred, identité cible attendue indépendante,
lease active exclusive couvrant tout l'usage et ENDoriginal. STOP si une pièce
manque, pas d'ACK humain ancien ou d'absence de clients transformée en grant.
Freshness120s à l'admission initiale, puis admitted_at initial fiable et
phase/lease actuelles jusqu'à RELEASE/END ; aucun reset d'horloge entre hôtes.
C PC ne devient pas TTY LOCAL sans nouveau contrat clock/transport/allocation.

Credentials : référence protégée externe G seulement, path/UID/mode/revision
à fournir au futur GO ; valeur, chemin actuel et hash du secret restent UNKNOWN
ici. Aucun .env/.vault lu, copié ou hashé pendant la préparation. Au runtime
futur, lecture minimale autorisée dans le worker, DSN en mémoire via pipe stdin,
jamais argv/env/log/request.json. Le runner B archive le champ stdin en clair :
il ne DOIT PAS être utilisé tel quel pour une vraie requête avec credentials.
Nouveau harness C devra garder seulement identité nonsecrète du fournisseur,
nonce/pins et résultat de permission ; aucune dump d'env/DSN, core ou sortie
native contenant la requête. stdout/stderr client doivent rester génériques.
Le natif ne reçoit aucun jeton Common, lockdir ou accès d'owner par son env.
La recette ne désactive pas la validation TLS à distance : cible loopback
strictement allouée ; tout host distant ou Unix non prévu est refusé.

## Filets et preuves manquants avant tout GO C

- Vrai parseur cibleallouée : host/port/role/DB/nonce/pins étrangers refusés avant
  connexion, aucun fallback ; doublons JSON/champssupplémentaires/NaN refusés.
- Transport fake au niveau bibliothèque/source : transactionsRO vérifiées,
  droitsowner/writer/PUBLIC/héritage invalides refusés, rowshape/COMMIT en erreur.
- Chaque erreur connect/query/shape/timeout/cancel/join/postcheck : raw et
  firstcause gardés, driver toujours attribué, cleanupMINEND sans nouveau SQL
  métier ni succès après rouge, aucun backend_absent inventé.
- Startup/stdin lent, sortie tronquée, fsync/scan lent, descendants/driver
  survivants : refus borné, kill/reap propre attribué, runtimeOS réel à qualifier.
- Credentials canaris : aucun secret dans bruts/manifests/argv/env/proofs ni
  messages d'erreur ; harness C distinct de l'archive stdin B.
- V2 : refus v1/transportinconnu, cap/clock/host manquants, noncephaseallocation
  incohérents, replayREADY/lease expirée/admitted_at absent, publicationtardive.

Livrables proposés dans l'ordre : lot source/tests avantcorrectifs ; RO totale
contrat+code ; nouveau paquet immuable et ELF/source/lock/pinsSQL ; CPUdistinct ;
qualificationprocessus/fixture isolée sous GO distinct ; allocation/attentesRO
et référence credential protégée ; paquet/RO/GO C précis avant connexion.
Aucune étape n'est implicitement autorisée par ce document. Les anciens rouges
et les verts B sont conservés dans leur portée exacte ; FTS302 reste ouvert.

## Complément RO R — timeout avant attribution serveur

RO contrat ea3bb178e7 favorable bornée source/doc/inactif, aucun runtime admis.
Si connect expire AVANT acquisition PID et CancelToken, ni serveur attribué
ni jeton d'annulation n'existent côté observateur. Le statut serveur est
UNKNOWN : ne pas inventer une annulation possible, un PID, backend_absent=true
ou une quiescence globale depuis la fermeture du socket client. Arrêt/reap
attribués côté OS et conservation des pièces restent requis dans END ; résultat
REFUSED, aucun grant ou lease admis. Une recherche négative par application_name
nonce n'est pas une preuve globale d'absence ni d'exclusivité du serveur.

Si CancelToken existe mais pas encore le PID (connexion réussie, SELECTidentité
non terminé), seul le token réellement acquis peut servir au cleanupMINEND ;
l'attribution PID reste UNKNOWN. Même avec un cleanup apparemment réussi,
le rouge initial ne devient pas vert et aucun backend_absent complet n'est
inféré. Ajouter les deux cas au futur harnais source/fixtures avant correction,
avec bruts partiels et preuve qu'aucun appel cancel n'est fait sans token.
Le présent ajout est documentaire : aucune connexion ou query n'a été tentée.
