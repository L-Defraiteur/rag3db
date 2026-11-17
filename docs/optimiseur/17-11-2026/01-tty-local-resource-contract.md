# TTY local — contrat de ressources inactif

Préparation C7 suivi5b6ade163c, même ticket
`docs/tty/tickets/2026-11-16-les-commandes-et-options-du-terminal-en-anglais.md`.
Aucun GO3600, ACK, réservation ou lease réel. Ce contrat permet de construire
le pilote/Q désarmé ; il ne rend aucune ressource admissible.

## Topologie nominale et responsabilités

| Consommateur futur LOCAL | Destination nominale | Porteur |
|---|---|---|
| http://127.0.0.1:7979 | service PC http://127.0.0.1:7878 | G transport/tunnel, Embar service/modèle |
| http://127.0.0.1:7980 | service PC http://127.0.0.1:7879 | G transport/tunnel, Embar service/modèle |
| PostgreSQL 127.0.0.1:5433 | instance de test LOCAL, mapping interne5432 à attester | G identité/topologie/allocation, C7 admission, TTY consommateur |

LOCAL désigne le poste du futur canon TTY, à identifier dans son allocation.
7979/7980→7878/7879 est le mapping documentaire historique portable→PC
(`docs/optimiseur/16-11-2026/02-passation-pour-codex.md`), pas une observation
fraîche de tunnels. Ne pas rediriger le client TTY vers les ports PC à sa place.
Aucun tunnel PG5433 vers PC n'est affirmé. Les instances PG LOCAL et PC ne
sont pas interchangeables, même si elles partagent les noms/ports attendus.
Aucune santé ni disponibilité actuelle attestée. Les modèles/health/politiques
sont portés par le contrat Embar distinct ; pas de cycle noembedding substitué.

## Identité PG attendue et limites

Noms bootstrap issus du contrat source : database=rag3weaver_test,
user/session_user/owner=rag3weaver attendus à confirmer explicitement par
allocation, sans mot de passe ni DSN dans ce document. Source fixture
`extension/rag3weaver/tests/common/base_pg.rs` SHA256
`41491f5a21191fb9256d0eb76210decfa66c698ecb2f347ef4f3170a8adfb067`.
Les suites créent leurs bases `rag3weaver_test_<poste>_<pid>` et nettoient
leurs enfants ; aucun flag BASE_FIXE ajouté pour contourner cette provenance.
Le producteur d'identité RO ne fait ni CREATE ni DROP.

UNKNOWN pour LOCAL : container_id/image_id/binding réellement utilisés,
OID database/user/owner, version serveur, adresse interne, postmaster_start,
réservation et processus consommateurs. Ne pas transformer une future valeur
observée en attente de sa propre validation. Un port ouvert ou zéro clients
ne prouve ni identité ni réservation exclusive. Aucune création de DB/instance,
inspection Env, SQL ou transport nécessaire à la présente préparation.

PGv1 existant : source relue8ddb87233a22eda7122c04f2b4ba8b1656577550,
`docs/optimiseur/16-11-2026/pg-ro-attestation/01-protocol.md` et
pg_receipt.py SHA256
`de69c498b1ac38d80d4976144fea79ddbfa7d3a34ea8ce7dae1d9e4a457aeed6`.
Il exige private-host-psql et CLOCK_MONOTONIC PC. Il ne s'applique donc pas
silencieusement à un observateur LOCAL : domaine d'horloge/hôte et transport
qualifié devront être explicitement relus/pinés avant admission future.
Le candidat Rust reste incompatiblev1 et non qualifié. PhaseB01 a refusé
l'autorité Common avant rustc : aucun ELF, CLI ou transport acquis.

## Allocation, durée et restitution futures

TTY prépare source/pilote/Q exacts et manifeste désarmé. Avant un nouveau GO,
G/C7 doivent fixer hôte LOCAL réel, références source/pilote/Q/recette, scope
Common original, nonce/READY frais, identité attendue indépendante et
réservation explicite exclusive de l'instance de test pour tout l'usage.
Embar attribue ses ressources modèles avec phases et END couvrants ; G
attribue séparément transport et identité PG. Aucun message humain ancien
ne vaut ACK runtime ni grant automatique.

Composer l'acteur/contrôleur commun Arbre-Embar : source publiée2dccc070f1564d8ca9d22a59fcb2b73222f415e5,
acteur STRICT4f54, sous docs/orchestration/17-11-2026/coordination.
Ces pins sont des références de préparation communiquées par leur porteur,
pas une RO globale ou preuve d'acteur vivant émise ici. APIs supervise,
read_admission/check_scope ; avant ARM, preflight doit refuser si transport
PG non qualifié ou attentes/allocation manquantes. Après vrai lancement
autorisé : preuve PID/start/cgroup+liveness/heartbeat AVANT READY, grants
liés à source/pilote/Q/phase/nonce et lease couvrant END original et tout usage.

Fraîcheur PG120s à l'admission initiale, avant deadline READY ; valid_until
couvre END commun original. Après canon, relecture des mêmes pièces avec
admitted_at initial fiable : phase/firstred/lease courants, aucun TTL, SQL ou
END réinitialisé. Le futur contrat LOCAL devra garder ces invariants avec
son domaine d'horloge propre. La preuve d'absence backend SQL attribué est
une lecture séparée prévue dans la même borne d'observation, pas un serveur
lancé pour le test.

Premier refus arrête les actions dépendantes et garde les preuves. L'acteur
reste surveillé jusqu'à RELEASE/END ; restitution liée au même scope/nonce,
résultat RELEASED ou REFUSED selon contrôleur, arrêt borné et preuve de
quiescence des seuls processus attribués. Aucune opération service/tunnel ou
suppression de données partagées implicite. Sans preuve fraîche à chaque
frontière, HOLD ; le Q peut être construit mais aucun run ne doit être admis.

Le JSON voisin est volontairement de type préparation, pas un grant ou une
allocation consommable. Toutes ses identités runtime restent null et inactive.
