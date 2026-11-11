# Les appels PostgreSQL concurrents peuvent bloquer la session épinglée

- **État** : ouvert, blocage réel du lot 1a ; cause et préexistence à confirmer par témoin direct borné.
- **Famille** : produit
- **Gravité** : ingestion bloquée dans une transaction, sans attente de verrou SQL.
- **Atteignable en service** : oui via ingest_entities du lot 1a ; antériorité hors 1a non établie.
- **Touche rag3weaver** : oui, connexion PostgreSQL et progression de son runtime pendant les appels concurrents.
- **Pour** : arbre-principal, coordination recherche/orchestration avant correctif partagé.
- **Signalé par** : témoin SQL privé des versions/occurrences du lot 1a.

## Preuve et correction du diagnostic de phase

Source b6f78bf87, `cx-contenu-pg-v4.log` sur luciepc, base privée cx_contenu_2191_20261111.
Le test a dépassé 60 secondes puis a été arrêté seul (PID1166384, chemin et filtre relus).
Trois sessions privées, une idle-in-transaction, aucun bloqueur SQL ; gdb refusé par Yama,
aucun contournement. Connexions closes après SIGTERM.

**Le contrôle SQL des données persistées corrige la phase initialement supposée** :
seulement les deux racines alpha et leur version sont validées ; aucune bêta n'est écrite.
Le blocage est donc dans la deuxième ingestion atomique, avant les lectures publiques,
pas encore dans rechercher. La dernière requête de la session épinglée est une lecture
meta par préfixe. Aucun résultat de lecture publique PG n'est revendiqué.

## Hypothèse bornée et témoin suivant

PostgresConnection possède un runtime current_thread ; une session épinglée tient son
mutex pendant await. Deux appelants synchrones peuvent se disputer la progression I/O
et ce mutex. Le fichier est inchangé par rapport à master : cela ne prouve pas encore
que l'ancien contrat concurrent se bloquait sans 1a. Un témoin direct begin puis deux
appels concurrents, sans entités/version/projection, sera exécuté avec délai externe et
source exacte de baseline. Aucun changement runtime tant que cette cause n'est pas prouvée.

Lien au ticket [versions et occurrences 1a](2026-11-11-deux-occurrences-identiques-reembarquent-le-meme-contenu.md).

Le témoin direct est préparé sur la baseline **68cc338c6**, branche archive
origin/cx-arbre-pg-pin-baseline, parent masterc04. Aucun changement runtime, seulement
le témoin de file Product déjà utilisé et le nouveau test PostgreSQL. begin puis deux
threads, barrière par tour, 16 requêtes SELECT/pg_sleep chacun et rollback ; ni alias,
ni version, ni projection. Le blob postgres_connection.rs est strictement le même
**1f4fcfb82ca4f7562453a12c0a99a9394520a667** sur c04, master ED c693 et notre code actuel.
Compilation séparée puis exécution sous poste lourd et timeout externe de 40 s ; un
bâti interrompu ne sera pas pris pour un rouge de concurrence. Journal attendu
`cx-pg-pin-baseline.log`, base SQL privée identique, aucune donnée effacée par ce témoin.

Premier montage du témoin direct : compilation native+postgres réussie, puis lanceur
refuse la suite qui exige aussi code, avant tout test ; ce n'est pas un rouge de concurrence.
Relance sur les huit features usuelles, même source68cc et délai de 40 s après compilation,
journal `cx-pg-pin-baseline-v2.log`. Aucune recherche ni modèle réel dans ce témoin direct.

Le lancement v2 n'est pas un rouge de concurrence : le premier no-run avait compilé,
mais le lanceur a repris le target de l'autre worktree et invalidé ses empreintes ;
le délai externe de 40 s a donc expiré dans ce second bâti, sans PG_PIN_BEGIN ni test.
Relance v3 avec propriétaire du target déjà établi et même source, logs séparés. Aucun
changement runtime ni inférence de cause à partir de ces échecs de montage.

## Comparateur direct vert, aucune cause déduite

