# 17 — Investigation UNWIND + MATCH + SET : bug moteur rag3db (pas HNSW)

## Résumé

L'investigation du "bug HNSW avec UNWIND SET" (doc 16, 2 tests Phase 2 échouent) a révélé que **le bug n'est PAS dans le code HNSW**. C'est un bug du moteur rag3db/Kuzu core : `UNWIND $items AS item MATCH (n:T {pk: item.pk}) SET n.col = item.val` ne traite que **2 items sur 3**. Le 3e item du UNWIND est silencieusement ignoré par le MATCH — aucun `NodeTable::update()` n'est déclenché pour lui.

## Preuve : test d'isolation sans HNSW

Test minimal ajouté dans `e2e_search.rs` (`debug_unwind_match_set`) :

```rust
// Création de 3 nœuds (auto-commit séparés)
boxed.execute("CREATE NODE TABLE T(id STRING, val FLOAT[4], PRIMARY KEY(id))").await;
boxed.execute("CREATE (:T {id: 'aaa'})").await;
boxed.execute("CREATE (:T {id: 'bbb'})").await;
boxed.execute("CREATE (:T {id: 'ccc'})").await;

// Vérification : 3 nœuds existent bien
// → OK, 3 nœuds trouvés

// PAS de HNSW index créé

// UNWIND SET avec 3 items
"UNWIND $items AS item MATCH (t:T {id: item.id}) SET t.val = item.emb"
// items = [{id:'aaa', emb:[1,0,0,0]}, {id:'bbb', emb:[0,1,0,0]}, {id:'ccc', emb:[0,0,1,0]}]

// Résultat :
// aaa → dim=4 ✅
// bbb → dim=4 ✅
// ccc → dim=Null ❌  ← MATCH n'a pas trouvé ce nœud !
```

**Reproductible à 100%** sur 2 exécutions consécutives. Toujours le 3e item (`ccc`, offset=2) qui est ignoré. AUCUN index HNSW impliqué.

## Diagnostic détaillé

### Ce qui se passe (traces fprintf)

Debug ajouté dans `NodeTable::update()` et `OnDiskHNSWIndex::update()` :

```
[NodeTable::update] tableID=0 offset=0 colID=1 propIsNull=0 numIndexes=1
[NodeTable::update] index[0] skip (not built on col)
[NodeTable::update] offset=0 COMMITTED path
[NodeTable::update] tableID=0 offset=1 colID=1 propIsNull=0 numIndexes=1
[NodeTable::update] index[0] skip (not built on col)
[NodeTable::update] offset=1 COMMITTED path
```

**Seulement 2 appels à `NodeTable::update()`**, pas 3. Le pipeline UNWIND → MATCH → SET ne produit que 2 tuples. Le MATCH pour le 3e item ne trouve pas le nœud.

### Observation non-déterministe dans le test Phase 2

Dans le test complet Phase 2 (avec Catalog + drain), le chunk manquant CHANGE entre les exécutions :
- Run 1 : offset=0 et offset=2 mis à jour, offset=1 (French Cuisine) manquant
- Run 2 : offset=0 et offset=1 mis à jour, offset=2 (Machine Learning) manquant

Dans le test d'isolation simplifié, c'est toujours offset=2 (le 3e) qui manque — probablement parce que les UUIDs/PKs sont déterministes dans ce cas.

### Vérification que les nœuds existent

Avant le UNWIND, `MATCH (t:T) RETURN t.id ORDER BY t.id` retourne bien 3 nœuds. Ils sont committés et visibles. Mais pendant le UNWIND, le MATCH ne les trouve pas tous.

Une vérification dans l'EmbedProcessor (`MATCH (n:Document_Chunk) RETURN n._uuid`) exécutée PENDANT le drain retourne **0 rows** — les chunks ne sont pas visibles dans un READ tx à ce moment — mais le UNWIND (WRITE tx) en trouve quand même 2 sur 3. Cela suggère un problème de snapshot/visibilité transactionnelle.

## Hypothèses sur la cause racine

### Hypothèse 1 : Bug dans le pipeline UNWIND → MATCH (la plus probable)

Le UNWIND produit N items. Le MATCH utilise la primary key pour résoudre chaque item. Le pipeline d'exécution (getNextTuple) pourrait avoir un bug où :
- Le 3e appel à getNextTuple du MATCH retourne "no more results" prématurément
- Le MATCH utilise un index scan qui ne se réinitialise pas entre les itérations UNWIND
- Le primary key index lookup a un edge case avec 3 items

