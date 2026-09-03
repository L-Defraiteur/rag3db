# Knowledge dump — de quoi mener la sonde sans rien redécouvrir

Écrit pour une session qui ne connaît pas le projet. Ce qui suit est vérifié au
3 septembre 2026 ; ce qui ne l'est pas est signalé comme tel.

## 1. Ce qu'est ce dépôt

`rag3db` est un **fork de Kuzu**, une base de graphe embarquée en C++. Le fork
porte Kuzu **jusqu'à son archivage** (10 octobre 2025) et y ajoute :

- un renommage global `kuzu` → `rag3db` (1 608 fichiers, un seul commit
  `c647fbb33`) ;
- **29 fichiers de modifications réelles du cœur**, 542 lignes, toutes autour
  d'un mécanisme unique — voir [03](03-notre-greffe.md) ;
- des extensions, dont `extension/vector` (HNSW) qui est le seul consommateur de
  ce mécanisme ;
- `extension/rag3weaver/`, un **crate Rust indépendant** qui n'a rien à voir avec
  cette mission. **Ne pas y toucher** : d'autres sessions y travaillent.

## 2. Repères géographiques

| chemin | quoi |
|---|---|
| `src/` | le cœur C++ de la base. **C'est là que se joue la mission.** |
| `src/optimizer/filter_push_down_optimizer.cpp` | notre plus gros greffon (119 l.) |
| `src/include/common/index_search_types.h` | le contrat, entièrement à nous |
| `src/processor/operator/scan/index_scan_node_table.cpp` | l'exécution du scan par index |
| `extension/vector/` | HNSW — le consommateur vivant du mécanisme |
| `extension/lucivy/ld-lucivy/` | **sous-module** : le moteur lucivy (Rust) |
| `extension/rag3weaver/` | crate Rust, hors sujet ici |
| `extension/fts`, `extension/postgres`, `extension/neo4j` | extensions **d'amont** (Kuzu), pas à nous — ne pas les confondre avec nos dialectes Rust du même nom |
| `docs/` | docs du fork C++ et de ses extensions |
| `extension/rag3weaver/docs/` | docs du crate Rust — autre monde |

**Sous-modules** : un `git clone` a besoin de `--recursive`.

```
third_party/fuzzy-fst
third_party/tantivy-search
extension/lucivy/ld-lucivy
extension/rag3weaver/codeparsers
```

## 3. Les points de repère git

| ref | quoi |
|---|---|
| `89f0263cc` | **notre dernier commit Kuzu**, 10 oct. 2025 (*« remove logo »*). La base de comparaison correcte. |
| `v0.11.2` | un tag de branche de version. **Pas notre base** — s'en servir fait lire 462 fichiers au lieu de 29. |
| `ladybug-main-2026-08-31` | tête de `ladybug/main` au moment du repérage (`bdc162654`). Épinglée pour que les chiffres restent reproductibles. |

Le distant Ladybug a été retiré après le repérage. Pour le rouvrir :

```sh
git remote add ladybug https://github.com/LadybugDB/ladybug.git
git fetch ladybug
```

Attention : **retirer un distant supprime ses refs de suivi**. Les objets
survivent jusqu'au prochain `git gc`, d'où le tag.

Comparer un fichier chez eux et chez nous, renommages annulés — leur namespace
est `lbug` et leur macro d'export `LBUG_API`, pas seulement `ladybug` :

```sh
git show ladybug-main-2026-08-31:src/include/storage/index/index.h \
  | sed 's/lbug/kuzu/g; s/LBUG_API/KUZU_API/g' > /tmp/leur.h
git show 89f0263cc:src/include/storage/index/index.h > /tmp/notre.h
diff /tmp/notre.h /tmp/leur.h
```

Sans cette normalisation on surestime largement l'écart. Ça s'est déjà produit
pendant le repérage.

## 4. Construire

Le guide complet est [`docs/builds-et-tests.md`](../builds-et-tests.md). Pour
cette mission, seul le build natif compte :

```sh
mkdir -p build/release && cd build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . -j $(( $(nproc) - 2 ))
```

**Le `-j $(( $(nproc) - 2 ))` n'est pas une coquetterie.** Un `-j$(nproc)` fige
le poste pendant toute la compilation C++. Laisser deux cœurs libres.

Le guide donne aussi des invocations avec `-DBUILD_EXTENSIONS="lucivy_fts"` :
**elles sont périmées**. Cette extension a été supprimée (commit `a39698fd4`) et
aucun `CMakeLists` ne la déclare plus. Le guide n'a pas été mis à jour.

## 5. Les pièges qui coûtent une demi-journée

**Rediriger vers un fichier, puis lire le fichier.** Jamais un tuyau filtrant en
bout de chaîne sur une longue compilation — on perd la sortie et on ne peut pas y
revenir.

```sh
cmake --build . -j $(( $(nproc) - 2 )) > /tmp/build.log 2>&1
tail -40 /tmp/build.log
```

**`pkill -f <motif>` tue son propre shell** quand le motif apparaît dans sa
propre ligne de commande. Mettre une lettre entre crochets :

```sh
pkill -f 'mon-processu[s]'
```

**Deux `cargo`/`cmake` d'affilée dans une même commande** : on lit la sortie du
second en croyant lire celle du premier. Une commande, une lecture.

**Ne pas conclure sur la foi d'une signature.** Le repérage a d'abord conclu que
la greffe servait le plein texte. Il a suffi de chercher **qui appelle** le code
(`grep -rn isIndexScanPredicate`) pour voir qu'elle sert le vecteur, et que
l'extension plein texte avait été supprimée. Chercher les appelants avant de
raconter à quoi sert quelque chose.

**Ne pas travailler dans l'arbre principal** si la sonde doit compiler quelque
chose de Ladybug. `git worktree add` un dossier séparé, et le dire.

## 6. Vérifications utiles, toutes prêtes

Qui utilise le mécanisme d'index scan :

```sh
grep -rn "isIndexScanPredicate" --include="*.cpp" --include="*.h" src/ extension/
grep -rn "IndexSearchBindData" --include="*.cpp" --include="*.h" src/ extension/
```

Nos modifications réelles d'un fichier :

```sh
git diff 89f0263cc HEAD -- src/optimizer/filter_push_down_optimizer.cpp
```

Ce que Ladybug a fait d'un fichier depuis l'archivage :

```sh
git log --format='%h %ad %an — %s' --date=short --since=2025-10-10 \
  ladybug-main-2026-08-31 -- src/optimizer/filter_push_down_optimizer.cpp
```

Les scripts du repérage (séparation renommage / vrai travail, comparaison
fichier par fichier) ont vécu dans un dossier temporaire et n'ont pas été
conservés. Ils tiennent en trente lignes de Python chacun et le
[document 01](01-reperage-ladybug.md) décrit ce qu'ils font ; les réécrire coûte
moins cher que de les retrouver.

## 7. Ce qui n'est pas su

- **Le build de Ladybug n'a jamais été tenté** sur ce poste. Rien ne dit qu'il
  passe.
- **Les 187 fichiers partagés hors du cœur** (`extension/geo`,
  `extension/vector`, `tools/rust_api`, `tools/java_api`, `tools/wasm`) n'ont pas
  été comparés à Ladybug. C'est la mesure manquante du coût d'un rebasage.
- **`docs/builds-et-tests.md` est partiellement périmé** — au moins sur
  `lucivy_fts`. Les autres sections n'ont pas été revérifiées.
