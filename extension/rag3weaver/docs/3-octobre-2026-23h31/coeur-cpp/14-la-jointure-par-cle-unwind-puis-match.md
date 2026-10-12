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
