# Des lectures qui n'attendent pas l'écrivain

*Arbre principal, 12 octobre 2026. Une page de conception, avant tout code : la demande de
l'orchestration, à relire par le cœur (la sémantique de l'instantané) et par le proto (la montre et la
réaction). Le témoin visé : **`usages` pendant une réindexation de 2 s répond en moins de 100 ms,
avec l'état d'avant.***

## Le constat

Sur la base entière (102 k morceaux), réindexer un fichier coûte environ 2 s. La réaction
`reindex_on_change` tient le catalogue pendant tout ce temps (`reindex_nodes.rs:250-292`). Le budget
de tenue de 400 ms (`c391e8387`) ne peut donc pas jouer : le plus petit lot est d'un fichier.

Une lecture d'outil attend :

- **dans `read_catalog_in`** (`catalog_read.rs:93-143`) : une attente bornée à 1 s, puis « occupé »
  ;
- **avant, sans limite** :
  - `Backend::build_services` prend `self.catalog.lock()` à chaque appel d'outil
    (`backend.rs:2992`, `:3001`), et sans se compter comme lecteur en attente : la réaction ne lui
    cède pas ;
  - les nœuds de `search_code` font de même (`generic_search_nodes.rs:735`, `:808`, `:924`, `:1103`,
    `:1574`, `:2389`, `:2470`) ;
- **et `run_index`** tient le catalogue pendant tout le plein texte d'une source, des minutes
  (`index_nodes.rs:414-439`).

Le verrou du catalogue protège trois choses à la fois, qu'il faut séparer :

1. **la connexion au moteur**, la seule du catalogue (`catalog.rs:461`), qui porte aussi les
   transactions de l'écrivain (`connection.rs:114-131`) ;
2. **la forme** : `entity_configs`, un `HashMap` nu, les handles du plein texte et du creux, et les
   DDL (ALTER, l'index vectoriel tombé puis refait) ;
3. **la cohérence des données** : un fichier réindexé passe par un état où ses scopes sont retirés et
   les neufs pas encore posés.

## Ce que le moteur donne déjà

Une transaction de lecture voit un instantané : `startTS = lastTimestamp`
(`transaction_manager.cpp:36-41`). La visibilité se juge par `(startTS, transactionID)`
(`version_info.cpp:147-207`), et le catalogue du moteur est versionné lui aussi. Une lecture sur une
**autre connexion** de la même `Database` ne voit donc pas une transaction d'écriture ouverte. Elle
voit l'état validé d'avant, sans attendre. Le moteur sert plusieurs connexions par base dans un
processus, et `create_sync_connection` le fait déjà (`rag3db_connection.rs:393`).

Ce qui bloque encore une lecture côté moteur :

- **un point de reprise demandé** : une nouvelle transaction attend sa fin, au plus 250 ms pour un
  point de reprise automatique (A8), et toute son écriture pour un point de reprise forcé (un COPY,
  la fermeture) ;
- **une validation forcée** attend au plus 5 s le départ des autres transactions, lecteurs compris,
  puis échoue. Un lecteur long pendant le COMMIT d'un paquet COPY est donc un risque pour
  l'écrivain ;
- **le DDL de l'index vectoriel** : une recherche pendant un DROP/CREATE rend une exception brute
  (ticket ouvert du 12 octobre) ;
- **HNSW** : la lecture concurrente d'une insertion est **à confirmer par le cœur**.

## La conception

### 1. Une connexion de lecture

Le `Catalog` ouvre, à côté de sa connexion d'écriture, une **connexion de lecture** (une ou un petit
pool) sur la même `Database`, avec peu de fils (`set_max_num_threads_for_exec`). Les lectures d'outil
(usages, impact, callees, liens, la recherche) passent par elle, en auto-commit. Chacune voit
l'instantané validé au moment de son instruction.

Ce lot règle aussi un risque relevé en passant. `VectorSearchNode` relâche le catalogue puis
interroge par `conn` (`generic_search_nodes.rs:735`). Une transaction de paquet ouverte entre-temps
par un autre fil fait tourner la lecture **dans la transaction de l'écrivain** : son échec défait le
paquet.

### 2. La forme, sous un verrou à part

Une **vue de lecture** (`ReadView`) est un `Arc` immuable : `entity_configs`, `dialect`, les handles
du plein texte et du creux, le moteur de recherche, la connexion de lecture.

- **La lecture** prend la vue courante par un `RwLock<Arc<ReadView>>`, en lecture. C'est une copie
  d'`Arc`, rien de plus, et elle ne touche jamais le mutex du catalogue.
- **L'écrivain** publie une vue neuve quand la forme change (`register_entity`, un ALTER, une colonne
  de modèle), et prend le verrou de forme **en écriture** autour des DDL qui ne souffrent pas une
  lecture : l'index vectoriel tombé puis refait (`ajuster_l_index_pour_le_retard`,
  `rebuild_vector_index`). Une lecture qui tombe pendant ce DDL attend là, et seulement là.

Les écritures de lignes, elles, ne prennent pas ce verrou.

### 3. Une réindexation, une transaction

`reingest_files` (la montre) n'ouvre aujourd'hui aucune transaction : chaque instruction se valide
seule. Une lecture sur la connexion de lecture verrait donc un fichier à moitié retiré. Chaque fichier
réindexé (ou chaque lot) passe dans **une transaction** sur la connexion d'écriture. La lecture voit
le fichier d'avant, entier, jusqu'au COMMIT, puis le neuf, entier.

**À voir avec le proto** :

- la tenue d'une transaction de 2 s face au point de reprise automatique, et à la validation qui
  attend les lecteurs au plus 5 s ;
