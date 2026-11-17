# Coordination opératoire : acteurs réels avant admission, HOLD actuel

C7 réserve GO7455, parent/rapport17 publiés9f1adca9b3 AVANT action. Trois RO
code+pins restent favorables sur APPf1e8/PILOTE229b ; elles n'établissent pas
un acteur vivant, une réservation ou une publication automatique. Le contrôleur
PC229b, Q/config0e6278 et paquet29/f5c5 restent FIGÉS ; aucun nouveau lancement.
Le [ticket porteur](../tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md)
est ouvert. [Plan37](37-coordination-operatoire-plan.json) fige références/SHA
et HOLD, sans nonce/OID/lease/allocation inventés. Ce plan n'est pas un pilote.

## Écart constaté dans le vrai code229b

Runner.published écrit un ledger et attend90s un JSON de publication sur deux
hôtes, pour global5ELF puis agent-red/green/review1ELF chacun. Aucun producteur
portable effectif n'est relié à cette attente. Runner.acknowledge attend90s
pour D6 puis canonique, mais son request.deadline ne porte que l'attente ACK,
pas self.phase_deadline ni la couverture des1800/3900s et du cleanup. Un ACK
satisfaisant ces seuls90s ne prouve pas une autorisation couvrant la phase.
Le prochain raccord doit expliciter ces bornes avant l'observation des valeurs.
Le producteur doit rester vivant après publication, pas être un ping humain.

## Primitives existantes à composer, sans troisième relais

| Porteur | Source exacte lue | Contrat et état |
|---|---|---|
| Embar |96f3a2a121d0ab65e910a83844e813a0bc794925, coordination/grant_watch_actor.py, SHA789058a8a8d15815e1a1b4083c1201d6ae390a79e9471740c11ab38af31f5175 |Acteur commun publié ;12CPU factices verts selon Embar, aucun import/rejeu ici. Hooks policy.preflight/publish_ready, ARM réel après exec, HB10s jusqu'à RELEASE/END.|
| Codeparsers |cf83228cd3371fba19b1e1272714e9b7d6f5d9e8, rust-probe-relay/relay.py+gate.py |Coordinateur portable/porte PC, transfert deuxhôtes+ACK exclusif après rehash. Prototype spécifique build/runtime, pas D6 générique qualifié. Deux défauts de fermeture encore ouverts.|
| G |pg_receipt.py SHAde69c498b1ac38d80d4976144fea79ddbfa7d3a34ea8ce7dae1d9e4a457aeed6, lu au commit808be0595ffd033e5ffd69d934eedfceee22d174 |Validation offline PGv1. Transport private-host-psql requis, aucun outil compatible qualifié. Aucun grant PG réel productible actuellement.|

Chemins complets et quatre SHA sont dans37. Aucun de ces fichiers n'est copié
ou substitué dans Q229b. Une adaptation future publie un NOUVEAU pin et son
delta avant CPU/RO ; ne pas repeindre les RO229b en avis du code futur.

## Acteurs à raccorder et preuve d'armement

Le contrôleur PC, déjà sous son vrai watchdog et Common original, doit lancer
l'acteur commun en enfant attribué après config/autorité épinglées. Le parent
conserve le Popen ; après le vrai exec et avant préflight métier/garde/build,
il exige poll vivant ET prélèvement OS indépendant PID/start/PPID/UID/CG,
exeSHA/argv complets/scriptSHA/configSHA/nonces, cohérents avec ARM boucle>=1.
Un fichier ARMED isolé n'est pas suffisant. HB courant<=10s et identité vivante
restent requis pendant la phase ; perte/refus/premierrouge interrompt le lot.
Le preflight de politique du watcher précède ARM : outil PG non qualifié doit
REFUSER AVANT ARM, sans GET/SQL/health ni grant. Il ne se dit pas armé en attendant
une future qualification. Aucun acteur de ce lot n'est actuellement lancé.

La politique de phase distingue D6 et canonique, garde provider Vertex et
7878/7879/fixturePG réels sans NoEmbedding substitué. Nonces, allocation et pins
service/PG attendus proviennent G/Embar/C7, jamais de l'observation actual.
READY lie phase, source/pilote, autorité, cgroup, CommonEND original, deadline
phase et admission90. Le lease couvre au minimum cet END original, donc toute
phase ET son arrêt borné ; ni120s fraîcheur ni90s attente ne remplacent ce lease.
Admission PG fraîche<=120s ; après canonique pg_receipt(admission=False) reprend
admitted_at du grant initial et contrôle phase/firstred/lease actuels, sans SQL,
TTL ou END renouvelés. Au rejet, premier défaut immuable puis arrêt attribué.

