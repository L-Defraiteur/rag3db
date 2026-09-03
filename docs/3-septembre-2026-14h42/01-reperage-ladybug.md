# Repérage de fusion : LadybugDB

**3 septembre 2026.** Le repérage était noté « pas fait » dans les deux derniers
rapports de session. Il l'est. C'est une **mesure et une décision**, pas une
fusion : rien n'a été fusionné, rien n'a été modifié.

## 1. Ce que la mesure corrige d'abord

Les rapports disaient : *« on a forké à Kuzu v0.11.2.2, nos modifications du
cœur font 26 fichiers, 520 insertions »*. Le chiffre est bon. Le point de fork
ne l'est pas, et l'erreur est en notre faveur.

`git merge-base HEAD v0.11.2` répond `c89ca802d`, un commit **avant** le tag —
d'où l'idée qu'on serait huit versions mineures en retard. C'est faux : on ne
suit pas la branche de version, on suit la **branche principale de Kuzu**, et
notre dernier commit amont est `89f0263cc`, *« remove logo »*, du **10 octobre
2025**. C'est le jour où le dépôt Kuzu a été archivé.

**On porte Kuzu jusqu'à sa mort.** Il n'y a pas de retard sur Kuzu à rattraper ;
l'écart est exactement le travail propre de LadybugDB depuis le même point.

Mesuré depuis ce commit-là, sur `src/` :

| | fichiers | |
|---|---:|---|
| touchés | 1 637 | |
| dont **purement le renommage** `kuzu` → `rag3db` | 1 608 | mécanique, fait en un commit (`c647fbb33`) |
| **réellement à nous** | **29** | **542 lignes** |

Le vrai chiffre est donc bien de l'ordre annoncé — mais il fallait la bonne base
pour le voir. Avec la mauvaise, on lisait 462 fichiers et 13 504 lignes, et on
se serait cru devant une montagne.

## 2. Ce que nos 542 lignes font

Un seul sujet : **la descente de prédicat vers l'index plein texte, et le scan
par index**.

```
119  src/optimizer/filter_push_down_optimizer.cpp
 61  src/processor/operator/scan/index_scan_node_table.cpp     ← à nous
 60  src/include/processor/operator/scan/fts_scan_node_table.h ← à nous
 59  src/include/common/index_search_types.h                   ← à nous
 59  src/include/processor/operator/scan/index_scan_node_table.h ← à nous
 33  src/storage/table/node_table.cpp
 …
  4  src/include/common/fts_types.h                            ← à nous
```

Cinq fichiers sont **entièrement de nous** : ils n'existent pas chez Kuzu. Le
reste est du greffon sur des fichiers existants.

## 3. L'obstacle structurel : il n'y aura pas de `git merge`

Deux raisons indépendantes, chacune suffisante.