- si l'embarquement (le service distant) doit sortir de la transaction : l'écrire en dette, puis
  embarquer après le COMMIT, comme le fait déjà le chemin de masse.

### 4. Ce qui reste sous le mutex du catalogue

Le mutex reste l'affaire des **écrivains entre eux** : la réaction, `run_index`, `edit_file_shared`
et le drain. Une lecture d'outil ne le prend plus. `read_catalog_in` garde son rôle de dire l'état
(occupé, partiel, jamais indexé), mais il le lit dans la méta, par la connexion de lecture, sans le
mutex. `build_services` prend ses services dans la vue.

### 5. Ce qui se lit hors du moteur

**Le plein texte (lucivy)** : la recherche concurrente d'une écriture est à confirmer. Un handle sert
d'ordinaire un lecteur sur le dernier commit, et l'écrivain valide par génération
(`sync_generation`). **Le creux** : même question.

## Les lots

| lot | quoi | témoin |
|---|---|---|
| L1 | la connexion de lecture, et les lectures d'outil qui passent par elle | une lecture pendant une transaction d'écriture ouverte voit l'état d'avant, sans attendre |
| L2 | la vue de lecture (`ReadView`), et `build_services` hors du mutex | usages répond pendant qu'un écrivain tient le mutex |
| L3 | une transaction par fichier réindexé | une lecture pendant la réindexation ne voit jamais un fichier à moitié retiré |
| L4 | le verrou de forme autour du DDL vectoriel | une recherche pendant le DROP/CREATE attend au lieu de rendre l'exception brute |
| — | **le témoin de bout en bout** | usages pendant une réindexation de 2 s : moins de 100 ms, l'état d'avant |

## Les questions

- **Au cœur** :
  - (a) une lecture d'une table HNSW pendant une insertion sur une autre connexion : sûre ?
  - (b) un lecteur en auto-commit pendant le COMMIT d'un paquet : la validation l'attend-elle (les
    5 s) ou passe-t-elle ?
  - (c) un nombre de fils par connexion, pour que la lecture n'affame pas l'écrivain.
- **Au proto** :
  - (d) une transaction par fichier dans la réaction, et l'embarquement après le COMMIT ;
  - (e) le budget de tenue, qui devient celui du mutex entre écrivains seulement.

## Les réponses (12 octobre 2026, matin)

**Le cœur** :

- **(a) HNSW sous un écrivain : sûr, et prouvé.** Les témoins du banc
  `vector_search_during_insert_test.cpp` et `readers_during_delete_test.cpp` jouent une recherche sur
  une connexion pendant une insertion ou une suppression sur une autre. Le lecteur lit à son
  instantané, puisque `isVisible` filtre ce que le graphe lui fait atteindre. Depuis I1, les
  écrivains d'une table indexée se sérialisent entre eux, pas contre les lecteurs. Il reste deux
  réserves : aucun ordre garanti entre deux recherches, et quelques ms de « fraîcheur ».
- **(b) Une validation seule n'attend aucun lecteur.** Seul le point de reprise forcé les attend,
  jusqu'à `checkpoint_wait_timeout` ; la forme d'A7/A8 borne cette attente et remet le point de
  reprise. **Règle de la page** : des lectures courtes en auto-commit, et `auto_checkpoint` laissé au
  moteur, sans CHECKPOINT par paquet.
- **(c) `setMaxNumThreadForExec`** borne une connexion. Le pool est partagé, donc une connexion de
  lecture à 2 ou 4 fils n'affame pas l'écrivain.

Le lot L4 (le verrou de forme) se réduit donc : le pas 1 DDL d'hotfixs tient l'index vivant sous
une recherche même si un DROP passe. L4 ne garde que la mutation de `entity_configs`, par la vue
(L2), et le ticket de l'exception brute, s'il reste rouge après ce pas.

**Le proto** :

- **(d) Une transaction par lot, pas par fichier.** Le lot est l'unité que `reingest_files` rend
  cohérente : toutes les fins, puis l'ingestion, puis la résolution. Avec une transaction par
  fichier, un renommage qui touche deux fichiers se lirait à moitié. Trois conditions :
  - **un échec n'empoisonne pas le catalogue** : un ROLLBACK qui laisse servir, à la différence du
    chemin de masse (`terminer`, qui empoisonne et exige de rouvrir) ;
  - **le plein texte suit le COMMIT** ;
  - **l'embarquement passe après le COMMIT, en dette**, et la ligne de retard le dit.
- **(e)** Le budget de 400 ms ne protégera plus que les écrivains entre eux. Son défaut montera
  (2 s, ou le seul plafond de fichiers) le jour où les lectures seront sur master, avec la remesure
  du proto.

**Le plein texte et le ROLLBACK** : l'`IndexWriter` de lucivy sait défaire
(`index_writer.rs:489`), mais `ShardedHandle` ne l'expose pas, et lucivy est publiée (3.0.8). La
forme retenue ne touche pas lucivy : **les documents du plein texte d'un lot sont gardés en tampon et
envoyés après le COMMIT**. Un ROLLBACK les jette, sans rien défaire dans lucivy.

## Les lots, revus

| lot | quoi |
|---|---|
| L1 | la connexion de lecture (2 à 4 fils), lectures courtes en auto-commit |
| L2 | la vue de lecture, `build_services` et `read_catalog_in` hors du mutex |
| L3 | une transaction par lot réindexé, un ROLLBACK sans empoisonnement, le plein texte en tampon jusqu'au COMMIT, l'embarquement en dette après |
| L4 | réduit : la mutation de la forme par la vue ; l'exception brute du DROP/CREATE si elle survit au pas 1 DDL |
