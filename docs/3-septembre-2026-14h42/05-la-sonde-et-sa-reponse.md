# La sonde : la réponse

**3 septembre 2026.** La sonde décrite dans [02 — La mission](02-la-mission-et-son-critere.md)
est faite. Elle s'est répondue **par la lecture seule**, sans compiler quoi que
ce soit de LadybugDB, comme la branche A l'autorisait.

Rien n'a été fusionné, rien n'a été rebasé, rien n'a été renommé. La seule
modification du dépôt est le nettoyage du code mort prévu au §5 du document 02.

## Les trois phrases du critère

### 1. Leur descente de prédicat n'atteint pas un index classé

**Elle ne l'atteint pas, et elle est fermée deux fois plutôt qu'une** — par la
forme du prédicat *et* par le type d'index. L'arbre de décision du document 02
prévoyait ces deux formes comme exclusives ; elles sont cumulées.

**Fermeture par la forme du prédicat.** Leur ensemble de prédicats trie à
l'entrée sur un seul critère :

```cpp
// src/optimizer/filter_push_down_optimizer.cpp:417
void PredicateSet::addPredicate(std::shared_ptr<Expression> predicate) {
    if (predicate->expressionType == ExpressionType::EQUALS) {
        equalityPredicates.push_back(std::move(predicate));
    } else {
        nonEqualityPredicates.push_back(std::move(predicate));
    }
}
```

Un appel `vector_search(...)` est de type `FUNCTION`. Il tombe donc dans
`nonEqualityPredicates`, et les trois seules sorties de ce panier lui sont
fermées :

| sortie | ligne | ce qu'elle exige | verdict pour un appel de fonction |
|---|---|---|---|
| index secondaire | `:219` | itère `equalityPredicates` **uniquement** | jamais atteint |
| intervalle sur clé primaire | `:469` | `ExpressionTypeUtil::isComparison(...)` | rejeté |
| fonction de table | `:318` | `ColumnPredicateUtil::tryConvert` | rend `nullptr` |

La dernière est la plus nette, parce que c'est leur seul crochet qui ressemble à
un mécanisme générique. Il ne l'est pas :

```cpp
// src/storage/predicate/column_predicate.cpp
std::unique_ptr<ColumnPredicate> ColumnPredicateUtil::tryConvert(...) {
    if (ExpressionTypeUtil::isComparison(predicate.expressionType)) { ... }
    switch (predicate.expressionType) {
    case ExpressionType::IS_NULL:     ...
    case ExpressionType::IS_NOT_NULL: ...
    default:
        return nullptr;          // ← tout appel de fonction sort ici
    }
}
```

**Fermeture par le type d'index.** Même un prédicat d'égalité bien formé ne
descend que vers l'ART, par comparaison de chaîne, à deux endroits :

```cpp
// :236   et, pour l'intervalle, :288
!StringUtils::caseInsensitiveEquals(indexEntry->getIndexType(),
        ArtPrimaryKeyIndex::getIndexType().typeName)
```

**Et le véhicule n'a pas de place pour un score.** Ce qu'ils accrochent au scan
ne transporte qu'une clé :

```cpp
// src/include/planner/operator/scan/logical_scan_node_table.h
struct SecondaryIndexScanInfo final : ExtraScanNodeTableInfo {
    std::string indexName;
    std::shared_ptr<binder::Expression> key;
};
enum class LogicalScanNodeTableType : uint8_t { SCAN, PRIMARY_KEY_SCAN, SECONDARY_INDEX_SCAN };
```

Ni `k`, ni canal de sortie pour un score. Notre `IndexScanInfo` transporte une
*closure*, une limite, et des expressions virtuelles pour rendre le score.

**Le compte de fréquence confirme l'absence, il ne la démontre pas seul.** Sur
tout leur `src/` :

| terme | chez eux | chez nous |
|---|---:|---:|
| `isIndexScanPredicate` | 0 | 2 |
| `IndexSearchBindData` | 0 | 2 |
| `IndexSearchResult` | 0 | 2 |
| `searchFunc` | 0 | 7 |
| `SEARCH_SCORE` / `VECTOR_DISTANCE` | 0 | 3 |

**Et la raison de fond : ils n'ont pas d'extension vectorielle du tout.**
Aucun HNSW nulle part dans leur arbre. Ils n'ont donc jamais eu le problème que
notre greffe résout. Leur généralisation sert l'ART, un index ordonné, parce que
c'est le seul index secondaire qu'ils possèdent.

Leurs six nouvelles virtuelles sur `storage::Index` disent la même chose. La
seule qui accepte une borne de résultats, `scanPrimaryKeyRange`, prend un
`maxResults` mais rend un `std::vector<common::offset_t>` — **des décalages sans
score**. Un top-K classé n'y entre pas.

> **Les deux généralisations sont orthogonales.** L'hypothèse de travail du
> document 02 est confirmée, sur le code et non sur les signatures.

