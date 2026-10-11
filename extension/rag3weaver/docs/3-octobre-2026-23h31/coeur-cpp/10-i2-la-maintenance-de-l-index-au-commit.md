# I2 — la maintenance de l'index vectoriel au commit : une page avant le code

11 octobre 2026, 3 h 30, session cœur C++. I1 est sur master (`3ad09c8cc`) : tout écrivain d'une
table indexée tient `INDEX{table}` en exclusif de sa première écriture à sa validation. Cette
page prépare I2, la marche d'après (page 09 §0 et §3) : **deux écrivains d'une table indexée ne
s'attendent qu'à la validation**. Elle ne promet rien : I2 n'ouvre qu'avec la mesure que le banc
a lancée cette nuit (son essai `essai-fin-d-instruction-filtre`, `53c742c0b`). **[lu]** :
cartographie d'un agent à `3ad09c8cc`, non rejouée ; **[déduit]** : raisonné ; rien n'est mesuré
par moi.

## 0. Ce qui est, à `3ad09c8cc` **[lu]**

| Écriture | Quand l'index est réécrit | Sous quoi |
|---|---|---|
| insertion | **au commit** (`NodeTable::commit` étape 3 → `UncommittedIndexInserter` → `commitInsert` → `insertInternal`) | le mutex du gestionnaire de transactions (`mtxForSerializingPublicFunctionCalls`, `transaction_manager.cpp:70`) + `INDEX` exclusif depuis la première écriture (I1) |
| mise à jour d'un vecteur | **pendant la transaction**, une ligne par appel (`NodeTable::update:653` → `OnDiskHNSWIndex::update:1085`) : retire, répare les points d'entrée, réinsère, heuristique de `110a65f15` sur les anciens voisins, `keepNodeReachable` pour le reste ; **un `HNSWUpdateState` neuf par ligne**, ses élagages différés jetés avec lui | `INDEX` exclusif (I1), hors du mutex |
| suppression | **pendant la transaction** ligne par ligne (`delete_:815`, `deleteFromGraph`), puis **à la fin de l'instruction** (`finalizeDelete:839`, déclenché par `DeleteNode::getNextTuplesInternal` à l'épuisement de l'enfant, une fois : `delete.cpp:36-52`) | idem |
| COPY | `finalize` à la fin du COPY (insère tout, vide les élagages différés) ; `flushNodeTable` appelle `NodeTable::commit` **en cours de transaction** | `INDEX` exclusif pris avant le plan |
| annulation | `rollbackInsert` seul ; une mise à jour ou une suppression annulée laisse ses arêtes réécrites et ses points d'entrée (rattrapés par `findLiveNode`) | — |
| rejeu | `replayNodeDeletionRecord` : `delete_` + `finalizeDelete` **à chaque enregistrement** ; la mise à jour rejouée ligne à ligne | — |

Trois faits qui comptent pour I2 : **il n'existe pas de `finalizeUpdate` sur master** (l'état de
mise à jour est par ligne ; les deux branches d'essai du banc l'ajoutent) ; `commitInsert` ne vide
jamais ses élagages différés (seul le seuil de `createRels` élague tout de suite) — c'est la forme
qui tient depuis des semaines ; les points d'entrée (`HNSWStorageInfo`) sont des champs nus, sans
verrou ni annulation. Et une recherche **ne voit pas les lignes insérées par sa propre transaction**
(elles n'entrent dans le graphe qu'au commit) : « lire ses propres écritures » n'est déjà pas la
règle de l'index.

## 1. Ce que I2 veut, et ce que ça change de visible

**Le but** : qu'un `SET` de vecteur ou un `DELETE` ne touche pas le graphe pendant la transaction ;
que toute la maintenance (retirer, recoudre, réinsérer les modifiés, insérer les nouveaux — l'ordre
de la page `02-hnsw-sous-plusieurs-ecrivains.md` §4) se fasse **au commit**, sous `INDEX{table}`
exclusif pris **au commit seulement**. Deux écrivains d'une même table indexée écrivent alors en
même temps et ne se sérialisent qu'à leur validation — comme deux inséreurs aujourd'hui.

