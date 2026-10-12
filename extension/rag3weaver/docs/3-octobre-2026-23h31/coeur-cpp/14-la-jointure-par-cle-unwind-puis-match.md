# La jointure par clé : `UNWIND $liste AS u MATCH (d:T {clé: u})`

Lot du cœur C++ donné par l'orchestration le 12 octobre 2026 à 6 h 45, devant l'insertion HNSW :
la forme que compile l'IR pour chaque saut (`Hop`) et chaque `Select::by_uuids` ne produit aucun
masque, et `usages` reste à 352 ms sur l'index entier (F) alors que `WHERE d._uuid IN $uuids` fait
4,3 ms et un paramètre scalaire 1,1 ms, mêmes 117 lignes. Cible : `usages` sous 50 ms, parité par les
empreintes et les e2e. Témoins : `les_formes_du_saut_d_usages_sur_l_index_entier` (F, V1 et V4) et
`le_plan_du_saut…`.

## 1. Ce que fait le moteur **[lu]**

- Le binder fait de `{clé: u}` le prédicat `d.clé = u`. Le planificateur planifie l'UNWIND d'abord ;
  le MATCH passe par `planRegularMatch` (plan_subquery.cpp:173) : le prédicat lit `u`, du schéma de
  gauche, il est **remonté** (`predicatesToPullUp`) ; plus rien ne corrèle, `appendCrossProduct`
  (probe = UNWIND, build = le MATCH, un `SCAN_NODE_TABLE` entier de T), et le prédicat revient en
  `Filter` au-dessus.
- L'optimiseur (`visitCrossProductReplace`, filter_push_down_optimizer.cpp:83-141) fait du produit +
  égalité une `LogicalHashJoin((u, d.clé))` avec **`SemiMaskPosition::PROHIBIT`** (« non-id based
  joins ») : le passage d'information (`HashJoinSIPOptimizer`) s'arrête là (acc_hash_join_optimizer
  .cpp:314). Le `PRIMARY_KEY_SCAN` n'est posé que si le membre droit est une constante (littéral,
  paramètre, CAST d'une constante : `isConstantExpression`) ; `IN` n'est pas traité (un `Filter`
  après le balayage entier — F mesure pourtant 4,3 ms : c'est la jointure de hachage sur une liste
  petite côté build, pas un balayage par clé).
- Il n'existe ni jointure par index sur clé (l'INL de `tryPlanINLJoin` ne vaut que pour les
  relations), ni masque par clés (`SemiMaskKeyType` : NODE, PATH, NODE_ID_LIST ; le masqueur reçoit
  des `nodeID_t`). `NodeTable::lookupPK(tx, keyVector, pos, offset)` résout une clé en décalage en
  O(1), table locale d'abord puis l'index de hachage, visibilité comprise.

## 2. Les deux formes **[déduit]**

- **A. La recherche par clé par ligne** (UNWIND + NodeIndexSeek de Neo4j) : un opérateur non source
  à enfant, sur le modèle d'`IndexLookup` (réservé au COPY : il sort un INT64 et lève sur une clé
  absente), qui fait `lookupPK` par ligne, retire les absentes de la sélection, écrit `d._ID`, lit
  les propriétés ; à produire dans `planRegularMatch` (reconnaître `d.clé = expr(gauche)`, puis
  corréler sur `d._ID`) — la vraie jointure par index, mais un opérateur logique et physique de
  plus, le mapper, l'estimateur, la factorisation, EXPLAIN.
- **B. Le masque par clés, puis mon balayage par décalages** : ne plus poser `PROHIBIT` quand une
  clé de build est la clé primaire de `d` (une table) et que le build contient un `SCAN` de `d` ;
  dans `HashJoinSIPOptimizer`, une branche probe→build par clé (sans `isProbeSideQualified`) qui
  pose `SemiMasker(PRIMARY_KEY, clé = u castée au type de la clé)` + `Accumulate` sur le probe
  (`ON_PROBE / PROBE_DEPENDS_ON_BUILD / PROBE_TO_BUILD`, comme l. 257-261) ; un
  `PrimaryKeySemiMasker` physique qui fait `lookupPK` par valeur et masque le décalage ; la jointure
  de hachage reste (doublons de la liste, clés absentes : sémantique intacte) ; le `SCAN` de `d`
  passe en mode décalages (page 12 §10) et V1 (`d<-r-m`) en profite tel quel puisque le masque vise
  le `SCAN` de `d` dans le sous-arbre build.