**LadybugDB a réécrit son histoire au fork.** Nos commits Kuzu ne sont pas leurs
ancêtres : `23551e30c` chez nous et `d184591eb` chez eux sont le *même* travail
amont (« Add index hashing for uint128 (#6052) ») sous deux SHA différents.
`git merge-base HEAD ladybug/main` remonte à **2020**. Git ne voit aucune
parenté récente entre les deux arbres.

**Et les deux arbres portent un renommage global, en sens contraire.** Le nôtre
est `kuzu` → `rag3db` (1 608 fichiers). Le leur est `kuzu` → `ladybug`, et il va
plus loin que le nôtre : le *namespace* devient `lbug`, le macro d'export
devient `LBUG_API`. Même avec une parenté commune, une fusion entrerait en
conflit sur presque chaque fichier — pour des raisons de nom, pas de fond.

La reprise, si elle se fait, sera donc un **rebasage par le contenu** : prendre
leur arbre, rejouer notre renommage (scriptable, déjà fait une fois), reposer
nos patchs. Pas un `git merge`.

## 4. Ce qu'ils ont fait de nos 29 fichiers

Comparaison du contenu **amont** (Kuzu au 10 octobre 2025) à leur contenu
d'aujourd'hui, renommages annulés des deux côtés :

| verdict | fichiers | ce que ça veut dire |
|---|---:|---|
| intacts chez eux | 2 | notre patch se réapplique tel quel |
| **modifiés chez eux** | **22** | à relire un par un |
| entièrement de nous | 5 | rien à fusionner |
| supprimés chez eux | 0 | aucune zone n'a disparu |

Et les deux plus gros écarts tombent **exactement sur nos deux sites de greffe
principaux** :

```
326  src/storage/table/node_table.cpp
170  src/optimizer/filter_push_down_optimizer.cpp
```

## 5. Pourquoi ils ont bougé — et c'est ça, la trouvaille

Ce n'est pas de la dérive. Leurs commits sur `filter_push_down_optimizer.cpp`
depuis l'archivage, du plus récent au plus ancien :

```
2026-06-10  Support secondary ART indexes
2026-06-08  Add stats-aware query optimization
2026-05-20  Keep unpushed table function filters
2026-05-16  Guard ART range predicate pushdown
2026-05-15  Use primary key scan only with indexes
2026-05-15  Add ART primary key range scans
```

**Ils ont construit la version générale de ce qu'on a construit en particulier.**
Nous avons greffé une descente de prédicat vers *un* index — le plein texte. Ils
ont fait un mécanisme d'index secondaires enfichables, avec descente de prédicat
et optimisation informée par les statistiques.

Le signe le plus net est dans l'interface elle-même. `storage/index/index.h`
passe de **19 à 25 méthodes virtuelles**, et gagne :

```cpp
enum class IndexConstraintType : uint8_t {
    PRIMARY = 0,
    SECONDARY_NON_UNIQUE = 1,
};
```

Ils ont aussi un `src/processor/map/map_index_scan_node.cpp` que nous n'avons
pas.

## 6. La décision — après vérification de ce que la greffe sert vraiment

**Correction d'une conclusion trop rapide.** La première version de ce document
disait : « nos 542 lignes existent pour brancher le plein texte, la question est
de savoir si `lucivy_fts` peut devenir un index secondaire chez eux ». C'est
faux, et il a suffi de regarder qui appelle notre code pour le voir.

**L'extension C++ `lucivy_fts` a été supprimée** — commit `a39698fd4`,
*« chore(fts): supprimer l'extension C++ lucivy_fts (code mort) »*. Le plein
texte tourne désormais entièrement en Rust dans le processus (`ShardedHandle`,
`fts_handle.rs`) et ne touche plus le cœur C++ du tout.

Le seul consommateur vivant de notre greffe est **l'extension vecteur** :

```
extension/vector/src/function/vector_search_function.cpp:244:  func->isIndexScanPredicate = true;
extension/vector/src/function/vector_search_function.cpp:30:   struct VectorSearchBindData final : IndexSearchBindData {
```

Notre patch fait donc descendre un prédicat de **similarité vectorielle** vers un
scan d'index HNSW. Et notre modification de `storage/index/index.h` (4 lignes)
ajoute `finalizeDelete`, dont le commentaire dit : *« Override in extensions that
need batched cleanup (e.g., HNSW) »*.

Au passage : **deux de nos cinq fichiers neufs sont morts**.
`fts_scan_node_table.h` n'est inclus par personne, et `fts_types.h` seulement par
lui. Environ 64 lignes à supprimer, résidus du retrait de l'extension.

### Et leur généralisation ne va pas dans notre direction

Les six méthodes virtuelles que Ladybug a ajoutées à `Index` :

```
lookupAll            scanPrimaryKeyRange     discardPrimaryKey
lookupPrimaryKey     getStorageEntries       reclaimStorage
```

Toutes tournent autour de la **clé** : recherche exacte, parcours d'intervalle,
gestion du stockage. Aucune n'a la forme d'une recherche **classée** —
`(requête, k) → (offset, score)`. Et aucune n'est notre `finalizeDelete`.

C'est cohérent avec leurs commits : ils ont généralisé pour l'**ART**, un index
ordonné. Nous avons généralisé pour la **similarité**, qui n'est pas un
prédicat de comparaison. **Les deux généralisations sont probablement
orthogonales**, et dans ce cas notre patch survit à un rebasage au lieu de s'y
dissoudre.

### Ce que ça change pour la suite

La question devient :

> **Leur descente de prédicat peut-elle atteindre un index non ordonné, ou est-elle
> câblée pour les comparaisons ?**

- Si elle est généralisée : notre greffe se réduit, peut-être beaucoup.
- Si elle est câblée pour l'ordre — ce que les signatures laissent croire — alors
  nos ~478 lignes vivantes se réappliquent, et le coût du suivi reste le
  renommage. Mais notre `IndexSearchBindData` devient une **contribution amont
  évidente** : ils en auront besoin le jour où ils voudront pousser un prédicat
  vectoriel ou plein texte.

**Prochain pas : une sonde de faisabilité**, pas une fusion — décrite dans
[02 — La mission](02-la-mission-et-son-critere.md).

## 7. Le coût réel du rebasage, hors du cœur

Le cœur n'est pas le plus cher. Hors `src/` :

| | fichiers | |
|---|---:|---|
| **à nous seuls** (`extension/rag3weaver/`) | 410 | aucun conflit possible : c'est notre répertoire |
| **partagés avec l'amont** | 187 | la vraie surface de rebasage |

Les 187 se concentrent sur : `extension/geo` (l'index spatial R-tree, à nous),
`extension/vector` (HNSW), `tools/rust_api`, `tools/java_api`, `tools/wasm`.
C'est là qu'il faudra regarder ensuite — et ce document ne le mesure pas.

## 8. Ce qui n'est pas tranché

- **La sonde du §6** n'est pas faite. Tout ce document en dépend.
- **Les 187 fichiers partagés hors du cœur** ne sont pas mesurés. `extension/geo`
  et `extension/vector` peuvent réserver la même surprise que le cœur — en bien
  ou en mal.
- **Il existe un second fork** : `Vela-Engineering/kuzu`, MIT aussi, orienté
  mémoire d'agent avec écriture concurrente multi-processus. Ça touche une
  question qu'on a explicitement heurtée (issue sur `F_WRLCK`, deux processus sur
  une base). Non évalué.
- **Le renommage lui-même mérite d'être questionné.** Il coûte 1 608 fichiers à
  chaque reprise. S'il n'a pas de raison forte, le garder est une dépense
  récurrente pour un bénéfice à établir.
