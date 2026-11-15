# fmt/format.h : depuis tree-sitter 0.27, une macro est lue comme un corps de fonction

- **État** : corrigé (15 novembre 2026) — codeparsers 5f7c837 (`macro-format-h-2`) : une `function_definition` sans `function_declarator` n'est plus un scope ; témoin `tests/macro_format_h.rs` sur le vrai fichier, rouge sur le code d'avant
- **Gravité** : réponse fausse (parents et noms de scopes faux dans un fichier tiers).
- **Atteignable en service** : oui, sur toute base qui indexe `third_party/spdlog`.
- **Touche rag3weaver** : oui, par codeparsers (l'extraction des scopes C++).
- **Famille** : produit

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

## La correction

**La cause**, lue dans l'arbre de tree-sitter-cpp 0.23.4 : `FMT_BEGIN_NAMESPACE namespace detail { … }`
arrive en `function_definition` — la macro pour type, `detail` pour déclarateur (un `identifier`), le corps
du namespace pour corps — **sans `function_declarator`**, ce qu'aucune vraie fonction n'est. codeparsers
n'y trouvait pas de nom et en faisait un `AnonymousFunction`, parent de tout ce qu'il contenait. Quatre cas
dans le fichier : `FMT_BEGIN_NAMESPACE` trois fois (l. 83, 173, 242), `FMT_CLASS_API` une fois (l. 956).

**Le correctif** : une telle définition n'est plus un scope ; son contenu prend pour parent le namespace
écrit dans sa tête s'il y en a un, sinon le parent d'au-dessus.

**Le compte de `format.h`**, avant → après : 455 → 481 scopes ; 4 → 0 `AnonymousFunction` ; 45 → 0 scopes
sous l'un d'eux ; `do_throw` et `abort_fuzzing_if` sous `detail`. Les 4 faux scopes partent, 30 scopes que
leur corps cachait sont relevés.

**La limite** : sous `FMT_CLASS_API`, la grammaire lit `class FMT_API format_error : …` comme un nom qualifié
fautif ; les membres de la classe sont relevés, la classe n'a pas de scope. C'est la grammaire, pas
codeparsers : [ticket du même jour](../../codeparsers/tickets/2026-11-15-une-classe-derriere-une-macro-d-export-n-a-pas-de-scope.md).
