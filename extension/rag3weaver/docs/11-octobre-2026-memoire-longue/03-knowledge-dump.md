# Knowledge dump — mémoire longue, 11 octobre 2026

*Pour la session qui reprend ce chantier : où est le code, ce qui le tient, les pièges.
Ce que j'ai fait est au rapport (`02-rapport-de-session.md`) ; ici, ce qu'il faut savoir.*

## Le lien générique

- **Un seul nœud de lien** : `RelationBatchNode` (`src/backend_nodes.rs`). La garde
  « entités gérées » est partie (`49142d58f` prouve qu'elle ne protégeait rien).
- **L'existence d'un bout** : `Catalog::verifier_les_bouts` rend
  `BoutsVerifies { absents, hors_etat }`. Un bout existe s'il est en base ou en file
  (`_uuid` lu dans la donnée), et une suppression en file l'emporte. Le test passe
  par `Select::by_uuids(entity, [Column::Node("_uuid")])` : le même chemin pour
  rag3db et PostgreSQL.
- **Le drain de la fermeture**, pas des seuls liens : `tenir_l_exigence_d_ecriture`
  (pub(crate)), exigence par défaut `| DONNEE`. Drainer ses seuls liens perdait un lien
  dont le bout est en file (relecture de 97).
- **Les politiques d'état** : `BackendRelation { def, from_states, to_states }`
  (`src/backend.rs`), validées au chargement. Le service `backend_relation_policies`
  les donne aux deux nœuds qui lient (`RelationBatchNode`, `AddRefNode`).
- **Le huitième refus** : `Choices::Relations { access: RelationAccess { read, write } }`
  sur les paramètres (`node_registry.rs`). `bind` fusionne les accès par union
  (`graph_tool.rs`). Une réaction qui remonte une relation qu'aucun outil n'écrit est
  refusée au chargement.
- **`Catalog::link`** refuse un bout absent seulement quand le lien se pose maintenant
  (`!exige.est_vide()`). Les liens différés de l'indexation du code (exigence AUCUNE,
  `mettre_en_file_les_liens`) ne passent pas par là ; c'est voulu, ils se comptent
  au drain.
- **La cellule des bouts** n'est pas faite : elle attend `entity_uuid_in(&cell, …)` de
  la migration de l'arbre principal (§3 bis de la page du lien).

## L'atomicité, et comment la prouver

- Une instruction : atomique par le moteur. Plusieurs groupes de liens seuls (groupés
  par l'ensemble des clés de propriété non nulles) : `begin` / `commit` / `rollback`
  avec `set_in_transaction`. Une fermeture qui porte des créations : `atomic: false`
  et `atomic_reason`.
- **Les fautes injectées**, jamais actives sans leurs deux variables :
  - liens : `RAG3WEAVER_TEST_LINK_FAULT_RELATION` + `..._GROUP` (boucle des groupes de
    `LinkRecordNode`) ;
  - insertions : `RAG3WEAVER_TEST_INSERT_FAULT_TABLE` + `..._GROUP` (boucle des
    groupes d'`InsertRecordNode`, qui fait échouer les résolveurs et consigne l'échec).
  Les témoins qui les posent se sérialisent par un `Mutex` statique (`FAUTES`) : les
  variables d'environnement sont globales au binaire e2e.
- **`rendu_pret` d'un `FlushResult`** dit ce que les opérations *réussies* du drain ont
  rendu prêt. Il ne vaut pas pour une fiche dont l'insertion a échoué : le put
  regarde d'abord si la ligne existe (`Ok(Some(_))`, filtrée par la cellule).

## La forme C, le client mince

- `src/daemon_client.rs` : `DaemonClient::connect(address, manifest, client, keys)`,
  un `impl Hote`. Une session perdue (`unknownSession`) se rouvre une fois et l'appel
  se rejoue ; une erreur `transient` ne se rejoue pas. Quatre tests unitaires contre
  un faux hôte `tiny_http`.
- L'hôte (`src/daemon_host.rs`) : `POST /session`, puis `/open`, `/describe`, `/call`
  avec l'en-tête `X-Rag3weaver-Session` ; `GET /etat`, `/sante` ; `POST /arret`.
- **Pas fait, et pas à anticiper** (étape 4 de C) : `daemon.json`, « assurer » le
  démon, l'outil « état du démon ».

## Le banc PostgreSQL

- Le conteneur de Lucie : `pgvector/pgvector:pg17`, port 5433, utilisateur, mot de passe
  et base `rag3weaver` / `rag3weaver` / `rag3weaver_test`. Pas de `psql` sur le poste :
  pour une requête directe, un venv du scratchpad avec `pg8000`.
- Les paramètres de `pg_trgm` n'existent dans une session qu'une fois la
  bibliothèque chargée : `SHOW pg_trgm.similarity_threshold` échoue avant le premier
  appel à `similarity()`.
- **Tout `SET` de session est faux sous le pool `deadpool`** : il ne vise qu'une
  session. Le plancher de rappel en souffre (ticket du banc, à F).

## Les pièges de la journée

- Le Bash de l'outil est zsh : `poste` n'est pas dans le `PATH`, l'appeler par
  `~/.cache/rag3weaver-build/poste lourd …`.
- `git worktree remove` refuse un worktree à sous-modules : vérifier `git status`
  vide et le dernier commit poussé, puis `--force`.
- Le test `daemon::embeddings::tests::le_creux_seul_traverse_le_fil` rougit sous la
  charge d'une passe (« le démon n'a pas répondu », ticket du 10 octobre) : le rejouer
  seul avant d'en conclure quoi que ce soit, et ne pousser qu'après.
- Une suite e2e nouvelle se monte par `python3 tests/e2e/generer.py` après son
  `git add`, et se joue par `cargo test --test e2e -- --ignored <suite>::`.
- Le README des tickets compte ses sections : un conflit de rebase s'y règle en
  recomptant les lignes (`| [`) de chaque section, pas à la main.