Le watcher reste vivant jusqu'à RELEASE explicite après phase/cleanup, ou END.
RELEASE puis résultat/reap doivent rester dans la même enveloppe originale.
Une fin tardive/inconnue refuse ; pas de «0restes» depuis une boucle non entrée.
La fin du coordinateur/localSSH ne vaut pas preuve de fin native distante.

## Publication deuxhôtes avant chaque corps de test

Réutiliser le moteur Codeparsers corrigé avec adaptateur des descriptions de
payload ; ne pas écrire un troisième transport/arrêt/horloge. Le protocole
actuel a DEUX stages build/runtime spécifiques à sa sonde ; adaptation au
ledger global5ELF et aux3ledgers D6 séparés à écrire et à relire, pas existante.
Avant premier ledger : coordinateur portable vivant sous watchdog propre,
identité réelle/ready publié avant SSH, bootstrap PC réel puis identité après
exec, pins des deux rôles vérifiés. Le contrôleur ne commence pas préflight
métier sur simple ready fichier ni sur disponibilité déclarée en message.

Gate PC ne fournit que des durées restantes ; borne portable ancrée AVANT RPC,
latence soustraite. Jamais comparer les monotonic absolus entre hôtes ni ajouter
300s à la borne7455 du lot global. Chaque transfert/rehash/fsync/link/résultat et
fermeture consomme le même temps restant ; garder la borne physique indépendante
PC déjà présente. La marge d'arrêt est réservée AVANT épuisement, pas acquise
par appel cap() après expiration.

Chaque export vérifie source/pins/autorité active/premierrouge/END AVANT et APRÈS
copie ; ledgerJSON complet+ELF temporaires→rehash→fsync→lien exclusif, deuxhôtes,
puis ACK lié à ledger/source/pilot/ELFSHA/acteur/autorité. Le lien final est refusé
si phase rouge/terminée/expirée. Le driver revalide copies/identités/lease avant
runtime. Fin SSH anticipée, hash faux ou timeout : première cause immuable,
pas d'ACK ni nouvelle copie ; fermeture des seuls enfants attribués et refus
distant borné, sans arrêter ni interroger un service partagé hors autorisation.

## Blocages réels confirmés par les propriétaires

G : aucun psql/libpq/binding hôte qualifié. Nouveau Rust808be/paquetb9b0 est
NONQUALIFIÉ et INCOMPATIBLE transportv1 ; réserve watchdog parent hash/fsync en
cours. Ticket existant
`docs/recherche/tickets/2026-11-16-la-garde-des-comptes-en-blobs-reste-a-302-documents-pour-301-lignes.md`,
porteur G outil/capture. Sans outil qualifié puis vraie capture RO autorisée,
nonce/OID/lease externes manquent : ne pas fabriquer une pièce ou un grant.

Codeparsers : except/cap expiré empêche abort ; Gate.cancelalive=[] si boucle
non entrée peut annoncer quiet sans observation. Trois témoins publiés598f3851227071eb61241949ba56cae7a69d3844,
NONJOUÉS au retour propriétaire, correctif de ces deux points non implanté.
Ticket
`docs/codeparsers/tickets/2026-11-17-la-publication-du-ledger-de-sonde-attend-un-ack-sans-relais-autonome.md`.
Ne pas utiliser les20CPU d'une ancienne version pour qualifier ce raccord.

Suite : obtenir versions corrigées/qualifiées des mécanismes propriétaires,
puis matérialiser l'adaptateur global et les témoins des vrais raccords avant
nouveau pin/RO/GOCPU. Jusqu'alors préparation de contrat seulement, aucun acteur
ARMÉ ni grant réellement productible revendiqué. Aucun health/SQL/provider/
poste/Cargo/rustc/service/runtime/target actuel effectué. GO7455 reste retenu.


### Précisions propriétaires après publication du plan initial

G précise sourcepg_receipt commit8ddb87233a22eda7122c04f2b4ba8b1656577550,
SHAde69 identique au blob lu808be. Postcanon ici signifie validation de la pièce
existante avec admitted_at initial ; cela ne dispense PAS du postcheck SQL
prévu dans la qualification du producteur/sa capture initiale (même borne5s).
Réserve runner parent consignée b56d0ed0f5 avant correction en préparation.

Codeparsers a depuis observé ses3rouges598f et prépare le correctif groupé,
encore non qualifié. API actuelle : ready/start_phase/running/snapshot(timeout),
fetch(stage,name,destination,timeout), publish_ack(stage,ack,timeout), finish,
abort,stop. Le snapshot fournit remaining/phase_remaining DURÉES PC, stage,
ledger et files{name:SHA}. Le futur manifeste ordonné global+3D6 doit composer
cette logique commune après qualification, pas un fork ni une API4pub déjà
livrée. Aucun nouveau run ou test n'est exécuté par Archi pour ces précisions.
