# Repérage : le second fork, Vela

**3 septembre 2026.** Le document 02 §8 gardait `Vela-Engineering/kuzu` pour
« le repérage suivant, si la sonde tranche vite ». Elle a tranché vite. Voici le
repérage.

C'est une **mesure**, pas une fusion. Rien n'a été fusionné, rien n'a été
rebasé, aucun code n'a été modifié.

## Le résumé en une phrase

**Ce fork est tout ce que LadybugDB n'est pas** — même histoire git, aucun
renommage, tout dans un seul dépôt — et son travail se fond dans le nôtre
presque sans frottement. Mais **il ne résout pas le problème pour lequel on
l'avait noté** : deux processus écrivant sur une même base restent interdits.

## 1. Ce qu'ils sont

Le README ne cache pas l'intention :

> *Embedded graph database for **AI agent memory**. Concurrent multi-writer
> support. MIT licensed.* — maintenu par [Vela Partners](https://vela.partners).

Ils versionnent en `v0.12.0-vela.<sha>` et publient des paquets Python et
Node.js. C'est un fork **de production**, pas une expérience.

Points de comparaison épinglés :

| tag | commit | date | quoi |
|---|---|---|---|
| `vela-master-2026-09-03` | `2efa20b67` | 14 juin 2026 | leur `master` |
| `vela-checkpoint-2026-09-03` | `373f40945` | 12 juillet 2026 | leur branche active, **pas encore fondue** |

Le distant est ajouté sous le nom `vela`. Les tags rendent les chiffres
reproductibles même s'il est retiré.

## 2. Trois différences structurelles avec Ladybug, toutes en notre faveur

C'est ici que se joue l'essentiel, et c'est l'inverse point par point du
[document 01](01-reperage-ladybug.md) §3.

**Ils n'ont pas réécrit l'histoire.** Notre dernier commit amont est leur
ancêtre :

```sh
$ git merge-base --is-ancestor 89f0263cc vela/master && echo OUI
OUI
```

Chez Ladybug, `git merge-base` remontait à **2020**. Ici la base commune est
`89f0263cc` lui-même, le jour de l'archivage. **Un vrai `git merge` est
possible.**

**Ils n'ont rien renommé.** Le namespace `kuzu` est intact, la macro `KUZU_API`
aussi ; trois fichiers de `src/` mentionnent « vela », et ce sont des URL. Il n'y
a donc pas de renommage adverse à annuler — seulement le nôtre, qu'on porte de
toute façon.

**Ils ont tout gardé dans un seul dépôt.** Aucun `.gitmodules`. `extension/`,
`tools/`, `benchmark/`, `dataset/` sont des arbres, pas des pointeurs. **HNSW est
toujours là**, `extension/vector/` au complet. C'est exactement notre structure.

Rappel de ce que ça évite : chez Ladybug, 171 de nos fichiers (13 035 lignes)
tombent dans des chemins externalisés en sous-modules et ne se fusionnent plus
fichier par fichier. Ici, ce problème n'existe pas.

## 3. Ce qu'ils ont fait, et l'ampleur

Trente-quatre commits depuis l'archivage, de février à juin 2026, sur un seul
sujet : **rendre l'écriture concurrente**.

```
2026-02-21  feat: non-blocking checkpoint for read transactions (#1)
2026-02-22  feat: wal rotation and non-blocking writers during checkpoint (#2)
2026-02-23  feat: mvcc catalog snapshots for non-blocking checkpoint (#3)
2026-02-23  feat: concurrent storage checkpoint (phase 4) (#4)
2026-03-08  fix: skip file lock for read-only database opens
2026-05-19  fix: stabilize concurrent write checkpointing (#10)
2026-05-24  feat: run auto-checkpoints in background (#11)
2026-06-06  transaction: harden checkpoint recovery (#12)
2026-06-14  transactions: default concurrent writes (#17)
```

Le reste est de la distribution : roues Python, greffon Node.js, registre
d'extensions.

| | fichiers | insertions | suppressions |
|---|---:|---:|---:|
| total | 109 | 3 545 | 561 |
| dont `src/` | 58 | 1 312 | 384 |
| hors `src/` | 51 | 2 233 | 177 |

C'est **petit**. À titre de comparaison, Ladybug a bougé 170 lignes du seul
`filter_push_down_optimizer.cpp`.

Une part notable du hors-cœur est du test : `test/transaction/transaction_test.cpp`
gagne 376 lignes, et ils ajoutent `tools/stress/agent_memory_concurrency.py`, 600
lignes de test de charge sur la mémoire d'agent.

## 4. Le recouvrement avec notre greffe : cinq fichiers

Sur les 58 fichiers du cœur qu'ils touchent et nos 27 fichiers réels, le
croisement tient en cinq lignes :

| fichier | leur écart |
|---|---:|
| `src/storage/local_storage/local_rel_table.cpp` | +122 −17 |
| `src/storage/table/node_table.cpp` | +100 −35 |
| `src/include/storage/table/node_table.h` | +15 −2 |
| `src/common/file_system/local_file_system.cpp` | +10 −0 |
| `src/include/extension/extension.h` | +4 −1 |

**Aucun de nos sites de greffe principaux n'est touché.** Ils ne vont ni dans
`filter_push_down_optimizer.cpp`, ni dans `index_scan_node_table.cpp`, ni dans
`index_search_types.h`. Leur travail est au **stockage et aux transactions** ; le
nôtre est au **plan de requête**. Orthogonaux, encore, mais cette fois sans
tension.

## 5. L'essai de fusion : 55 sur 58

Fusion à trois branches, fichier par fichier, notre renommage neutralisé — base =
l'amont Kuzu, « à nous » = notre arbre, « à eux » = leur travail :

| | fichiers |
|---|---:|
| **se fondent proprement** | **55 / 58** |
| en conflit | 3 |
| non appariables | 0 |

Trois conflits, une zone chacun :

**`src/include/extension/extension.h`** — l'URL du dépôt d'extensions. Ils
pointent vers leur registre GitHub Pages et ajoutent une variable
d'environnement `KUZU_EXTENSION_REPO` pour la surcharger. Décision d'une ligne,
et leur variable d'environnement est une bonne idée à prendre.

**`src/storage/local_storage/local_rel_table.cpp`** — **et c'est une bonne
nouvelle.** Nous avions corrigé à la main un débordement de tableau : une table
de relations à sens unique (les tables de couches HNSW) n'a pas d'index inverse,
et `directedIndices[reverseIdx]` sortait des bornes avant même la vérification.
Ils ont refondu la même boucle pour itérer directement sur le conteneur :

```cpp
for (auto& csrIndex : directedIndices) {
    auto& nodeIDVector = deleteState.getBoundNodeIDVector(csrIndex.direction);
```

Le débordement **disparaît par construction**. Leur version subsume la nôtre, et
notre correctif devient inutile. À vérifier à l'exécution avant de le jeter, mais
la forme est meilleure.

**`src/storage/table/node_table.cpp`** — le même croisement que chez Ladybug, à
un détail près. Nous avons remplacé la boucle par des états de suppression
pré-créés ; ils ajoutent un `isLocalNode` pour les nœuds non validés. Les deux se
combinent, à la main, une dizaine de lignes.

> **Coût de reprise de leur travail : de l'ordre de la demi-journée.** Trois
> zones, dont une qui simplifie notre code.

## 6. Ce qu'ils ne résolvent pas — et c'est la raison pour laquelle on les notait

Le [document 01](01-reperage-ladybug.md) §8 dit que ce fork *« touche une
question qu'on a explicitement heurtée (issue sur `F_WRLCK`, deux processus sur
une base) »*. **Il la touche à moitié, et il faut être précis sur laquelle.**

