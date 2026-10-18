# Des scopes sans `DEFINED_IN` dans une réindexation à neuf : 3 825 sur 82 808 dans code-lourd

- **État** : ouvert (18 octobre 2026, codeparsers, à la demande de l'orchestration). Structurel,
  à regarder : vu en mesurant les pertes du COPY de liens, sans lien avec elles.
- **Gravité** : réponse incomplète. Un scope sans `DEFINED_IN` ne se rattache pas à son fichier par
  cette arête ; les outils qui passent par elle (la fin d'un fichier, ce qu'un fichier définit) ne
  le voient pas. Rien n'est perdu au stockage : le scope garde son `file_path`.
- **Atteignable en service** : oui, à chaque index.
- **Touche rag3weaver** : oui (l'ingestion du code ; peut-être l'analyse de codeparsers).

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
