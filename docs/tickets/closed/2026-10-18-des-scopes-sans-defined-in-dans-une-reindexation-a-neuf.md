# Des scopes sans `DEFINED_IN` dans une réindexation à neuf : 3 825 sur 82 808 dans code-lourd

- **État** : corrigé (15 novembre 2026) — codeparsers d024118 (`defined-in-homonymes`) : un uuid par scope, même entre homonymes ; témoin `tests/defined_in_homonymes.rs`, rouge sur le code d'avant
- **Gravité** : réponse incomplète. Un scope sans `DEFINED_IN` ne se rattache pas à son fichier par
  cette arête ; les outils qui passent par elle (la fin d'un fichier, ce qu'un fichier définit) ne
  le voient pas. Rien n'est perdu au stockage : le scope garde son `file_path`.
- **Atteignable en service** : oui, à chaque index.
- **Touche rag3weaver** : oui (l'ingestion du code ; peut-être l'analyse de codeparsers).
- **Famille** : produit

## Ce que c'est

Sur `code-lourd` (arbre `rag3db-lourd` @ `a72acfeef`), la réindexation à neuf au seuil de COPY de
liens haut, celle au seuil par défaut et la base vive comptent toutes les trois **3 825 scopes sans
`DEFINED_IN`** sur 82 808. Les autres bases de code en ont autant : 3 849 et 3 629 pour
`code-principal` (vive et réindexée à neuf), 4 206 pour `code-rag3db-depot`. Un morceau sans
`*_CHUNKED_FROM`, lui, ne se trouve nulle part.

Par genre (`scope_type`, réindexation de référence de `code-lourd`) : function 2 341, method 985,
class 397, namespace 47, lambda 36, enum 17, variable 2.

## Exemples (clé, fichier, ligne)

- `extension/geo/test/geo_test.cpp#TEST:function#7` … `#14` (les macros `TEST` de gtest, une par
  test du fichier) ; `extension/geo/test/geo_math_test.cpp#TEST:function`.
- `extension/lucivy/ld-lucivy/columnar/src/value.rs#Coerce:class#1`, `Coerce.coerce:method#1` et
  `#2` (plusieurs `impl Coerce for …` dans le même fichier).
- `extension/lucivy/ld-lucivy/bindings/nodejs/src/lib.rs#Index:class#1`.
- `experiments/mtga/scripts/…#list_comprehension:lambda#1`, `#generator_expression:lambda#1`.

Le motif qui saute aux yeux, sans preuve : des scopes **de même nom dans un même fichier**
(clés à suffixe `#N`, ou leur premier de série). Le lien `DEFINED_IN` se pose peut-être par une clé
qui ne tient pas compte du suffixe.

## Revu par le banc, le 15 novembre 2026

Vu en jugeant l'écrivain des générations (everything declarative) : sa sonde de structure, « les
Scope visibles sans `DEFINED_IN` vers un File visible », ne valait pas 0, et valait autant au
parent qu'au lot. Ce n'est pas l'écrivain : c'est ce ticket.

| base (copie de `code-rag3db-depot`) | Scope | sans `DEFINED_IN` | avec une ligne File au même chemin |
|---|---|---|---|
| R, d'avant les versions de scope | 79 662 | **3 931** | 3 931 |
| migrée par 1b, puis réindexée sur 835d5298f (le parent e95a1b8c60) | 77 154 visibles | **3 814** | 3 814 |
| la même, réindexée par l'écrivain (d087e02c7) | 77 154 visibles | **3 814** | 3 814 |

- **Le défaut est antérieur à la migration 1b** : la base R l'a déjà. Il vient de l'ingestion, et
  l'écrivain n'y change rien.
- **Aucun de ces Scope n'a d'arête `DEFINED_IN`**, ni vers un File ouvert ni vers un fermé. Leur
  fichier, lui, est bien là : une ligne File au même chemin pour chacun.
- **Par genre**, sur R : function 2 489, method 966, class 336, namespace 50, variable 39, et
  deux genres de plus que l'affichage a coupés. Sur la base réindexée : function 2 489,
  method 865, class 334, namespace 50, lambda 34, et deux de plus.
