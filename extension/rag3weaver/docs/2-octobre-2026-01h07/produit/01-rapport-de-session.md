# Rapport de session — produit (arbre principal), nuit du 1er au 2 octobre 2026

Session qui bâtit et livre dans l'arbre principal, sous le contrôle de
l'orchestration (« rag3weaver archi »), étape par étape.

## 1. Ce qui est livré sur `master`

| Quoi | Commits | Preuve |
|---|---|---|
| Correctifs C++ extraits de l'expérience MTG (quantificateurs, `ParsedParameterExpression::copy`, `StringChunkData::finalize`), test de `copy` prouvé rouge | `9edf6f3b4` | lambdas / prédicats 7, string_finalize 2, api paramètres 6 |
| Fusion de `mtg-experiments`, et trois corrections (forme à plat de `get` contractualisée, `journal` annoncé par `describe`, test de persistance à jour) | `32d6e0b44`, `d83a557d3`, `f15fab787`, `d4f32f9e0` | lib, e2e, Python verts ; 16 tests débloqués par les poids minilm |
| **D** — le WAL : les enregistrements de plus de 4 Kio gardent leur début (vraie cause des WAL illisibles) | `955b1b136` | `WalTest.LongRecordsReplayWithTheirValues` rouge avant, transaction_test 50 |
| **E** — fin de journal déchirée : rouvre au dernier COMMIT, toute troncature copiée à côté, silence en lecture seule ; la limite (longueur abîmée lue comme fin déchirée) écrite | `cb152c833` … `98d25c9ee` | `WalTornEndTest` (5 cas), transaction_test 55 ; livré avec le rouge connu nommé |
| `sparse-vector` 4.3.0 (sources identiques à 4.0.1, montée nominale) | `7c653f66c` | suites sparse vertes |
| Trois suppressions : `fuse_results` (11 tests portés sur `fuse_signals`), grappe d'exploration, `search_with_strategy` (garde `max_rounds` dans `build_dataflow_graph`) | `01791e347` | 11 suites recherche / code / burn vertes, même nombre de cas avant / après |
| Écriture en lot sous `Lifecycle` : passe par la machine à états, tout ou rien | `213eeb214` | `test_backend_lifecycle_batch.py` rouge avant |
| Banc de l'étage réparé (deux aiguilles mortes depuis le 18 septembre) et référence en granite-278m avant / après | `3711da0ca`, `504b86168` | doc `docs/2-octobre-2026-01h01/01-…` |
| Correctif de reprise de l'index de clé primaire (session cœur C++), livré | `6bf46150b`, journal `8c1f93a5f` | transaction_test 56, api_test 102/102, lib 1101, 8 e2e, 3 Python |

## 2. Où le travail s'est arrêté

- **Arbre principal** : sur `master` à `8c1f93a5f`, à jour avec `origin`,
  `git status` propre hors des cinq fichiers non suivis connus
  (`build-lecteurs-csv.log`, `build-rag3weaver.log`, `follows.csv`,
  `user.csv`, `user.parquet`).
- **`build/lecteurs-csv`** : reconstruit le 2 octobre à 01 h 13 sur le C++ de
  `6bf46150b` (identique à `master` `8c1f93a5f` : seuls des docs et des patchs
  sont venus depuis). Extension vector reliée à 01 h 14. État sain.
- **`build/tests-wal`** : build C++ de tests au même C++ (transaction_test,
  api_test, suites de stockage).
- **La synchronisation par périmètre** : branche `synchronisation-par-perimetre`,
  poussée, un commit `325134ff0` « wip » — seulement la déclaration
  (`SnapshotConfig`, `OnMissing`, dans `config.rs`) et l'exemple de mesure
  `examples/mesure_marque_snapshot.rs`. Rien n'est validé ni utilisé encore.
  À la reprise : `git checkout synchronisation-par-perimetre && git rebase origin/master`,
  puis l'ordre prévu (voir §4).
- Aucun backend, démon ni ingestion n'a été arrêté. Le démon du port 7878
  (bge-m3) tourne, lancé par les tests.
- Branches laissées telles quelles : `fin-de-journal-dechiree`,
  `sparse-et-suppressions`, `lot-sous-lifecycle`, `banc-reference`,
  `livraison-reprise-index` (locales, fusionnées) ; sur origin
  `fin-de-journal-dechiree` porte des hash d'avant un rebase.

## 3. Ce qui attend une décision de Lucie

- **La corbeille** (son idée du « cache de garbage collector ») : évaluée,
  recommandée en seconde étape sous la forme « vraie suppression + copie à
  côté ». À trancher : la faire, et **le délai par défaut** (en jours, ou
  « jusqu'à la synchronisation suivante réussie »).
- **L'annulation en bloc d'une fin de synchronisation** (choix E) : aucune API
  du catalogue n'annule un drain aujourd'hui ; `DeleteRecordNode` ne garde que
  les lignes, pas les chunks ni les vecteurs. Il faut un mécanisme à écrire ;
  la corbeille le fournirait presque entièrement. Lucie dira si l'annulation
  attend la corbeille ou se fait seule.
- **Le rouge connu d'`api_test`** (lecteur d'un autre processus pendant un
  checkpoint) : son correctif est dans le plan des écritures parallèles.
- **La base MTG** : à reconstruire quand Lucie le dira ; ne pas l'ouvrir.

## 4. À faire en premier à la reprise

1. Lire le journal des chantiers et l'index du dossier du jour.
2. Synchronisation par périmètre, sur sa branche rebasée, dans cet ordre :
   l'entité synthétique et les tests rouges ; la validation de `snapshot`
   (champs du périmètre existants, état de `onMissing` déclaré, `keepFor`
   refusé) ; la colonne `_snapshot` (bloc `v8` de `migrate_scope_columns`,
   `SCHEMA_VERSION` 8, et `schema.rs` pour les tables neuves) ; le marquage
   par lot dans `EntityBatchNode` (paramètre `snapshot`, toutes les lignes du
   lot partagent la valeur du périmètre) ; l'appel de fin et ses quatre
   garde-fous ; `onMissing: state` par `update` ; le comptage des relations
   avant `DETACH DELETE` ; puis l'annulation selon la décision de Lucie.
   Arrêt avant la fusion.
3. Quand le moteur saura refuser après un checkpoint échoué : reconnaître
   l'erreur dans `Rag3dbConnection::execute` (`src/rag3db_connection.rs:245`)
   et décider (rouvrir ou arrêter proprement) — à la demande de
   l'orchestration.
