```json
{"parent": "579a3f3ca90eae6c66149356ca5fe7a8ce6bbd95", "tree": ".", "needs_lib": false, "witness": "the_extraction_does_not_walk_down_the_tree_for_each_identifier", "test": "cargo test --test parent_calls", "suites": ["cargo test"], "hints": ["src/scope_extraction/parents.rs", "tests/parent_calls.rs", "tests/parent_cost.rs"], "ours": "828f855d00d832b51884d0b075c8e7caaf7f4c4a"}
```

# Cas de campagne : l'extraction redescend l'arbre pour chaque identifiant

Session codeparsers, 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie ; lanceur d'archi,
juge du banc). Un cas court, en Rust, dans le crate codeparsers seul, qui touche les sept langages du routeur.

## Le point de départ

- **Dépôt** : codeparsers, le sous-module de rag3weaver, pas rag3db. `parent` est un commit de ce dépôt.
- **Commit parent** : `579a3f3ca90e` (19 novembre 2026). Le témoin y est, rouge ; le correctif n'y est pas. Tous les
  parents de l'extraction passent déjà par `parents::parent_of`, qui compte ce qu'il demande à tree-sitter.
- **Arbre** : `.`, le crate codeparsers entier, sans la lib rag3db.

## Le signalement, tel qu'il était

> Un profil en release de l'analyse (rag3weaver/src, le moteur C++, un corpus Python et JavaScript) montre que
> `Node::parent` de tree-sitter pèse environ la moitié du temps. Ce n'est pas un coût quadratique : tree-sitter ne
> garde pas les parents, et `parent()` redescend depuis la racine du fichier à chaque appel. L'extraction commune le
> demande pour chaque identifiant (`usage_of`, `is_definition_identifier`, et d'autres), dans tous les langages.
> L'extraction d'un fichier ne doit plus redescendre l'arbre pour chaque identifiant, et chaque nœud doit garder
> exactement le parent que l'arbre lui donne. Le correctif reste dans codeparsers, sans toucher à tree-sitter.

## Le témoin

- **Filtre** : `the_extraction_does_not_walk_down_the_tree_for_each_identifier`, dans `tests/parent_calls.rs`.
- **Commande** : `cargo test --test parent_calls`, depuis la racine du crate.
- **Rouge au parent** : pour un fichier de 200 appels par langage, l'extraction redescend l'arbre de 2 810 à 7 608 fois
  selon le langage (Rust, C, C++, Python, TypeScript, Go, C#), au-delà de la borne de dix. L'appel à `g` reste relevé
  dans tous. C'est un compte, pas un chronomètre : le témoin est déterministe.
- **Identique sur master** dès l'atterrissage du lot `parent-cache` (déposé au pousseur le 19 novembre).
- **Suites exigées** : `cargo test` du crate entier (73 lots au 19 novembre). `tests/parent_cost.rs` (une fonction de
  20 000 instructions par langage, sous trois secondes) y est une garde, verte avant comme après.

## Notre correctif (caché à Algareth, pour le juge)

`0bfe5bb4c374` puis `828f855d00d8`, « perf(parents): le parent d'un nœud lu dans un index bâti une fois par arbre » et
« la racine de l'arbre indexé répond sans demander l'arbre » : un index des parents bâti en un parcours par
`TreeCursor` au début du parsing de chaque fichier, sous une garde, dans les sept parseurs ;
`src/scope_extraction/parents.rs` et les sept parseurs, 8 fichiers, +155 −8 au total. Vert : 0 descente dans les sept
langages ; l'analyse en release −40 % sur les trois corpus.