### 2. Notre greffe se réapplique, et devient une contribution amont

Mesuré par un vrai essai de fusion à trois branches, fichier par fichier :
base = l'amont Kuzu au 10 octobre 2025, « à nous » = notre greffon, « à eux » =
leur arbre — les deux renommages neutralisés.

| | fichiers |
|---|---:|
| greffons partagés à reposer | 24 |
| **se reposent seuls, sans intervention** | **15** |
| demandent une intervention | 9 |

Les 9 pèsent 12 zones à rouvrir, et **la plupart ne sont pas des désaccords** :

| nature | zones | ce que ça coûte |
|---|---:|---|
| collision d'emplacement — les deux côtés **ajoutent** au même point, la base est vide | 7 | garder les deux, mécanique |
| notre patch est un **résidu de renommage** sans valeur | 3 | prendre le leur, coût nul |
| **arbitrage réel** | 2 | à écrire à la main |

Les trois résidus méritent d'être nommés, parce qu'ils gonflaient le compte :
notre « modification » de `local_file_system.cpp` n'est qu'une URL de
documentation (`docs.kuzudb.com` → `docs.kuzu.com`), et celle de `extension.h`
l'adresse du dépôt d'extensions. Ni l'une ni l'autre n'est du travail.

Les **deux arbitrages réels** :

- `src/planner/operator/scan/logical_scan_node_table.cpp` — ajouter `INDEX_SCAN`
  à leur aiguillage et au schéma plat. Environ 5 lignes.
- `src/storage/table/node_table.cpp` — le seul vrai croisement. Ils ont ajouté
  une garde `isLoaded()` pour sauter les porteurs d'index orphelins ; nous avons
  remplacé la boucle par des états de suppression pré-créés. Les deux se
  combinent, mais il faut le faire à la main. Environ 9 lignes.

S'y ajoutent nos fichiers entièrement neufs, qui se déposent sans conflit
possible. Ils étaient cinq ; **deux étaient morts et viennent d'être supprimés**,
il en reste trois, environ 178 lignes.

> **Coût de la greffe du cœur sur leur arbre : une demi-journée à une journée.**
> Deux zones à écrire, sept à recoller, trois patchs à jeter.

Et la conclusion du document 01 tient : **notre `IndexSearchBindData` est une
contribution amont évidente.** Ils ont bâti la descente de prédicat vers un index
secondaire, mais uniquement pour l'égalité sur clé. Le jour où ils voudront
pousser un prédicat vectoriel ou plein texte, il leur faudra exactement ce que
nous avons — un contrat `(requête, k) → (décalage, score)` posé dans le cœur, et
un canal pour rendre le score. Nous ne dupliquons pas leur travail, nous
complétons l'axe qui leur manque.

### 3. Le rebasage complet — et la surprise qui en change la forme

**Le repérage cherchait 187 fichiers partagés hors du cœur. La question ne se
pose plus dans ces termes : LadybugDB a sorti ces répertoires du dépôt.**

```
$ git show ladybug-main-2026-08-31:.gitmodules
[submodule "extension"]    url = https://github.com/ladybugdb/extensions.git
[submodule "tools/rust_api"]  url = https://github.com/ladybugdb/ladybug-rust
[submodule "tools/java_api"]  url = https://github.com/ladybugdb/ladybug-java
[submodule "tools/wasm"]      url = https://github.com/ladybugdb/ladybug-wasm
  … ainsi que benchmark, dataset, tools/nodejs_api, tools/python_api
```

Ce sont des entrées `160000`, des pointeurs, pas des fichiers. `extension/geo`,
`extension/vector`, `tools/rust_api` n'existent plus comme contenu dans leur
dépôt principal.

Conséquence directe : **171 de nos fichiers, 13 035 lignes, ne peuvent pas être
fusionnés fichier par fichier avec leur arbre.** Ce n'est plus le même dépôt. Ce
n'est pas un coût de fusion, c'est un choix de structure : ou bien on clone
chacun de leurs dépôts d'extensions et on rebase dans chacun, ou bien on garde
nos extensions dans notre arbre principal et on diverge de leur découpage. Ce
coût-là **n'est pas chiffrable depuis ce dépôt**, et il ne faut pas prétendre le
chiffrer.

Ce qui reste et qui est mesuré :

| poste | volume | nature |
|---|---:|---|
| le renommage, arbre entier | 2 899 fichiers, 17 299 occurrences | **mécanique**, un script |
| nos docs (`docs/`) | 245 fichiers, 61 515 lignes | ajouts purs, aucun conflit |
| le cœur (`src/`) | 24 greffons | 15 seuls, 2 arbitrages |
| hors cœur, réellement comparable | **14 fichiers, 463 lignes** | marginal |
| hors cœur, en sous-module chez eux | 171 fichiers, 13 035 lignes | **non mesurable ici** |