### Hypothèse 2 : Bug snapshot transactionnel

Le UNWIND ouvre une transaction WRITE. Les nœuds ont été créés dans des transactions précédentes (auto-commit). Le snapshot de la transaction WRITE devrait voir tous les commits antérieurs. Mais peut-être que le snapshot est créé trop tôt ou ne voit pas les derniers commits.

### Hypothèse 3 : Bug dans le primary key hash index

Le MATCH sur `{id: item.id}` utilise le hash index sur la primary key. Si le hash index a un bug avec certains patterns de clés ou après certaines séquences d'insertion, un lookup pourrait échouer.

## Fichiers modifiés (debug uniquement, à reverter)

| Fichier | Modifications |
|---------|--------------|
| `extension/vector/src/index/hnsw_index.cpp` | fprintf debug dans `update()` : offset, isNull, listEntry, insertInternal |
| `src/storage/table/node_table.cpp` | fprintf debug dans `update()` : tableID, offset, colID, propIsNull, index calls, COMMITTED/UNCOMMITTED path, propIsNull avant/après index update |
| `extension/rag3weaver/src/catalog.rs` | eprintln debug dans EmbedProcessor : UUIDs, dimensions, DB check avant UNWIND |
| `extension/rag3weaver/tests/e2e_search.rs` | Test `debug_unwind_match_set` — reproduction minimale sans HNSW |

## Code path analysé (pour référence)

```
UNWIND $items AS item
    │
    ▼
MATCH (t:T {id: item.id})     ← BUG ICI : ne trouve pas le 3e item
    │
    ▼
SET t.val = item.emb
    │
    ▼
SingleLabelNodeSetExecutor::set()
    ├── info.evaluator->evaluate()
    ├── NodeTableUpdateState::new(columnID, nodeIDVector, columnDataVector)
    ├── NodeTable::initUpdateState() → crée Index::UpdateState FRAIS par itération
    └── NodeTable::update()
            ├── index->update() (HNSW si indexé) ← INNOCENT, pas appelé
            └── nodeGroups->update() (écrit la valeur)     ← INNOCENT, pas appelé
```

## Ce qu'on sait avec certitude

1. **Le HNSW est innocent** — le bug se reproduit SANS aucun index HNSW
2. **Chaque itération UNWIND crée un état frais** — pas de problème de state reuse (confirmé par le code : `NodeTableUpdateState` est `make_unique` à chaque appel de `set()`)
3. **Les nœuds EXISTENT** — 3 nœuds créés et committés, vérifiés par MATCH avant le UNWIND
4. **Le MATCH échoue silencieusement** — le 3e item ne produit aucun tuple, le SET n'est pas exécuté, aucune erreur
5. **Le bug est dans le moteur rag3db core** — pas dans l'extension vector, pas dans rag3weaver

## Prochaines étapes

1. **Investiguer le pipeline UNWIND → MATCH** dans le moteur rag3db/Kuzu :
   - `src/processor/operator/unwind.cpp` — comment les tuples sont émis
   - `src/processor/operator/scan/index_scan.cpp` ou équivalent — comment le primary key lookup fonctionne dans un pipeline UNWIND
   - Vérifier si le primary key index scan réinitialise correctement son état entre les itérations

2. **Test avec UNWIND seul** (sans MATCH) : vérifier que `UNWIND $items AS item RETURN item` produit bien 3 items

3. **Test avec MATCH sans SET** : vérifier que `UNWIND $items AS item MATCH (t:T {id: item.id}) RETURN t.id` retourne bien 3 rows

4. **Test avec 4 ou 5 items** : vérifier si c'est toujours le dernier item qui est perdu, ou si c'est spécifique à 3 items

5. **Reverter les fprintf** dans node_table.cpp et hnsw_index.cpp une fois le bug trouvé

## Leçons retenues

1. **Toujours tester l'hypothèse la plus simple en premier** — on a passé beaucoup de temps à analyser le code HNSW (state reuse, embeddingScanState, pool management) alors que le bug était dans le moteur core
2. **Un test d'isolation minimal est la méthode de diagnostic la plus efficace** — le test `debug_unwind_match_set` (30 lignes) a prouvé en 0.07s que le HNSW est innocent
3. **Les fprintf dans le C++ sont essentiels** — la trace `[NodeTable::update]` avec seulement 2 appels au lieu de 3 a immédiatement pointé vers le MATCH, pas le SET