- **Le motif, avec un cas précis** : dans `extension/json/src/include/json_utils.h`, les
  constructeurs de `JsonMutWrapper` (clés `…#JsonMutWrapper.JsonMutWrapper:method`, `#1`, `#2`, `#3`).
  Seul `#3`, le dernier de la série, a son `DEFINED_IN` ; la classe aussi. Les trois premiers ne
  l'ont pas. Ça appuie l'hypothèse d'une clé qui ignore le suffixe `#N` au moment de poser le
  lien : le dernier de même nom l'emporterait.
- **D'autres exemples sur la base réindexée** : `extension/geo/test/geo_test.cpp` `TEST`
  (l. 171 et 193), `extension/duckdb/src/connector/duckdb_result_converter.cpp`
  `convertDuckDBVectorToVector` (l. 12), `experiments/mtga/scripts/research_ultimatum_colors.py`
  `list_comprehension` (l. 25).
- **La recette** : `json_utils.h` seul, indexé, et ses quatre constructeurs devraient avoir chacun
  un `DEFINED_IN`.
- **La preuve** : la sonde Cypher `releve_des_lignes_par_cypher`
  (`extension/rag3weaver/tests/sonde_parite_search_code.rs`, branche `banc-parite-search-code`),
  et ses relevés à luciepc, dans `banc-parite/sorties/` : `sans-defined-in-r.json` et
  `ecrivain-propre/qui-3814.json`.

## Recette

À écrire : un fichier de deux `impl Coerce for …` (Rust) ou de deux `TEST(...)` (C++), indexé seul,
puis `MATCH (s:Scope) WHERE NOT EXISTS { MATCH (s)-[:DEFINED_IN]->() } RETURN s.key`.

## Témoin

La sonde `extension/rag3weaver/tests/sonde_aretes_par_relation.rs` (`compter_les_aretes`, sur une
copie de base) rend la ligne `sans_lien  Scope  DEFINED_IN  <n>`, le compte par genre et des
exemples. Un témoin sur le fichier de la recette reste à écrire.

## Pour le fermer

Trouver où `DEFINED_IN` se pose et pourquoi ces scopes y échappent ; le témoin de la recette à 0 ;
la sonde à 0 sur une réindexation à neuf de `code-lourd`.

## La correction

**La cause**, lue avec la recette du banc (les constructeurs de `JsonMutWrapper` dans `json_utils.h`) : le
parseur C++ reconstruit la même signature pour des surcharges (« void JsonMutWrapper() » pour les quatre), et
l'uuid d'un scope en dépend (`blake3(fichier : nom : genre : hash(parent + signature))`,
`relationship_resolver.rs`). Quatre homonymes, un seul uuid : 2 uuids distincts pour 5 scopes. Chez
rag3weaver, la table uuid → clé (`code.rs`, `identity`) gardait le dernier, et les arêtes des trois autres se
rabattaient sur lui — `#3` seul avec son `DEFINED_IN`, comme l'a vu le banc. Le même mécanisme valait pour les
autres exemples (deux `impl Coerce for …`, des `TEST` de gtest, des lambdas de compréhension) et pour **toutes**
les arêtes d'un homonyme, pas seulement `DEFINED_IN`.

**Le correctif** : `build_global_scope_mapping` calcule l'uuid de chaque scope une fois, par sa position ; le
premier de mêmes entrées garde son uuid d'aujourd'hui, les suivants ajoutent leur rang d'apparition. Les clés
des scopes de rag3weaver ne changent pas (elles ont leur propre règle, `stable_scope_keys`) : seules les arêtes
manquantes s'ajoutent.

**Le compte**, par l'analyse de rag3weaver sur `extension/rag3weaver/src` (290 fichiers) : scopes sans
`DEFINED_IN` **65 → 0**, pour 16 222 scopes, en nombre inchangé. Sur les bases (`code-rag3db-depot`, 3 814
avant, et `code-lourd`), le banc mesure après la pose, par `releve_des_lignes_par_cypher` : la réponse
s'ajoutera ici. (L'analyse de l'arbre entier en mémoire crevait l'enveloppe de 32 Gio : ticket fermé du
15 novembre, « une passe confinée reçoit un SIGTERM sans cause dite ».)

La signature fausse des surcharges, elle, avait sa cause propre (les paramètres par pointeur ou par
référence perdus) : [ticket du même jour](2026-11-15-un-parametre-cpp-par-pointeur-ou-reference-perd-son-nom.md).
