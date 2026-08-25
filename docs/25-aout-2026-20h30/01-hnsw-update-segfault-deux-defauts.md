# 01 — L'UPDATE de l'index HNSW segfautait au-delà de ~512 lignes : deux défauts

25 août 2026, 20h30. Trouvé en ingérant le code de rag3weaver lui-même
(`extension/rag3weaver/docs/25-aout-2026-18h58/03`), corrigé le soir même.

## 1. Le symptôme, et son isolation

`SET v.emb = […]` sur une table indexée par `CREATE_VECTOR_INDEX` :
SIGSEGV dans `OnDiskHNSWIndex::shrinkForNode` → `computeDistance` →
`simsimd_cos_f32`, à partir de la 600ᵉ ligne environ. Le chemin d'insertion
(`CREATE (:V {emb: […]})`) tenait à 4 096. Le chemin UPDATE est celui
qu'emprunte toute l'ingestion de rag3weaver (INSERT puis SET), et le seul
possible pour une ré-ingestion — donc bloquant.

Sonde : `extension/rag3weaver/tests/e2e_hnsw_scale.rs`, Cypher brut, sans
rien du dessus.

## 2. Ce que le build Debug a montré

Un build `-DCMAKE_BUILD_TYPE=Debug -DENABLE_RUNTIME_CHECKS=ON`
(`build/native-debug/`) rend les `KU_ASSERT` et les assertions libstdc++.
Trois assertions, dans l'ordre où elles sont apparues :

1. **`rel_table.cpp:182`** — à l'insertion d'une arête HNSW,
   `srcNodeIDVector.state->getSelVector().getSelSize() == 1` faux.
2. **`local_rel_table.cpp:117`** — `std::vector<DirectedCSRIndex>::operator[]`
   hors bornes (`_GLIBCXX_ASSERTIONS`), **sur le chemin d'insertion aussi**.
3. Celle que je cherchais — `KU_ASSERT(!vector.isNull())` dans
   `shrinkForNode` — n'a jamais eu le temps de parler.

## 3. Défaut A — l'état de suppression partageait ses vecteurs avec l'état d'insertion

`HNSWInsertState` (constructeur amont, `444deac67`) construit
`relDeleteState` **sur les mêmes `ValueVector`** que `relInsertState`
(`srcNodeIDChunk[0]`, `insertChunk[0]`, `insertChunk[1]`). Or
`RelTable::detachDeleteForCSRRels` prend `dstNodeIDVector.state` comme état
de sortie de son scan et **réécrit sa sélection** (`setToFiltered(1)`,
`sel[0] = …`, `setRange`). Après un `detachDelete`, tout `relTable.insert`
qui suit lit des identifiants de nœuds à une position de sélection
corrompue : en Debug l'assertion 1 ; en Release, des arêtes vers n'importe
quoi, puis un graphe qui finit par pointer hors de tout.

Le chemin amont y est peu exposé (`shrinkForNode` n'y est appelé qu'à la
finalisation). Notre `update()` (fork, `98e35566a`) commence par
`deleteFromGraph` → `detachDelete` → **chaque** insertion suivante est
corrompue.

**Correctif** : `relDeleteState` a ses propres chunks (`deleteSrcNodeIDChunk`,
`deleteChunk`). `hnsw_index.h` / `.cpp`.

## 4. Défaut B — la table ne sait pas encore le vecteur qu'on insère

`NodeTable::update` appelle `index->update(…)` (ligne 486) **avant**
d'écrire la colonne (ligne 497). Pendant l'insertion HNSW, `shrinkForNode`
relit l'embedding du nœud **dans la table** (`getEmbedding(offset)`) : il
obtient l'ancienne valeur — NULL au premier `SET`, périmée à la ré-ingestion.
NULL → `getPtr()` = `nullptr` → simsimd déréférence : c'est le segfault de
la trace. Le chemin d'insertion ne le voit pas : une ligne insérée est lue
depuis le stockage local de la transaction (`isUnCommitted`), qui la connaît.

**Correctif** : `HNSWInsertState` porte `pendingOffset` / `pendingVector`
pendant `insertInternal` ; `shrinkForNode` lit le vecteur en attente au lieu
de la table pour ce nœud — comme sujet du shrink **et** comme voisin d'un
autre nœud (où il était silencieusement écarté, `isNull` → `continue` :
le nouveau nœud perdait ses arêtes retour). `NodeWithDistanceAndEmbedding`
porte un pointeur résolu, avec ou sans poignée.

## 5. Défaut C — hors bornes dans le cœur, bénin en Release

`LocalRelTable::delete_` prenait `auto& reverseDirectedIndex =
directedIndices[reverseIdx]` **avant** le test `reverseIdx < size()` trois
lignes plus bas. Sur une table de relations à une seule direction — celles
des couches HNSW — c'est hors bornes ; comportement indéfini en Release, abort
en Debug (assertion 2). Code amont. **Correctif** : références après les
tests. `src/storage/local_storage/local_rel_table.cpp`.

## 6. Mesuré, après correctifs

Release, `RAG3DB_PROBE_HNSW=1 ./run_e2e.sh --test e2e_hnsw_scale` :
**13 / 13** — chemin insertion 1 024 et 4 096 ; chemin UPDATE 1 024 et
**4 096** ; chemin catalogue (`ingest_entities`, INSERT puis SET) 64 → 4 096 en
dimensions 4 et 64 ; **double ré-ingestion** de 2 048 lignes (chaque
embedding remplacé une seconde fois, ancienne valeur non nulle). 208 s.

Debug : insertion, UPDATE et catalogue à 1 024, sans assertion.

`e2e_code` sur le module entier de rag3weaver : 25 fichiers, 1 402 scopes,
66 771 relations, 18 s d'ingestion, ré-ingestion idempotente.

Passe E2E complète après correctifs : **30 suites, 249 tests, 0 échec**
(les sondes à 1 024 sont des canaris permanents ; celles à 4 096 tournent
avec `RAG3DB_PROBE_HNSW=1`).

## 7. Ce qu'on garde de la méthode

- **Le build Debug est la première chose à faire**, pas la dernière : la
  trace Release pointait `computeDistance` ; la vraie cause (A) était trois
  appels plus haut et invisible sans l'assertion.
- Une assertion libstdc++ sur le chemin « qui marche » n'est pas du bruit :
  c'était un vrai hors-bornes (C) qui masquait la suite.
- `build/native-debug/` reste là (non versionné) ; `native-test` est
  reconstruit en Release, extension comprise — le fichier
  `extension/vector/build/libvector.rag3db_extension` est **partagé** entre
  les deux builds, le dernier construit gagne.
