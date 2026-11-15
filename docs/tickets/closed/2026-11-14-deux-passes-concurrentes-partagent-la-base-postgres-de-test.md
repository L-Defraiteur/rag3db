# Deux passes concurrentes partagent la base Postgres de test, et l'une rougit l'autre

- **État** : corrigé (15 novembre 2026, 552299bec, G au mot de c7) : une base par passe, créée et effacée par chaque processus de suite.
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

## Une seconde occurrence, côté everything declarative (14 novembre 2026, 23 h 24)

C'était l'autre passe de la même collision. La passe courte de `vue-publiee-pile` (23101838b, lib
373b5477b, à luciepc) a vu 7 tests d'`e2e_postgres` sur 36 échouer, tous dans le schéma partagé, aucun
dans une réponse :
- six fois : `initialize sur postgres : 42704: referenced schema was concurrently dropped`, sur
  `CREATE EXTENSION IF NOT EXISTS pg_trgm|vector SCHEMA rag3weaver` (`l_ingestion_ecrit_des_lignes`,
  `l_hybride_fusionne`, `la_marque_de_confiance_nomme_son_signal`, `la_marque_deau_traverse_la_frontiere`,
  `la_reprise_apres_incident_tient_sur_postgres`, `le_chainage_une_chaine`) ;
- une fois : `42P01: relation "rag3weaver._dataflow_node_state" does not exist`
  (`deux_rattrapages_ne_reclament_pas_les_memes_chunks`).

La trace est à luciepc : `~/.cache/rag3weaver-build/ed-generations-2/pile/e2e_postgres-schema-partage.log`.
La même suite, rejouée seule sur le même commit, est le verdict de la passe (c7).

**La forme attendue** (c7) : un schéma par passe, pas un verrou. Il serait nommé par le worktree, ou
par un `RAG3WEAVER_PG_SCHEMA` posé par défaut et dérivé de la passe, et effacé à la fin.


## La correction (552299bec, 15 novembre 2026, 0 h 56)

Le schéma `rag3weaver` est fixé par le produit (le dialecte) : c'est la base qui se sépare. `tests/common/base_pg.rs` fait
créer à chaque processus de suite (e2e_postgres, e2e_contrat_transactions) sa base `rag3weaver_test_<poste>_<pid>` sur le
service de `RAG3WEAVER_PG`. Le processus l'efface à sa sortie (`atexit`, `DROP DATABASE … WITH (FORCE)`), et efface au
départ les bases de son poste dont le pid n'existe plus. `RAG3WEAVER_PG_BASE_FIXE=1` garde la base donnée.

**Rouge puis vert, à luciepc, deux e2e_postgres en parallèle sur 5433** :
- sur 270b6284d, la base d'avant le lot, à 1 h 00 : **ROUGE**, 2 passés et 34 échoués, puis 5 passés et 31 échoués ;
  90 « referenced schema was concurrently dropped » et 20 « does not exist » ;
- sur 7950c4789, le lot, à 0 h 53 : **39/39 chacun**, chacun dans sa base (`rag3weaver_test_luciepc_1377915`, `…_1377989`),
  aucune base restante.

Les deux occurrences vraies du 14 au soir : recherche 9/36, ED 7/36 à 23 h 24 (le doublon d'ED est fermé).