Leur correctif de verrou fait trois lignes :

```cpp
// src/storage/storage_manager.cpp
auto flag = readOnly ? FileHandle::O_PERSISTENT_FILE_READ_ONLY :
                       FileHandle::O_PERSISTENT_FILE_CREATE_NOT_EXISTS;
if (!readOnly) {
    flag |= FileHandle::O_LOCKED_PERSISTENT_FILE;    // ← inchangé pour l'écrivain
}
```

**L'écrivain prend toujours le verrou exclusif de fichier.** Un seul processus
écrivain, comme avant.

Et leur concurrence est **interne au processus**. Le gestionnaire de
transactions s'appuie sur des primitives de fil d'exécution, pas de processus :

```cpp
// src/include/transaction/transaction_manager.h
std::mutex mtxForCheckpoint;
std::mutex mtxForActiveTransactions;
std::condition_variable cvActiveTransactionsChanged;
std::atomic<uint32_t> activeTransactionCount{0};
```

Un `std::mutex` ne coordonne rien entre deux processus.

Donc, exactement :

| configuration | avant | avec Vela |
|---|---|---|
| plusieurs fils écrivains, **un** processus | non | **oui** |
| plusieurs processus **lecteurs**, un écrivain | non | **oui** |
| plusieurs processus **écrivains** | non | **non** |

