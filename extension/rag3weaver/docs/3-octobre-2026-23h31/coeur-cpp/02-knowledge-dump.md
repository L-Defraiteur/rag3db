# Cœur C++ — ce que la session sait

Relevé de connaissances sur le moteur. Ce qui est affirmé porte un fichier, un commit ou
une mesure ; ce qui ne l'est pas est dit « non vérifié ». Les numéros de ligne datent du
3 octobre 2026 et bougent : chercher le nom de la fonction.
**Dernière mise à jour : 3 octobre 2026, vers minuit.**

Le relevé du 2 octobre (journal, point de reprise, lecteurs concurrents, mode
multi-écrivains) reste valable :
`../../2-octobre-2026-01h07/moteur-concurrence/02-knowledge-dump.md`.

## 1. Le journal et la reprise

- **Le rejeu a lieu dans le constructeur de `Database`** (`Database::initMembers` →
  `StorageManager::recover`, `src/main/database.cpp`), donc avant tout `LOAD EXTENSION` de
  l'appelant. À la fin du rejeu, la reprise écrit elle-même un point de reprise : après la
  première réouverture il n'y a plus de journal.
- **`LOAD EXTENSION` est journalisé et rejoué** (`ExtensionManager::loadExtension`,
  `src/extension/extension_manager.cpp` ; `WALReplayer::replayLoadExtensionRecord`). Un
  second `LOAD` d'une extension déjà chargée n'écrit rien. Un point de reprise vide le
  journal, cet enregistrement compris.
- **Qui écrit un point de reprise sans qu'on le demande** : `CREATE_VECTOR_INDEX`, `COPY`,
  le point de reprise automatique, la fermeture, la fin du rejeu.
- **La garde 1** (`fcd9a7882`) : `NodeTable::isWritableIndex`
  (`src/storage/table/node_table.cpp`). Au rejeu, un index non chargé est *détaché*
  (`IndexHolder::detach`, `src/include/storage/index/index.h`) : il n'est plus écrit par
  `NodeTable::serialize`, `getIndexHolder` et `getIndex` ne le voient plus, `addIndex`
  remplace un détaché du même nom. L'état « entrée au catalogue sans exemplaire dans la
  table » est l'état « à rebâtir », sans changement de format. Côté extension :
  `initHNSWEntries` (`extension/vector/src/main/vector_extension.cpp`) saute une telle
  entrée ; `throwIfBehindItsTable` (`extension/vector/src/index/hnsw_index_utils.cpp`) fait
  refuser la recherche et la création par `HNSWIndexUtils::INDEX_BEHIND_ITS_TABLE`
  (« is behind its table ») ; `DROP_VECTOR_INDEX` fonctionne.
- **Hors rejeu**, c'est le lieur qui refuse d'écrire dans une table dont un index n'est pas
  chargé (« … but its extension is not loaded », « … used in one or more indexes which is
  unloaded ») ; `NodeTable::INDEX_NOT_LOADED_FOR_WRITE` est un second rempart.
- **Le rejeu d'une mise à jour** appelle maintenant `initUpdateState` avant `update`
  (`WALReplayer::replayNodeUpdateRecord`) ; avant, aucun index n'en était informé.
- **Le rejeu d'un `DROP` d'index** retire l'entrée du catalogue *et* l'exemplaire de la
  table (`replayDropCatalogEntryRecord`, `f5acca417`). À l'exécution c'est la fonction de
  l'extension qui fait les deux (`drop_hnsw_index.cpp`).
- **Un doublon de clé primaire validé rend la base impossible à rouvrir** (« Found
  duplicated primary key value » au rejeu), index ou pas. Atteignable seulement sous le
  mode multi-écrivains. Réponse retenue par l'orchestration : A3′ l'empêche de naître ; la
  reprise devra mettre de côté la transaction fautive (fichier à côté, erreur nommée avec la
  table et la clé) au lieu de refuser d'ouvrir.
- **Mes tests de reprise ne tuent pas le processus** sauf
  `vector_index_crash_reopen_test.cpp` : les autres ferment la `Database` après
  `CALL force_checkpoint_on_close=false` (le journal reste et se rejoue), ou forcent un
  point de reprise en échec (`FlakyCheckpointer`). Le banc a désormais la famille « arrêt
  brutal, un seul écrivain » par vrai SIGKILL.
