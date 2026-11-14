# Deux passes concurrentes partagent la base Postgres de test, et l'une rougit l'autre

- **État** : ouvert (14 novembre 2026, session recherche).
- **Gravité** : faux rouge. Une passe verte peut tomber sans rien de fautif dans son lot, et elle
  coûte alors un rejeu entier.
- **Atteignable en service** : non (tests seulement).
- **Touche rag3weaver** : oui (`tests/e2e_postgres.rs`, et le script de passe qui pose
  `RAG3WEAVER_PG`).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Ce que c'est

À luciepc, toutes les passes pointent `RAG3WEAVER_PG` sur la même base,
`postgres://rag3weaver:rag3weaver@127.0.0.1:5433/rag3weaver_test`. Le 14 novembre à 23 h 2x, le
rejeu court du lot des orphelins (ff4ced00a) a rougi 9 cas sur 36 d'e2e_postgres. Au même moment,
une passe d'everything declarative (worktree `rag3db-ed-generations-2`) jouait les mêmes suites.
Le même code était vert une heure plus tôt.

Les messages :
- `initialize sur postgres: query error: 42704: referenced schema was concurrently dropped`,
  sur `CREATE EXTENSION IF NOT EXISTS unaccent SCHEMA rag3weaver` (et `pg_trgm`) ;
- `42P01: relation "contenupg" does not exist`, sur
  `SELECT _uuid, _row_id FROM "contenupg" WHERE _uuid = ANY($1)`.

Une suite efface et recrée le schéma `rag3weaver` pendant que l'autre y écrit.

## Ce qu'il faudrait

Donner à chaque passe sa base, ou au moins son schéma : un nom dérivé du worktree ou du pid,
créé au départ et effacé à la fin. La garde de `run_e2e.sh` fait déjà la même chose pour le
`TMPDIR`. À défaut, un verrou consultatif Postgres (`pg_advisory_lock`) pris pour toute la suite.
Aujourd'hui, un rouge d'e2e_postgres pendant la passe d'une autre session se lit d'abord par
`ps` : qui joue `e2e_postgres` à ce moment-là.
