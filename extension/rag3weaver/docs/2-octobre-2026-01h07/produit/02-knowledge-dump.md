# Knowledge dump — session produit (arbre principal)

*2 octobre 2026, nuit. Session qui bâtit et livre dans l'arbre principal
`~/git_workspaces/rag3db`. Ce qui suit est orienté sur cette partie ; le
reste du projet est dans le knowledge dump du 1er octobre
(`../../1-octobre-2026-22h47/02-knowledge-dump.md`) et dans le journal des
chantiers (`docs/journal-des-chantiers.md`, à lire en premier).*

## 1. L'arbre principal et ses builds

- **`build/lecteurs-csv`** : la bibliothèque du moteur que lient le crate et
  toutes les suites e2e (`run_e2e.sh` : `RAG3DB_BUILD`, défaut
  `build/lecteurs-csv`). Cibles à reconstruire :
  `cmake --build build/lecteurs-csv --target rag3db_shared single_file_header -j<N>`.
  `single_file_header` produit `rag3db.hpp`, sans lequel le pont Rust ne
  compile pas.
- **L'extension vector** sort dans `extension/vector/build/libvector.rag3db_extension`.
  Après une reconstruction de `librag3db.so`, **elle ne se relie pas d'elle-même**
  (aucune dépendance ninja ne change) : déplacer le fichier puis
  `cmake --build build/lecteurs-csv --target rag3db_vector_extension`.
- **`build/tests-wal`** (dans l'arbre principal, ignoré par git) : build C++
  `BUILD_TESTS=TRUE`, sans extensions, pour `transaction_test`, `api_test` et
  les suites de stockage. Incrémental : `cmake --build build/tests-wal --target <cibles>`.
- **`RAG3DB_ROOT`** désigne l'arbre où le C++ est bâti, **jamais un worktree**.
  Pour jouer un banc ou une suite dans un worktree : `RAG3DB_BUILD` et
  `RAG3DB_ROOT` sur l'arbre principal, `CARGO_TARGET_DIR` sur
  `extension/rag3weaver/target` (le target partagé évite de rebâtir burn), et
  le sous-module `extension/rag3weaver/codeparsers` absent du worktree : un
  lien symbolique vers celui de l'arbre principal (à retirer **avant**
  `git worktree remove --force`, qui sinon ne suit pas le lien mais vaut mieux
  sans).
