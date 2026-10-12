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
