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