- Compilation Rust : `RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR=$PWD/build/lecteurs-csv/src
  RAG3DB_INCLUDE_DIR=$PWD/build/lecteurs-csv/src`, et `LD_LIBRARY_PATH` vers le
  même dossier pour exécuter. `-j6` pour toute reconstruction de burn (après un
  changement d'épinglage des forks, ~5 min) ; jamais `-j$(nproc)`. Le poste est
  partagé avec d'autres sessions qui compilent du C++ en `-j8`.
- `TMPDIR=/var/tmp` pour les tests.

## 2. La liste de livraison, et pourquoi chaque suite y est

Livrer = tout vert et rien de non joué, puis fusion en avance rapide dans
`master`, push sans force, journal des chantiers à jour.

| Suite | Ce qu'elle garde |
|---|---|
| `transaction_test` (C++) | journal d'écriture, checkpoint, transactions — dont `WalTest.LongRecords…` et `WalTornEndTest` |
| suites de stockage C++ (8) | le garde de `BufferedFileReader::read` touche tous ses appelants |
| `api_test` (C++) | lecteurs concurrents, paramètres ; **un rouge admis** : `LecteursConcurrents.CeQueLeLecteurVoitEstCoherent` (course connue, rouge aussi au 18 septembre) |
| `cargo test --lib` | 1 101 tests unitaires (depuis le retrait de l'exploration) |
| `e2e_prise_atomique`, `e2e_checkpoint`, `e2e_undo` | lecteurs, reprise, annulation |
| `e2e_search`, `e2e_chemin_de_masse`, `e2e_simple_entity`, `e2e_entites_derivees` | recherche hybride, chemin de masse, `Lifecycle`, dérivées |
| `e2e_code` | agent de code, graphe de stratégie |
| recherche : `e2e_generic_search`, `e2e_result_mode`, `e2e_search_queue`, `e2e_graph_tool`, `e2e_agent_loop`, `e2e_dataflow_observe` | à jouer quand la recherche ou le graphe de stratégie bouge |
| `e2e_burn_embedder` | prouve qu'un épinglage neuf des forks se bâtit et tourne |
| `test_backend_persistence.py`, `test_backend_harness.py`, `test_backend_lifecycle_batch.py` | backend déclaratif : persistance, harnais, lot sous `Lifecycle` |
| `test_chat_app.py`, `test_backend_mcp_render.py`, `test_structured_payloads.sh` | chat, rendu MCP, payloads imbriqués |

Un test « non joué » n'est jamais vert. Les poids minilm, multilingual-minilm,
bge-m3, granite-107m et granite-278m sont dans `~/.cache/rag3weaver/` ; sans
minilm, 16 tests e2e ne tournent pas. Le démon d'embarquement du port 7878
est lancé **par les tests eux-mêmes** (`tests/common/mod.rs:207`, binaire debug)
quand il manque ou est périmé : ne pas s'étonner de son pid qui change.

## 3. Le journal d'écriture (WAL)

- **Le bug des 4 096 octets** (corrigé, `955b1b136`) : `resizeBufferIfNeeded`
  (`checksum_writer.cpp`, `checksum_reader.cpp`) remplaçait le tampon sans
  recopier le début de l'enregistrement ; la somme de contrôle ne voyait rien.
  C'était la vraie cause des WAL illisibles de la base MTG, pas l'arrêt brutal.
- **La fin déchirée** (étape E, `cb152c833` et suivants, décision de Lucie) :
  `EndOfFileException` pour une lecture au-delà de la fin ; une fin de fichier
  au milieu d'un enregistrement, ou des enregistrements sans COMMIT, rouvrent
  au dernier COMMIT ; toute troncature est copiée dans
  `<journal>.ecarte-<ms>` (par blocs de 1 Mio) et dite sur stderr ; en
  lecture seule, rien n'est écrit ni dit.
- **La limite** : sans longueur par enregistrement, une longueur abîmée au
  milieu du journal se lit comme une fin déchirée et fait écarter des
  transactions validées (copiées à l'octet, rien supprimé). Seul un changement
  de format la lèverait.
- **La base MTG** (`experiments/mtga/data/engine-search-lucivy43-recovered.rag3db`)
  a un `.wal` écrit par l'ancien code : **ne pas l'ouvrir**. Lucie : « on la
  laisse tranquille pour le moment ».
- Course connue : un lecteur d'un autre processus rejoue le journal d'un
  écrivain vivant par trois lectures non atomiques (`wal_replayer.cpp`,
  `dryReplay` → `readCheckpoint` → rejeu) ; c'est le rouge admis d'`api_test`.
  Le plan des écritures parallèles le traite (session cœur C++).
- **À noter pour plus tard** : une erreur de checkpoint échoué (que le moteur
  apprendra à rendre bloquante jusqu'à réouverture) arriverait dans le crate
  par `Rag3dbConnection::execute` / `execute_with_params`
  (`src/rag3db_connection.rs:245-249`), puis remonterait comme un échec de
  drain (`FlushResult`) ou une `CatalogError::DbError` ; côté backend, comme
  une erreur de l'appel. C'est là qu'il faudra la reconnaître et décider :
  rouvrir la connexion ou arrêter le backend proprement.

## 4. Le backend déclaratif et ses tests

- `rag3weaver-backend <backend.json>` : JSONL sur stdio, ops `describe`,
  `call`, `journal`, `journal_read`, `shutdown`. `describe` annonce
  `capabilities: ["journal","journal_read"]` ; le chat n'envoie `journal`
  qu'à un backend qui l'annonce.
- `EntityBatchNode` (≤ 512 lignes, tout ou rien) : schéma JSON, identités,
  doublons, **et désormais la machine à états** (`213eeb214`) :
  `Catalog::lifecycle_refusals` relit l'état d'avant (`get_many`) et applique
  `lifecycle_verdict`, la même règle que l'ingestion. Une ligne refusée
  rejette le lot, rien n'est écrit. Refuse encore les entités à
  `WritePolicy` gérée (dates, révision, immuable).
- `Catalog::get` / `get_many` rendent une **ligne à plat** (champs, `_uuid`,
  `_label`), la même forme sur les deux dialectes ; contrat testé par
  `e2e_search::get_rend_une_ligne_a_plat`.
- `build_dataflow_graph` rend un `Result` et porte la garde `max_rounds` ;
  `search_with_strategy`, `fuse_results` et la grappe d'exploration n'existent
  plus. Les tests de fusion portent sur `fuse_signals`.
- Scripts Python : `extension/rag3weaver/scripts/test_backend_*.py`, avec le
  binaire `target/debug/rag3weaver-backend` (à rebâtir avant) et le Python de
  `experiments/mtga/.venv`.

## 5. Le chemin d'ingestion et `Lifecycle`

- `ingest_entities` → `split_unchanged` (relit l'état d'avant, court-circuite
  l'inchangé) → `apply_lifecycle` (refus nommés, ligne écartée, le lot
  continue) → écriture (MERGE ou chemin de masse par COPY sur table vide).
- L'avertissement « 0 relue » de `split_unchanged` ne se tait que si le lot
  part par le chemin de masse des naissances (désactivé par défaut,
  `RAG3WEAVER_COPY_NAISSANCES`).
- Le COPY sur table non vide coûte la taille de la table (non résolu).

## 6. La synchronisation par périmètre (en cours) et la corbeille

Décidé avec Lucie (six choix) : l'entité déclare `snapshot` —
`scope` (liste de champs, aucun nom en dur, `[]` = l'entité entière),
`maxMissingRatio` (0,5 par défaut), `onMissing` (`delete` ou
`{"state": "<état final>"}`), `keepFor` réservé (refusé tant que la corbeille
n'existe pas). Mécanisme : chaque lot d'une session marque ses lignes
(`_snapshot = <id>`, un `UNWIND … SET` par lot : mesuré 324 ms pour 27 000
lignes) ; un appel de fin retire, dans le périmètre seulement, les non
marquées, par le chemin de suppression existant. Garde-fous : rien avant la
fin ; instantané vide refusé ; au-delà de la proportion, `force` ; la fin
rend ce qu'elle retire. Tests d'abord sur une entité synthétique.

Ce que le code du crate sait déjà (relevé du 2 octobre) :
- les colonnes internes d'une entité sont en dur dans
  `schema.rs:180-196` ; une colonne neuve sur des bases existantes = un bloc
  `v8` dans `Catalog::migrate_scope_columns` et `SCHEMA_VERSION` à 8 (modèle :
  `_chunked_hash`, v5, `catalog.rs:3699-3729`) ;
- la validation d'une `EntityConfig` est `EntityConfig::validate`
  (`config.rs:938`), erreurs en `CatalogError::SchemaError` ;
- l'annulation : `DeleteRecordNode` garde les lignes (pas les chunks, ni les
  vecteurs, ni les relations) ; **aucune API du catalogue n'annule un drain**,
  seul `migration_rollback_graph` rejoue des `undo`. « Une fin = une unité
  annulable » demande donc un mécanisme à écrire ;
- `DETACH DELETE` emporte les relations sans les compter
  (`relations_deleted` vaut toujours 0) : il faudra les compter avant.

La corbeille (idée de Lucie) : évaluée, recommandée en seconde étape sous la
forme « vraie suppression + copie à côté » (table interne, ligne, hash,
textes et vecteurs des chunks), restaurée sans réembarquer si la ligne
reparaît identique ; à purger par `SET` plutôt que `DELETE` tant que rag3db
ne récupère pas les lignes supprimées.

Dette de généricité relevée (au journal) : `reingest_file` (code_tools.rs),
`WorkDomain` / `Selector` (sources, repos, languages), l'heuristique
`file_path` / `path` de la recherche et du rendu, la liste `CONSUMED` du
rendu, le curseur de source du code.

## 7. Le banc de recherche

- `tests/e2e_banc_etage.rs`, `RAG3WEAVER_BANC_MODELE=granite-278m` (sans la
  variable : HashEmbedder, chiffres sans valeur). ~3,5 min sur ce poste.
- **Corpus vivant** : le banc indexe `src/` ; toute modification de `src/`
  bouge ses chiffres. Ses **aiguilles** sont des signatures de fonctions qui
  doivent exister : une fonction retirée fait paniquer l'extraction (c'est
  arrivé du 18 septembre au 2 octobre). `e2e_banc_qualite` et
  `e2e_banc_texte_brut` partagent questions et aiguilles : les tenir alignés.
- Référence du 2 octobre (`docs/2-octobre-2026-01h01/01-…`) : après les
  suppressions, 43 questions, 5 247 scopes, tel quel 0,333, G 0,412. Le banc
  ne sort pas le rang par question : un écart de 0,005 reste non attribuable.
- Poids granite **régénérés** (`88495d5ba`) : sha256 `69aed115…`, pas le fichier
  du 18 septembre.

## 8. L'expérience MTG

Voir le knowledge dump du 1er octobre §3.8. Ce qui a changé : la base MTG est
à ne pas ouvrir (journal écrit par l'ancien code) ; elle sera reconstruite
quand Lucie le dira, avec le nouveau moteur. Le deck builder garde ses restes
au journal (§3).

## 9. Ce qui coûte du temps

- Un `git checkout` d'une branche déjà tirée dans un autre worktree échoue :
  créer une branche locale à soi depuis `origin/…`.
- zsh : `$VAR` contenant une liste ne se découpe pas en mots ; utiliser un
  tableau `T=(…)` et `"${T[@]}"`.
- Un test qui casse dans une suite qui en partage l'état (`LazyLock poisoned`)
  cache la vraie cause dans le premier échec de la sortie.
- Les commits ne portent aucun trailer d'IA ; `git config user.email` du
  dépôt est l'adresse personnelle de Lucie, à vérifier avant tout commit hors
  de rag3db.