- **Non élucidé** : `e2e_idempotent_registration` rougit environ une fois sur dix,
  « Reading past the end of the file …wal with size 0 at offset 0 », avant comme après mes
  correctifs (2 sur 20 des deux côtés).

## 2. L'index vectoriel (HNSW) et notre greffe

- L'insertion dans l'index se fait **au commit** (`needCommitInsert`,
  `NodeTable::commit` → `scanIndexColumns` → `OnDiskHNSWIndex::commitInsert`). La
  suppression et la mise à jour sont **notre greffe** (`98e35566a`) et agissent pendant la
  transaction. Les points d'entrée et `numCheckpointedNodes` vivent hors transaction.
- La recherche lit les vecteurs **dans la table**, pas dans le graphe : un graphe aux
  positions périmées rend encore des distances justes, ce qui masque des défauts.
- **Lecteurs** : `NodeTable::lookup<false>` / `lookupMultiple<false>` (lecture des vecteurs)
  et le balayage des relations en mémoire passent sous `NodeGroup::inMemAppendMtx`
  (partagé), exclusif pendant `NodeGroup::append` (A5 bis). Les informations de version et
  l'index des relations en mémoire ont leurs propres verrous (A5).
- **Corrigé** : `finalizeDelete` qui ne s'exécutait jamais ; les arêtes pontées à la
  suppression ; `keepNodeReachable` (un survivant reste atteignable) ; le `ROLLBACK` d'un
  `DELETE`.
- **Défauts connus, non corrigés** (journal §6) :
  - le `SET` d'un vecteur vers un autre perd des lignes (969 sur 1000 ligne à ligne, 532 par
    lots de 512) ; depuis NULL, rien ;
  - une ligne injoignable dès la construction : à 98 et 100 lignes sur
    `embeddings-8-1k.csv`, et toute ligne très loin des autres (500 lignes, une à
    `[9000, 2, 3, 4]`) ; l'élagage différé des voisins est soupçonné, cause non élucidée ;
  - `QUERY_VECTOR_INDEX … RETURN count(*)` rend toujours `k` ;
  - appeler `createRels` depuis `finalizeDelete` plante dans `shrinkForNode` ;
  - sous plusieurs écrivains : conflits d'écriture entre voisins (H3), état corrompu (H4).
- L'étude : `docs/3-octobre-2026-15h47/02-hnsw-sous-plusieurs-ecrivains.md`.

## 3. Le planificateur

- La recherche par l'index de clé primaire n'existe que pour une clé **constante**
  (littéral, paramètre, `CAST` de l'un des deux : `isConstantExpression`,
  `src/optimizer/filter_push_down_optimizer.cpp`). L'opérateur physique évalue la clé une
  seule fois.
- Un `MATCH` après `UNWIND` : tout prédicat qui dépend de la ligne extérieure est mis de
  côté, le motif est joint par produit cartésien puis filtré (`planRegularMatch`,
  `src/planner/plan/plan_subquery.cpp`). `visitCrossProductReplace` rattrape en jointure de
  hachage **seulement** si chaque côté de l'égalité est une colonne nommée — une variable
  simple oui, `STRUCT_EXTRACT(item, …)` non.