Baseline **68cc338c6**, `cx-pg-pin-baseline-v3.log` : test réellement exécuté,
**1/0/0**, 0,19 s, PG_PIN_BEGIN puis 32 SELECT concurrents et PG_PIN_ROLLBACK atteints.
La garde de suite a aussi 2 verts. Cette forme simple ne reproduit donc pas le blocage ;
aucune préexistence générale ni faute de runtime ne sont établies. PostgresConnection
reste inchangé. L'hypothèse doit être départagée sur le graphe réel de la deuxième ingestion.

Diagnostic suivant borné : compilation du cas réel sur le socle combiné, puis lancement
du seul processus de test comme enfant de gdb (relation parent/enfant normale), interruption
automatique et piles sans arguments ni variables. Pas d'attachement à un tiers ni modification
de Yama, pas de changement runtime expérimental pour faire passer un témoin déjà vert.

## Pile du graphe réel : attente du pilote et de la session

Source exacte **ea6c1043c**, `cx-pg-graphe-gdb.log`, enfant seul PID1563432 lancé sous
gdb puis interrompu après 25 s et tué par son parent. Piles sans arguments ni variables,
Yama inchangé. Le témoin direct baseline vert reste distinct de cette reproduction réelle.

- Fil2 : PostgresConnection::session, ligne139, attend le std::Mutex de la session ;
  la pile contient CurrentThread::CoreGuard::block_on : ce fil tient le pilote.
- Fil5 scheduler-0 : PostgresBlobStore::save → BlobWriter::terminate_ref → fold_new_texts ;
  son execute_with_params est garé dans CurrentThread::block_on, voie sans core disponible.
- execute_with_params_async garde SessionTenue pendant l'attente réseau (lignes441–442).
  Les deux piles et ce contrat montrent le cycle : attendre synchroniquement la session
  immobilise le pilote dont dépend la requête du callback de stockage.

Le correctif candidat conserve le runtime et la sérialisation de la session mais fait
attendre son mutex **asynchroniquement** (tokio::sync::Mutex/MutexGuard). Cela permet au
pilote de progresser pendant la contention ; begin et clore emploient la même attente.
Coût : garde et réveil asynchrones sous contention. Aucun nouveau paramètre d'exécution,
API publique, thread ou contrat transactionnel. La préexistence générale sans ce graphe
n'est toujours pas déduite du comparateur direct vert. Une capture plus profonde du même
montage est en cours pour nommer aussi le parent public du dernier appel du fil2.

Capture profonde même sourceea6 : `cx-pg-graphe-gdb-v2.log`, parent arrêté son enfant
PID1617454 après 25 s. La pile confirme le point public : split_unchanged →
ingest_entities_jusqu_a → ecrire_un_lot_atomique → ingest_entities, test ligne2124.
Le callback bloqué est aussi vu en suppression de blobs pendant une compaction. Le dernier
événement public confirmé est l'ingestion atomique de la cellule suivante, pas une recherche.

Correctif isolable **60961bb24b1bd893f53e775bb6551c74f205f507**, src/postgres_connection.rs
seul, 8 insertions/6 retraits : type mutex/garde asynchrone et trois acquisitions await.
Pool, runtime current_thread, session épinglée et fermeture transactionnelle inchangés.
Même cas réel relancé sous gdb sur source60961, délai/piles identiques ; aucun vert encore
revendiqué. Les contrôles begin/commit/rollback/erreur/concurrence et la suite PG entière
restent requis avant clôture du ticket et demande de tour.

Relance même cas sous gdb, source **60961bb24**, `cx-pg-graphe-gdb-vert.log` : les deux
ingestions et leurs comptes passent, plus de blocage ; le processus termine en 0,37 s.
Le test complet reste **0/1** : première recherche refusée par SQL42601 (`_uuid IN $1`).
C'est un autre défaut, [appartenance scalaire du filtre](2026-11-12-les-listes-scalaires-de-filtre-produisent-du-cypher-sur-postgres.md).
Ne pas transformer ce progrès de l'ingestion en vert global du cas complet ni attribuer
l'erreur de filtre au mutex. Zéro session de la base privée après fin du processus.