Hors du cœur, le seul patch de fond est dans le `CMakeLists.txt` racine : 14
lignes ajoutant `-include cstdint` pour GCC 13 et suivants, dans une zone où ils
ont bougé 208 lignes. Un quart d'heure. Les autres écarts notables — `LICENSE`,
`README.md` — sont des remplacements complets où l'on garde le nôtre.

> **Le rebasage du cœur est petit et connu. Le vrai chantier n'est pas le code,
> c'est le découpage en dépôts.** Répondre à cette question-là est un préalable à
> toute reprise, et elle n'a rien à voir avec la descente de prédicat.

## Ce que la sonde corrige dans les documents précédents

Deux erreurs, dites avant d'être crues.

**Le type de contrainte d'index n'est pas d'eux.** Le document 01 §5 présente
comme un signe de leur généralisation :

```cpp
enum class IndexConstraintType : uint8_t { PRIMARY = 0, SECONDARY_NON_UNIQUE = 1 };
```

Il existe **déjà en amont**, à `89f0263cc`, mot pour mot. Ils n'ont fait que
retirer la macro d'export. Le reste du §5 tient : le compte de virtuelles passe
bien de 19 à 25, vérifié.

**La recette de normalisation du document 04 est fautive.** Elle prescrit
`sed 's/\blbug\b/kuzu/g'`. En expression régulière, le tiret bas est un caractère
de mot : `\b` ne coupe donc pas entre `rag3db` et `_`, et tous les identifiants
de l'API C (`rag3db_connection_execute`, `lbug_connection_execute`) échappent au
remplacement. L'effet n'est pas anodin — il **surestime l'écart d'un facteur
deux** en nombre de fichiers. Un fichier de test de l'API C ressortait à 2 906
lignes d'écart alors qu'il est identique à l'amont.

La recette correcte est un remplacement **sans frontière de mot**, avec les
variantes de casse :

```sh
# chez eux
sed 's/LADYBUG/KUZU/g; s/Ladybug/Kuzu/g; s/ladybug/kuzu/g; s/LBUG/KUZU/g; s/lbug/kuzu/g'
# chez nous
sed 's/RAG3DB/KUZU/g; s/Rag3db/Kuzu/g; s/rag3db/kuzu/g'
```

Avec elle, les chiffres du document 01 se reproduisent exactement : 1 637
fichiers touchés dans `src/`, dont 1 608 de pur renommage, **29 réellement à
nous, dont 5 entièrement neufs**. Le repérage était juste ; c'est la recette
publiée qui ne l'était pas.

## Le nettoyage, fait

Les deux fichiers morts du §5 du document 02 sont supprimés, 64 lignes :

```
src/include/processor/operator/scan/fts_scan_node_table.h   60 lignes
src/include/common/fts_types.h                               4 lignes
```

La preuve qu'ils sont morts est plus forte que « personne ne les inclut » :
`fts_scan_node_table.h` référence `PhysicalOperatorType::FTS_SCAN_NODE_TABLE`, une
valeur d'énumération **qui n'existe nulle part dans l'arbre**. Ce fichier ne
compilerait pas si on l'incluait.

Vérification de non-régression, sans recompiler : sur les **2 076 fichiers de
dépendance** des builds existants, **aucun** ne référence ces deux en-têtes,
tandis que l'en-tête vivant `index_search_types.h` apparaît dans 64. Aucune unité
de compilation n'est affectée.

## Deux choses trouvées en chemin, à ne pas perdre

**Ils ont travaillé sur le verrou de fichier.** Notre document 01 §8 note qu'on a
heurté `F_WRLCK`, deux processus sur une même base. Dans leur
`local_file_system.cpp`, l'échec de verrou interroge maintenant `F_GETLK` et
nomme le processus qui tient le verrou :

```cpp
"Could not set lock on file : " + fullPath +
" (Lock is held by PID " + std::to_string(get_fl.l_pid) + ")"
```

Ça ne lève pas le verrou, mais ça rend l'erreur diagnosticable. C'est petit, et
c'est directement dans notre douleur.

**Le renommage reste à questionner.** Il coûte 2 899 fichiers à chaque reprise de
l'amont. Le document 02 §8 le signalait ; la sonde le confirme en chiffre. Ce
n'est pas la question de cette mission.

## Ce que la sonde n'a pas fait

- **Rien n'a été compilé de LadybugDB.** La branche A a suffi, comme prévu. Le
  build de Ladybug reste non tenté sur ce poste.
- **La branche B n'a pas lieu d'être** : elle ne s'ouvrait que si la descente
  était atteignable. Elle ne l'est pas.
- **Le contenu de leurs dépôts d'extensions n'a pas été lu.** Il faudrait les
  cloner. C'est le repérage suivant, et c'est lui qui décide du rebasage.
- **`Vela-Engineering/kuzu`** reste non évalué.
