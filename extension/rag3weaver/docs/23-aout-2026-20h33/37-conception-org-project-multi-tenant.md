# Doc 37 — Conception : `org` × `project`, multi-tenant natif

Décidé avec Lucie le 24 août 2026 au soir. Chantier 2 du doc 29.

## 1. Le modèle : deux axes orthogonaux, pas une hiérarchie

- **`org`** = *qui* : propriété, frontière de confiance, facturation.
- **`project`** = *quoi* : une partition de données et d'usage (un corpus, un
  agent, une fonctionnalité).
- Chaque ligne porte **les deux**. Le moteur n'impose **pas** « un projet
  appartient à une org » : le client d'un logiciel multi-tenant décide de sa
  grille, et veut pouvoir comparer « pour le projet X, quelle org coûte le
  plus » — impossible si l'org contenait ses projets.
- Une hiérarchie, si une application en veut une, est une **convention de
  nommage** : `_org = "acme/eu/team3"` + filtre `starts_with` (présent chez
  nous et dans lucivy). Pas de règle moteur, pas de migration.
- **Matriochka** : notre propre plan de contrôle est une base rag3weaver où
  nos *clients* sont des **entités** (`Customer`, `Deployment`, `UsageEvent`),
  et où `(:Customer)-[:OWNS]->(:Database)` est le « troisième concept »
  (quelle base à qui) — une donnée, pas une feature du moteur. Composition de
  bases, jamais imbrication de labels.
- Mono-tenant embarqué : une org et un projet **par défaut** (`"default"`),
  zéro cérémonie.

## 2. Ce que le moteur garantit

1. **Colonnes système** `_org` et `_project` (STRING) sur **toutes** les
   tables porteuses de données : entités, `{KB}_Index`, `{KB}_Index_Chunk`,
   `{Entity}_Chunk`. Sur les chunks aussi, parce que le filtre vectoriel
   s'exécute sur la table des chunks (`rag3db_search_backend.rs:95-106`).
2. **Un index FTS et un index sparse par cellule `(kb|entité, org, project)`**,
   jamais partagés. Raisons : l'IDF de BM25 est calculé sur tout l'index (un
   index partagé fait fuir la pertinence *et* de l'information entre tenants) ;
   l'isolation devient structurelle (pas de `WHERE` à oublier) ; le cycle de
   vie suit (supprimer/exporter un projet = ses blobs) ; c'est le grain que
   lucistore sait sharder et distribuer.
   Clé : `{table}/{org}/{project}` → nom d'index `Lucivy_{table}__{org}__{project}`,
   blobs rangés sous ce préfixe dans `_index_blobs`.
3. **`_org`/`_project` aussi déclarés comme champs `string` de l'index FTS**
   (`filter_fields`, plomberie existante jamais branchée : `catalog.rs:1272`,
   `:2198`, `:2801` passent `&[]`). Coût nul ; ceinture et bretelles ; permet
   un mode partagé si on le voulait un jour — on ne l'active pas.
4. **`Scope { org, project }` porté par le `Catalog`** (`set_scope`, posé une
   fois) : stampe l'ingestion, sélectionne les index, filtre la recherche par
   défaut. Surchargeable par appel via `SearchOptions.scope` (recherche dans
   une autre cellule) et `SearchOptions.scopes` (fan-out sur plusieurs cellules
   + fusion RRF — le cas « tous les projets de mon org »). En embarqué un
   processus vit presque toujours dans un seul scope ; un paramètre obligatoire
   partout finirait dans un `..Default::default()`.
5. **Nœuds `_Org {id, name}` et `_Project {id, name}`** dans le graphe, sans
   arête de contenance imposée. Créés à la volée au premier stamp (MERGE).
6. **Le signal sparse est filtré** — aujourd'hui il ne l'est **pas du tout**
   (`search_sparse_via_backend`, `search.rs:2186`, sans paramètre de filtre ;
   `SparseHandle::search_filtered(allowed_ids)` existe, jamais branché). Avec
   un index par cellule le trou se ferme structurellement ; on branche quand
   même `allowed_ids` pour les `FilterCondition` utilisateur.
7. **Le résolveur vérifie** (`WHERE _org = $org AND _project = $project` à la
   résolution) — assertion de défense, pas contrat : le contrat est le
   pré-filtre (index par cellule + `WHERE` vectoriel).

## 3. Ce qui ne change pas

- La hiérarchie KB / entités simples, les signaux, la fusion.
- L'API : `register_entity`, `register_kb`, `ingest_entities`, `create`,
  `update`, `delete`, `search` gardent leurs signatures ; le scope est un état
  du `Catalog` + une option de recherche. Le FFI gagne `set_scope` et le
  parsing de `scope` dans les options (au passage : `parse_search_options`
  ignore aujourd'hui `filters`/`filterCondition` — à remplacer par serde).
- Le vecteur HNSW reste par table de KB, filtré par colonnes (sur-fetch +
  `WHERE`) : limite honnête de cette étape ; une table par cellule
  multiplierait les tables kuzu, à trancher avec des chiffres.

## 4. Plan d'implémentation

| Étape | Fichiers | Test qui le prouve |
|---|---|---|
| A. `Scope` + colonnes système sur les 4 DDL (helper partagé `scope_columns()` — il n'y a pas de liste `SYSTEM_COLUMNS` commune aujourd'hui) + tables `_Org`/`_Project` + migration ALTER ADD (défaut `"default"`) sous clé méta `schema_version` | `config.rs`, `schema.rs`, `dialect.rs`, `catalog.rs` (`migrate_entity`, `create_kb_tables`) | `e2e_idempotent_registration` : réouverture d'une base d'avant → colonnes présentes |
| B. Stamp à l'ingestion : `ingest_entities` (`catalog.rs:1680`), `create` (`:1855`), chunks (`record_nodes.rs:1095`, `:1309`, `:2011`), `KBUpdateNode` (liste codée en dur `:2588`) | `catalog.rs`, `record_nodes.rs` | `e2e_simple_entity` : deux scopes ingérés, colonnes lues |
| C. Index par cellule : `ensure_fts_handle` / `open_fts_handles_for` / sparse handles indexés par `(table, scope)` ; `filter_fields` branchés | `catalog.rs`, `fts_handle.rs` | même base, deux scopes → deux `Lucivy_*` dans `_index_blobs` |
| D. Recherche : scope par défaut, `SearchOptions.scope`, `scopes` (fan-out + RRF), `WHERE` vectoriel, sparse `allowed_ids`, assertion à la résolution | `search.rs`, `catalog.rs`, `rag3db_search_backend.rs` | **`e2e_scope.rs`** : BM25 / vector / sparse / hybride ne voient jamais l'autre cellule ; fan-out voit les deux ; `FilterCondition` E2E (aujourd'hui **zéro** test E2E sur les filtres) |
| E. FFI : `rag3weaver_catalog_set_scope`, `scope` dans les options (serde) | `wasm_ffi.rs` | test lib |
| F. Docs 30/31, README | | |

Ordre : A → B → C → D (chaque étape verte avant la suivante), E et F à la fin.
Le test sparse de D **doit échouer avant C** — c'est la preuve du trou.

## 5. Hors périmètre (nommé pour ne pas y glisser)

Le plan de contrôle cloud (bases ↔ clients), la fédération des `UsageEvent`
(lucistore `sync_server`), une table HNSW par cellule, des quotas par org.