**Ce qui devient visible** **[déduit]** :

- une recherche dans la transaction, après un `SET` du vecteur d'une ligne, trouve la ligne à sa
  **position d'avant** (le graphe n'a pas bougé) ; la projection rend la ligne avec son nouveau
  vecteur. Après un `DELETE`, la ligne reste dans le graphe jusqu'au commit — c'est déjà ce que le
  `CALL` nu rend (ticket du banc, « rend des nœuds supprimés au CALL »). Même règle que les
  insertions : l'index est l'état validé ;
- **l'annulation n'a plus rien à défaire** dans le graphe : la classe des points d'entrée non rendus
  et des arêtes réécrites par une transaction annulée se ferme d'elle-même (les quatre pièces
  d'`IndexedTableRollbackTest`, le ticket fermé de la recherche à une seule ligne) ;
- le rejeu applique la maintenance **au COMMIT rejoué**, pas à chaque enregistrement (la branche
  `essai-fin-d-instruction-maj` le fait déjà pour la mise à jour : « un appel juste avant le COMMIT
  rejoué ») ;
- le conflit nommé d'A4′ (« could not serialize ») ne peut plus naître des arêtes internes (elles
  ne sont réécrites que par un détenteur exclusif, au commit, sur le graphe validé).
- **limite nommée** : un lecteur pendant la maintenance d'un gros commit voit un graphe en
  recouture — c'est aujourd'hui aussi (le lecteur ne prend aucun verrou) ; « l'index = l'état
  validé » se lit avec cette réserve, pas sans.

## 2. La forme **[déduit]**

1. **Un état par transaction** dans l'index (à la place du `HNSWUpdateState` par ligne et du
   `HNSWDeleteState` par instruction) : les lignes mises à jour (offset, ancien vecteur lu avant
   l'écriture — il est perdu après), les lignes supprimées. `Index::update` et `Index::delete_` ne
   font plus que **noter** ; `finalizeDelete` ne fait plus rien ; `NodeTable::update/delete_` ne
   prennent plus `INDEX` (la clé seulement, A4′). **C'est là que l'union se définit** : une ligne
   mise à jour deux fois garde comme « ancien vecteur » le **premier** (celui qui est dans le
   graphe), jamais l'intermédiaire, et sera réinsérée au dernier ; une ligne mise à jour puis
   supprimée compte comme supprimée seulement ; une ligne insérée puis mise à jour ou supprimée
   dans la même transaction n'est pas dans le graphe : rien à retirer, elle entre (ou non) par
   `commitInsert` avec son vecteur final.
2. **Au commit**, dans `NodeTable::commit`, une étape avant l'étape 3 actuelle : prise de
   `INDEX{table}` en exclusif (prise groupée avec ce que la transaction tient déjà ; elle peut
   **attendre** là — c'est le seul endroit où deux écrivains d'une table indexée s'attendent), puis,
   dans l'ordre : retirer les lignes supprimées et les lignes modifiées du graphe (`deleteFromGraph`
   sur l'union), recoudre une fois (`cleanEdgesForNode`, `repairEntryPoint`, l'heuristique du banc
   **en filtre sur l'union de la transaction**, `keepNodeReachable` pour le reste), réinsérer les
   lignes modifiées à leur nouveau vecteur, puis `commitInsert` des nouvelles. Les élagages
   différés : non vidés, comme `commitInsert` aujourd'hui (les vider à chaque instruction a donné
   « buffer pool full », page 04 §9).
3. **Où tourne cette étape** : première forme, **sous le mutex du gestionnaire**, comme l'insertion
   aujourd'hui — un gros commit bloque les autres commits le temps de sa maintenance, exactement
   comme un COPY de 10 000 lignes aujourd'hui. C'est la forme à mesurer. La forme d'après, si la
   mesure l'exige : prendre `INDEX` et faire la maintenance **avant** d'entrer dans le mutex (dans
   `TransactionManager::commit`, entre les verrous et `mtxForSerializingPublicFunctionCalls`), le
   mutex ne couvrant plus que la pose des horodatages et des versions. Elle demande que la
   maintenance n'ait besoin d'aucune version posée par le commit : à vérifier au code, pas promis.
4. **Le rejeu** : les enregistrements de mise à jour et de suppression nourrissent l'état par
   transaction ; la maintenance se joue au `COMMIT` rejoué. Une transaction rejouée tient l'index
   seule (le rejeu est à un fil).
5. **Le COPY** garde sa forme (`finalize` à la fin du COPY, `INDEX` pris avant le plan) ; `flushNodeTable`
   en cours de transaction appelle `NodeTable::commit` : l'étape nouvelle n'y fait rien si l'état par
   transaction est vide — à garder vrai.

Ce que la marche **ne fait pas** : un lecteur pendant la maintenance d'un commit (une recherche sur
un graphe en cours de recouture) reste ce qu'il est — le lecteur ne prend aucun verrou (page 09 §3).