- **La recette** (session de l'arbre principal, `dialect::unwind_par_cle`) : `item` voyage,
  chaque clé est extraite dans le `WITH` juste avant son `MATCH` ou son `MERGE`. Vérifier
  par `EXPLAIN` : `HASH_JOIN`, pas `CROSS_PRODUCT`.
- La jointure de hachage balaie encore la table pour se bâtir (0,3 / 0,8 / 5,8 ms par lot de
  300 à 2 000 / 20 000 / 200 000 nœuds) ; la vraie réponse est une recherche par clé par
  ligne, que Ladybug a faite.
- La base de test en mémoire a un tampon de 73 Mo : un « buffer pool is full » y est un
  plafond de test.

## 4. Les verrous à venir

Note : `docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md` (les trois écarts
tranchés en tête du §1). Ce qui est décidé en plus, par message :
- une ressource de verrou est un identifiant **plat** (genre, table, valeur), pour pouvoir
  vivre un jour dans un segment partagé (marche B) ; les détenteurs sont des transactions,
  pas des fils ; l'interface est étroite (prendre, tout relâcher, lister les attentes) ;
- deux genres dès V1 : ligne par clé, **index d'une table** (exclusif pour tout écrivain
  d'une table indexée, de sa première écriture au commit) ; H3 et H4 en sont les témoins ;
- l'annonce d'une clé d'une table indexée prend aussi l'index, sans que l'appelant le sache ;
- sous l'option A (instantané par transaction), un écrivain qui a attendu peut échouer sur
  « could not serialize » : le verrou d'index n'est vraiment utile qu'avec l'annonce en tête
  ou en auto-commit ;
- noms reconnus par le banc : « deadlock », « lock » + « timeout » (`CALL lock_timeout`),
  « could not serialize », « Interrupted », `CALL acquire_locks('Table', [clés])`,
  « duplicated primary key » ;
- la maintenance de l'index au commit vient juste après V2.
Les témoins sont au banc (`test/transaction/concurrence/lock_bench_test.cpp`), rouges.

## 5. Les amonts

- **Kuzu** : notre origine (renommage `c647fbb33`). Défauts d'origine trouvés : le rejeu
  après un point de reprise sans l'extension, le `DROP` d'index au rejeu, le rejeu d'une
  mise à jour qui ignore les index, la recherche par clé limitée aux constantes.
- **Vela** (tags `vela-master-2026-09-03`, `vela-checkpoint-2026-09-03`) : histoire commune
  jusqu'au 10 octobre 2025 ; rien sur le planificateur ni sur ces défauts.
- **Ladybug** (tag `ladybug-main-2026-08-31`) : histoire **séparée** de la nôtre, espace de
  noms `lbug`, même arborescence — on porte, on ne cherry-pick pas. Trouvé ce soir :
  `e92346c97` et sept correctifs (recherche par clé par ligne pour `MATCH`) ; `d2db8acb4`,
  `1ba0cc540` (gardes des index non chargés, reprises dans la garde 1). **Pas regardé** : ce
  qu'il a fait de la mise à jour et de la construction dans son HNSW, et de la reprise.

## 6. Essayé sans succès — à ne pas refaire tel quel

- **A5 bis, coût de la garde** : le verrou exclusif du groupe (double le temps de quatre
  recherches parallèles) ; une garde à bandes, une case par fil lecteur (aucun gain : le
  coût n'est pas la dispute d'une ligne de cache) ; la garde tenue par lot
  (`annexes/a5bis-variante-par-lot.patch`, aucun gain mesurable sous charge). Le poste n'a
  ni `perf` ni `valgrind`.
- **`SET` aligné sur `CREATE`** pour un champ nul : rejeté, des tests de l'amont fixent la
  rigueur de `SET` (`annexes/set-comme-create-et-ses-tests.patch`).
- **Mesurer un coût sous la charge des autres sessions** : une passe d'ingestion faite sous
  une charge de 20 a vu l'embarquement passer de 94 à 144 s.
- **Marquer la table « modifiée » dans `NodeTable::dropIndex`** : retiré, le test ne
  rougissait pas sans.
- **Isoler la croissance du `MERGE` avec la liste en littéral** : le temps est dominé par
  l'analyse de la requête, rien n'en sort.

## 7. Les mesures gardées

| Mesure | Résultat | Où |
|---|---|---|
| Coût d'A5 bis, recherche vectorielle seule, dos à dos | dim. 8 : +3 % à un fil, +17 % à quatre ; dim. 256 : +6 %, +21 % | `annexes/vector_search_cost_scratch_test.cpp` |
| Formes de `UNWIND … MATCH` par la clé | table au journal §6 | `annexes/pk_match_forms_scratch_test.cpp` |
| Lots de relations quand la table grossit | plat ; le coût suit le nombre de nœuds | `annexes/rel_insert_growth_scratch_test.cpp` |
| `SET` de vecteur et lignes joignables | table du rapport de session | `annexes/vector_set_reachability_scratch_test.cpp` |
| `e2e_idempotent_registration` en boucle | 18 vertes sur 20, avec ou sans `f5acca417` | `annexes/boucle-idem.sh` |
