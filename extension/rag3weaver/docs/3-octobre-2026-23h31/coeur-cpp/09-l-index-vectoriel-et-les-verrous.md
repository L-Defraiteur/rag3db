# L'index vectoriel et les verrous — une page avant le code

11 octobre 2026, 1 h 30, session cœur C++. A3′, A4′ et V2 sont sur master (`11887dc21`). Cette
page prépare la dernière marche des verrous (page 07 §2, « l'index vectoriel au commit »), et
propose de la couper en deux : d'abord **le genre « index »** tel que la note du 3 octobre et
le banc l'attendent (tout écrivain d'une table indexée tient l'index en exclusif jusqu'à sa
validation), petit et mesurable ; ensuite, et seulement sur mesure, **la maintenance au commit**
(deux écrivains d'une table indexée ne s'attendent qu'à la validation). **[lu]** : cartographie
d'un agent à `11887dc21`, non rejouée ; **[déduit]** : raisonné ; rien n'est mesuré.

## 1. Ce que l'index fait aujourd'hui, et quand **[lu]**

| Écriture | Quand | Ce qu'elle touche |
|---|---|---|
| insertion d'une ligne | **au commit** (`needCommitInsert`, `NodeTable::commit` étape 3 → `commitInsert` → `insertInternal`), sous le mutex du gestionnaire de transactions | les arêtes (tables de relations internes, versionnées, dans la transaction) ; `shrinkForNode` réécrit des arêtes **validées** des voisins ; les points d'entrée et `numCheckpointedNodes` (`HNSWStorageInfo`, en mémoire, **sans verrou, sans annulation**) |
| mise à jour d'un vecteur | **pendant la transaction** (`OnDiskHNSWIndex::update`) : retire les arêtes sortantes, répare le point d'entrée, réinsère, recontrôle les anciens voisins | arêtes validées des voisins, points d'entrée |
| suppression | **pendant la transaction**, ligne par ligne (`delete_`), puis **à la fin de l'instruction** (`finalizeDelete` : nettoyage des voisins, réparation du point d'entrée, `keepNodeReachable`) | idem |
| annulation | `rollbackInsert` corrige les points d'entrée d'une insertion annulée ; une suppression annulée **ne rend pas** le point d'entrée (rattrapé par `findLiveNode`) | |

Les écritures internes ne prennent aucun verrou d'extrémités (`takesLocks = false`, A4′) et ne
vont pas au journal (`logToWAL = false`). Aucun mutex dans l'index ; aucun verrou `INDEX` n'est
pris au commit ni par `update`/`delete_` : l'insertion le prend en **partagé** (A3′), le COPY et
l'annonce sur une table indexée en **exclusif**.

Ce que ça donne à deux écrivains : deux suppressions dont les voisinages se recouvrent réécrivent
les mêmes arêtes validées pendant leurs transactions → « Write-write conflict » pour le second
(H3, rouge attendu) ; deux insertions sous l'index en partagé réécrivent au commit les arêtes des
mêmes voisins, l'une après l'autre sous le mutex du gestionnaire — ça tient, mais les points
d'entrée sont lus et écrits sans verrou (H5, H2 : verts, « couvert, pas protégé »).

## 2. Ce que le banc attend **[lu]**

- `IndexLockBench.AutoCommitWriterWaitsForTheIndexThenCommits` et
  `BlockWriterWaitsForTheIndexThenCommitsOrGetsSerializationError` : rouges `waited` — « tout
  écrivain d'une table indexée prend l'index de la table en exclusif » (known_red, genre
  « index » de V1). Deux inséreurs de clés différentes doivent **s'attendre**.
- `H3_IndexedDeletesWithOverlappingNeighbourhoods` (Hot, Crash) : rouges `all-commit,row-count` —
  les deux suppressions doivent valider.
- H2, H5 (verts) doivent le rester ; H4 reste probabiliste (le processus meurt parfois).

Le commentaire de la page 07 visait plus loin : « deux écrivains sur la même table indexée ne
s'attendent qu'à la validation ». Le banc, lui, demande aujourd'hui qu'ils s'attendent.

## 3. La proposition : deux marches, pas une

### Marche I1 — le genre « index » : exclusif de la première écriture au commit

Sous le mode multi-écrivains, pour une table qui porte un index secondaire chargé (vectoriel) :

- `NodeTable::insert` prend `INDEX{table}` en **exclusif** au lieu de partagé (l'index le sait
  par `indexes` : un holder non primaire chargé) ; `update` et `delete_` d'une ligne validée le
  prennent aussi, en exclusif, avant la clé ; tout tient jusqu'à la fin de la transaction.
- L'annonce (`acquire_locks`) le prend déjà en exclusif sur une table indexée ; le COPY aussi.
- Rien ne change pour une table sans index secondaire (l'insertion garde le partagé), ni hors
  du mode.

Ce que ça ferme : H3 (le second attend le premier, puis réécrit des arêtes déjà validées ; A4′
ne lève pas l'erreur de sérialisation sur les arêtes, qui sont des écritures internes) ; les
deux `IndexLockBench` ; les courses sur les points d'entrée entre écrivains (un seul à la fois).
Ce que ça ne ferme pas : un **lecteur** (une recherche) pendant qu'un écrivain réécrit le graphe —
hors de cette marche, comme aujourd'hui.

Coût **[déduit]** : zéro hors du mode ; sous le mode, les écrivains d'une table indexée se
sérialisent de leur première écriture à leur commit — c'est ce que la note du 3 octobre
promettait (« l'index … exclusif pour tout écrivain d'une table indexée, de sa première écriture
au commit »), et ce que rag3weaver fait déjà de fait (un écrivain à la fois).

Témoins : les deux `IndexLockBench` et H3 passent au vert et sortent de `known_red` ; H2, H5
restent verts ; un témoin d'un fil : deux connexions, la seconde attend sur une mise à jour de
vecteur et sur une suppression, puis valide, et l'index rend les lignes vivantes.

### Marche I2 — la maintenance au commit (plus tard, sur mesure)

Ramener `update`, `delete_` et `finalizeDelete` au commit, sous le même `INDEX` exclusif pris
**au commit** seulement, pour que deux écrivains ne s'attendent qu'à la validation. Elle dépend
de l'état par instruction de la page 04 du banc — dont la fin d'instruction (le recontrôle sur
l'union des voisins) est **gardée à part** parce qu'elle fait régresser le ligne à ligne de ×7
à ×13. Tant que ce coût n'est pas compris, promettre la même maintenance au commit serait
promettre la même régression, ou pire (tout le recontrôle d'une transaction d'un coup, sous le
mutex du gestionnaire, où les autres attendent). Donc : I2 ne s'ouvre qu'avec une mesure du
banc qui dit ce que coûte la fin d'instruction et pourquoi, et une forme qui ne régresse pas.

## 4. Ce que je demande

- À l'orchestration : I1 maintenant (une journée d'agent : trois prises de verrou, un témoin,
  trois lignes de `known_red`), I2 après la mesure du banc. Ou l'inverse, si Lucie veut
  d'abord que les écrivains d'une table indexée ne s'attendent pas — alors la mesure d'abord.
- Au banc : confirmer que H3 attend bien `all-commit` sous I1 (le second valide après avoir
  attendu), et dire ce que sa branche `essai-fin-d-instruction-maj` a mesuré, pour I2.