## 3. Ce que ça coûte, et ce que la mesure du banc tranche **[déduit]**

Le coût d'I2 est celui de l'heuristique du banc appliquée à l'union d'une transaction au lieu de
l'union d'une instruction : même algorithme, même borne (1,6 nœud recontrôlé par ligne mesuré sur
l'union par instruction, page 04 §8), une seule passe par transaction au lieu d'une par instruction
(un `SET` par ligne en auto-commit : une transaction = une instruction, le coût est le même).
Le ×7 à ×13 du crochet de fin d'instruction (page 04 §9) venait du recontrôle complet de chaque
perte, pas de l'état par instruction — c'est précisément ce que l'essai du banc isole : le crochet
**avec** l'heuristique en filtre, élagages différés non vidés.

Donc : **si `essai-fin-d-instruction-filtre` reste près de master** sur `SetFromNullLineByLine`,
`SetToAnotherVectorLineByLine` et `TenThousandRowsInBatchesOf512`, I2 ne coûte que ce que coûte
l'heuristique, et la forme du §2 (l'état par transaction, l'application au commit) est son
prolongement direct : l'état d'instruction du banc devient l'état de transaction. **Sinon**, I2 reste
fermée et I1 est le produit : un écrivain à la fois sur une table indexée, de sa première écriture au
commit — ce que rag3weaver fait déjà.

Risque propre à I2, hors mesure : la maintenance d'une transaction qui a modifié beaucoup de lignes
se fait d'un coup, sous le mutex, là où aujourd'hui elle est étalée sur les instructions — « le
commit se déplace, il ne grossit pas » (page 02 §4), mais il se déplace sous un verrou global. Le
témoin de durée : `AThousandUpdatesInOneBlock` et `TenThousandRowsInBatchesOf512`, commit compris.

### 3 bis. La mesure est rendue (banc, 11 octobre 2026, 3 h 50, page 04 §10, `23e43d966`) — tranché

Sur luciepc en exclusif, deux passes :

| | master `7c7682111` | pertes seules filtrée `53c742c0b` | union filtrée `bb50fb768` |
|---|---|---|---|
| `SetFromNullLineByLine` | 26,0 / 25,7 s | 26,7 / 26,6 s | 28,3 / 29,7 s |
| `SetToAnotherVectorLineByLine` | 25,8 / 25,8 s | 25,4 / 25,5 s | 27,4 / 27,2 s |
| `TenThousandRowsInBatchesOf512` | 26,4 / 25,7 s | 21,2 / 21,1 s | 26,1 / 26,4 s |
| `TenThousand`, dix essais | 10 verts | 2 rouges légers (993 et 998 sur 1 000) | 1 rouge grave (7 982 joignables sur 10 000) |

Le report coûte ce que coûte master, à condition de garder l'heuristique **en filtre** ; le ×7 à
×13 venait seulement de la recherche complète sur chaque perte. Aucune variante n'est plus juste
que master. **Décision (orchestration, 4 h 05)** : I2 se fait sur « pertes seules filtrée » — l'état
d'instruction devient l'état de transaction, justesse de master visée ; l'union n'est pas le choix
générique (son rouge grave est d'origine non trouvée, le banc cherche). Les deux essais restent sur
`essai-fin-d-instruction-filtre`, non fusionnés.

Ce que le banc ajoute pour §2.1 (d'accord, à deux conditions) : **ce qui vit jusqu'au commit** —
`lostLowerInEdges`, `updatedRows`, l'état d'insertion (son état de recherche, ses états de relation),
la liste des tables et colonnes touchées (le rejeu le fait déjà dans `pendingIndexUpdates`) ; **pas
d'ancien vecteur à garder** : les anciens voisins se lisent dans le graphe (`deleteFromGraph`) — si
la mise à jour du graphe est elle-même reportée, c'est l'ancien **graphe** qu'il faut, et le graphe
local de la transaction le garde ; **une ligne mise à jour deux fois** : garder l'union des anciens
voisins sur toutes les fois, pas la dernière ; **le rejeu** finalise déjà au COMMIT rejoué, même
frontière. Pour les attentes (§4) : la borne devient « `commit:done` du second > `commit:start` du
premier » pour IndexLockBench ×2, H2, H3, H5 ; `writeInTurn` disparaît (les deux écrivent ensemble,
puis valident dans l'ordre) ; le banc réécrit ces témoins sur une branche partie de la mienne au
premier jet d'I2 — le lui dire à ce moment-là.

## 4. Les témoins (rouges d'abord)

- **Le banc** : `IndexLockBench.AutoCommitWriterWaitsForTheIndexThenCommits` et
  `BlockWriterWaits…` changent de sens — l'écrivain **n'attend plus à l'écriture**, il attend au
  commit (`waited` borné `commit:start`) ; H2 ×4, H5 ×4 (« celui qui valide le premier écrit le
  premier » devient « les deux écrivent, celui qui valide le premier recoud le premier ») ; H3
  (les deux valident, aucun n'attend avant son commit) ; H1 (deux inséreurs : inchangé) ; H4 reste
  probabiliste. Leurs lignes de `known_red.txt` bougent dans le commit d'I2.
- **Un fil** (`insert_lock_test.cpp`) : deux connexions écrivent dans la même table indexée sans
  s'attendre ; la seconde à valider attend le commit de la première ; l'index rend exactement les
  lignes vivantes par `node.id`, aux bons vecteurs ; une recherche dans la transaction après un
  `SET` dit la règle du §1 (position d'avant) — le témoin la nomme pour qu'elle soit choisie, pas
  subie ; l'annulation d'un `SET` et d'un `DELETE` ne laisse rien (les quatre pièces d'`IndexedTableRollbackTest`
  passent sous le mode).
- **Le rejeu** : `vector_index_crash_reopen_test.cpp` (`AReplayedVectorUpdateReachesALoadedIndex`,
  `AReplayedDeleteReachesALoadedIndex`) et `transaction.test` (`DeleteRecovery`, `DeleteBulkRecovery`)
  restent verts avec la maintenance au COMMIT rejoué ; un témoin neuf : deux `SET` de la même ligne
  dans une transaction rejouée (seul le dernier vecteur est indexé).
- **La durée** : `vector_index_update_test.cpp` entier (`vector-index-exact`), `AThousandUpdatesInOneBlock`,
  en `poste mesure` sur luciepc, contre master — « non comparable au poste principal ».

## 5. Ce que je demande

- Au banc : la mesure de son essai, et son avis sur « l'état d'instruction devient l'état de
  transaction » (§2.1) — c'est son code ; et la borne `commit:start` pour les nouveaux `waited`.
- À l'orchestration : I2 n'ouvre qu'après la mesure ; si elle ouvre, une relecture de la forme §2.3
  (sous le mutex d'abord) avant le code. Ordre de grandeur relatif : deux à trois fois I1 (l'état
  par transaction, le commit, le rejeu, les témoins du banc à réécrire avec lui).
- À Lucie : la règle du §1 (une recherche dans la transaction voit l'index validé, comme pour les
  insertions aujourd'hui) est un choix qu'elle peut renverser — l'autre forme (consulter aussi les
  lignes en attente à la recherche, par balayage) coûte une passe de plus à chaque requête d'une
  transaction qui écrit.