**Choix : B d'abord** — petit, local (optimiseur et masqueur), il tient la cible avec ce qui
existe ; A reste la forme générale si B laisse un cas (une clé composée, un `d` qui n'est pas un
`SCAN`). À vérifier : la taille du masque (`getNumTotalRows(transaction)`) couvre les décalages
locaux que `LocalNodeTable::lookupPK` rend ; une clé NULL ou absente ne masque rien.

## 3. Codé, mesuré (12 octobre, 8 h à 10 h) **[mesuré]**

- `SemiMaskKeyType::PRIMARY_KEY` et un `PrimaryKeySemiMasker` physique (une valeur → un décalage par
  `NodeTable::lookupPK`, table locale comprise ; NULL ou absente : rien) ; la descente des filtres ne
  met plus `PROHIBIT` quand une clé de build est la clé primaire d'un nœud et que la clé du probe a
  son type ; `HashJoinSIPOptimizer::tryKeyMaskHJSIP` accumule le probe et pose le masqueur sur le
  `SCAN` de d dans le build. La jointure de hachage reste (doublons, absents).
- **Le second niveau, lot suivant (branche `jointure-par-cle-niveau-2`, hors du push)** :
  `(d {clé: u})<-[r]-(m)` masque d mais lit m entier (Scope 81 407 lignes, ~20 ms par saut : tout
  l'écart entre un temps et deux temps), parce que le passage d'information ne tient un côté pour
  sélectif que s'il porte un filtre ou une recherche par clé. Premier essai : un balayage masqué
  compte comme sélectif et le build est revisité après le masque par clés. **Prouvé sur la V1
  exacte de F** (reflink de son sceau, lib 513438183) : deux SEMI_MASKER, SCAN Symbol 1,
  SCAN_REL MENTIONS 117, **SCAN Scope 117 lignes** (81 407 avant), V1 à 2,4 ms à chaud. Mon premier
  témoin ne le voyait pas parce que son saut était planifié en Extend sans jointure (petit voisin
  lu par sa clé) : refait sur la forme de F (grand voisin lu par trois propriétés : jointure de
  hachage sur m._ID). Lot poussé à part, après le témoin du banc et la remesure de F.
- Témoin `KeyMaskJoin` : 72 000 lignes, 117 clés INT64 → le balayage sort moins de 1 170 lignes,
  les lignes rendues sont celles des clés dans l'ordre, une clé en double rend deux fois, une
  absente rien ; la forme du produit (liste de chaînes en paramètre, clé STRING, un saut) rend ses
  lignes, deux masqueurs dans le plan.

**Mesure de F** (lib du lot avant le second niveau, copie de l'index entier, empreintes
identiques, 11 tests sur 11) :

| | avant | après |
|---|---|---|
| `usages` en un temps | 352 / 352 / 349 ms | **117 / 120 / 117 ms** |
| `usages` en deux temps | 230 / 234 / 209 ms | **43 / 47 / 44 ms** |
| le saut UNWIND (V1) | 111,7 ms | 24,1 ms (IN : 5,2 ; scalaire : 1,4) |
| la lecture par clé UNWIND (V4) | 28,1 ms | **1,3 ms** |

PROFILE de V1 : SEMI_MASKER présent, SCAN Symbol 1 ligne (86 248 avant), SCAN_REL MENTIONS 117
(491 102 avant) ; Scope encore 81 407 lignes (15 ms) : le second niveau, codé ensuite, à
remesurer par F (deux SEMI_MASKER attendus, Scope à ~117).

## 4. Le second niveau, mesuré (12 octobre, 10 h à 12 h) **[mesuré]**

Lot `jointure-par-cle-niveau-2`, **sur master `97589fc9d`** (12 octobre, 11 h 50) : un balayage masqué
(`LogicalScanNodeTable::isSemiMasked`) compte comme côté sélectif du passage d'information, et le
build d'une jointure par clé est revisité après la pose du masque (`visitOperator(buildRoot)`), avec
la garde ON_BUILD/ON_PROBE de `visitHashJoin` pour ne pas poser deux masques sur le même balayage.

| | avant (lot 1) | après |
|---|---|---|
| `usages` en un temps (F) | 117 ms | **65 ms** |
| `usages` en deux temps (F) | 43 ms | **29 ms** |
| le saut UNWIND (V1 de F) | 21 ms | **2,3 ms** |
| le témoin du voisin (banc, `MaskedNeighbour`) | ×1 | **×20** |

PROFILE de la V1 : deux SEMI_MASKER, SCAN Symbol 1, SCAN_REL MENTIONS 117, SCAN Scope 117 lignes
(81 407 avant). Ce qui reste dans `usages` en un temps : les sauts Scope → Scope, 18 à 20 ms chacun,
qui ne sont pas des voisins d'un nœud masqué mais des nœuds liés par leur clé à la partie
précédente — le troisième pas.

## 5. Le troisième pas : le nœud lié par sa clé s'estime à la cardinalité de la partie précédente **[mesuré]**

**Le défaut.** `UNWIND $liste AS u MATCH (d:Scope {_uuid: u})-[:HAS_PARENT]->(m:Scope)` : le prédicat
`d._uuid = u` dépend de la partie précédente, il est remonté au-dessus de la jointure et le balayage
de d ne le voit pas. d et m s'estiment donc tous deux à la taille de Scope ; à égalité,
l'énumérateur d'ordre de jointure étend depuis m (jamais masqué) et joint d par hachage. Le masque
par clés, posé après la planification, ne peut plus changer l'ordre : Scope entier est balayé pour
m à chaque saut. Avec un paramètre scalaire (`{_uuid: $u}`), le prédicat est local, d s'estime à 1
et l'extension part de d : c'est la forme rapide, que la liste perdait.

**Le code** (branche `cle-estimation-du-noeud`, **sur master `0c0af5bf5`**, 12 octobre, 13 h 33). `planRegularMatch` repère, parmi les prédicats
remontés, `n.clé = expr(gauche)` sur la clé primaire d'un nœud du motif (`getDependentExprs`), et
le note dans `QueryGraphPlanningInfo::keyBoundNodeIDs` avec `keyBoundCard` = la cardinalité de la
partie précédente. `planQueryGraph` rectifie alors l'estimation de ces nœuds à cette cardinalité
(`cardinalityEstimator.rectifyCardinality`), comme pour le nœud corrélé d'une sous-requête — pour
les nœuds du graphe planifié seulement (`hasNodeIDDom` : un graphe par composante, le premier essai
tombait sur `unordered_map::at` pour un nœud clé d'une autre composante). L'extension part de d
masqué, m n'est lu que par ses voisins. C'est l'estimation d'une recherche par index chez Neo4j.

**Témoins.** `KeyMaskJoin.AHopWithinTheSameTableStartsFromTheKeyedNode` (60 000 S, P de S vers S,
trois clés) : aucun balayage de S ne sort plus de 30 lignes, 3 lignes rendues. Au banc,
`MaskedSelfHop.StartsFromTheKeyedNode` sur une vraie base (81 407 Scope, 37 572 HAS_PARENT) :
rouge sur master (m 81 407, HAS_PARENT 37 572, un masqueur), vert avec le lot (d 1, m 1,
HAS_PARENT 1, deux masqueurs).

| médiane de 5 (banc) | master | lot |
|---|---|---|
| le saut Scope → Scope, 1 fil | 8,62 ms | **0,40 ms** |
| le saut Scope → Scope, 32 fils | 8,87 ms | **0,39 ms** |
| le voisin masqué | 9,57 ms | **0,56 ms** |

**Mesure de F sur la lib du troisième pas** (13f4647dc, index entier, empreintes identiques, 0 vide,
11 tests sur 11) : `usages` en un temps **7,4 / 9,5 / 7,1 ms** (352 au départ, 117 après le lot 1,
65 après le second niveau), contre 10,0 / 12,6 / 9,6 en deux temps — le saut Scope → Scope lit
1 ligne de Scope de chaque côté et 1 de HAS_PARENT, deux SEMI_MASKER, chaque saut d'usages
(IMPLEMENTS, INHERITS_FROM, HAS_PARENT, CONSUMES) à 1,0-2,4 ms au lieu de 18-20. Le saut en deux
temps est retiré de rag3weaver (c56cb3aa6, page 06 de F). En tout : **usages 352 → 7-10 ms**, le
but 2 de Lucie (< 50 ms) dépassé d'un ordre de grandeur.

**Trouvé en passant.** Deux nœuds liés par leur clé dans un même MATCH
(`UNWIND … MATCH (a {k: ka}), (b {k: kb}) CREATE …`) se croisent — produit des deux ensembles
masqués — avant la jointure avec la liste, parce que les deux masques sont indépendants et que
rien ne lie a à b avant le probe. Ticket ouvert (`docs/tickets/2026-10-12-deux-noeuds-lies-par-leur-cle-…`) ;
les mises en place des témoins passent par un nœud par MATCH (`WITH a, kb MATCH (b {k: kb})`).
Les clés calculées (`'d-' + CAST(…)`) dans le même MATCH donnent un produit cartésien pour la
même raison.