Notre douleur était la troisième ligne. **Elle n'est pas levée.** La deuxième
l'est, ce qui est déjà utile si nos lecteurs sont dans d'autres processus.

## 7. Et si l'on compare les trois arbres

| | LadybugDB | Vela | ce que ça vaut |
|---|---|---|---|
| histoire git commune | non, remonte à 2020 | **oui**, `89f0263cc` | Vela : `git merge` possible |
| renommage adverse | `kuzu`→`ladybug`, ns `lbug` | **aucun** | Vela : rien à annuler |
| structure du dépôt | extensions et API en **sous-modules** | **tout dans l'arbre** | Vela : fusion fichier par fichier |
| HNSW | **absent** | présent | Vela : notre extension a sa place |
| leur axe de travail | descente de prédicat, index ART | transactions, écriture concurrente | aucun ne heurte notre greffe |
| recouvrement avec nos 27 fichiers | 22 | **5** | |
| essai de fusion | 15/24 seuls, 2 arbitrages | **55/58 seuls**, 3 conflits | |

Les deux forks sont **complémentaires, pas concurrents**. Ils travaillent sur des
axes disjoints, et aucun des deux ne fait ce que fait notre greffe.

## 8. Ce que ce repérage ne dit pas

- **Rien n'a été compilé.** Le build de Vela n'a pas été tenté, pas plus que
  celui de Ladybug.
- **Leur correction est-elle correcte ?** Le fond de leur travail — MVCC,
  rotation de WAL, points de reprise non bloquants — n'a pas été relu. Un fork
  qui rend l'écriture concurrente sur un moteur conçu pour un seul écrivain
  mérite d'être examiné avant d'être cru. Leurs 976 lignes de test ajoutées sont
  un signe encourageant, pas une preuve.
- **Leur `master` est en retard sur leur propre travail.** La branche
  `storage/concurrent-checkpoint-recovery` porte six commits de juillet 2026 non
  fusionnés, dont *« recover partial checkpoint installs »* et *« fence failed
  rotated checkpoints »*. Reprendre `master` seul, c'est prendre une version dont
  ils corrigeaient encore la reprise sur panne un mois plus tard.
- **L'écriture multi-processus reste ouverte.** Aucun des deux forks ne la
  résout. Si on la veut, c'est notre travail, et le verrou de `storage_manager.cpp`
  en est la porte.

## 9. Ce que ça suggère, sans le décider

La reprise la moins chère n'est probablement pas de choisir un fork, mais de
**prendre chez chacun ce qui est disjoint** : la concurrence intra-processus de
Vela, qui se fond presque seule ; et, de Ladybug, rien pour l'instant, puisque
leur descente de prédicat ne nous sert pas et que leur découpage en dépôts coûte
cher.

Ce n'est pas une décision, c'est ce que les mesures penchent à dire. La décision
demande de savoir si leur concurrence est correcte, ce qui n'est pas mesuré ici.
