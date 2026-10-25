# fmt/format.h : depuis tree-sitter 0.27, une macro est lue comme un corps de fonction

- **État** : ouvert — recul accepté par Lucie (25 octobre 2026) avec le passage à tree-sitter
  0.27, contre ses gains ; aucun correctif.
- **Gravité** : réponse fausse (parents et noms de scopes faux dans un fichier tiers).
- **Atteignable en service** : oui, sur toute base qui indexe `third_party/spdlog`.
- **Touche rag3weaver** : oui, par codeparsers (l'extraction des scopes C++).

## Ce que c'est

Avec tree-sitter 0.27 et tree-sitter-cpp 0.23.4, une macro de fmt est lue comme le corps d'une
fonction : dans `third_party/spdlog/spdlog/fmt/bundled/format.h`, 39 fonctions passent sous un
parent `AnonymousFunction` (abort_fuzzing_if, assume, base_iterator, bit_cast…), et la classe
`FMT_COMPILE_STRING` disparaît. Le code de rag3db (`src/`) n'est pas touché : 0 scope changé.

## La recette

La sonde sur code-lourd, réindexé par le commit du passage à 0.27, comparée scope par scope à la
base du même master en 0.24 (`tests/sonde_aretes_par_relation.rs`, `SONDE_LISTER`) : les scopes
de `format.h` dont `parent_name` est un `AnonymousFunction` en 0.27 et un espace de noms en 0.24.

## Le témoin

Aucun pour l'instant. Il faudrait un test de codeparsers sur un extrait de `format.h` (la macro
qui ouvre la région, puis une fonction libre) qui exige le parent de la fonction.

## La cause

Pas encore lue. La grammaire C++ récente analyse une invocation de macro suivie d'un bloc comme
une définition de fonction ; codeparsers en fait un scope anonyme, parent de ce qui suit.

## Ce qu'il faut pour le fermer

Écrire le témoin, puis soit reconnaître ce motif dans codeparsers (une « fonction » sans nom dont
le déclarateur est une macro ne devient pas parent), soit le signaler à la grammaire et attendre.
