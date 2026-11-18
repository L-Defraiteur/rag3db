# La liste des mots vides de Rust est une copie de celle de JavaScript

- **État** : corrigé (18 novembre 2026) — Rust (branche `codeparsers/rust-stop-words-4`), Python et C (branche `codeparsers/stop-words-python-c`) ; C++, C# et Go dans le ticket `codeparsers/tickets/2026-11-18-les-mots-vides-de-c-plus-plus-c-sharp-et-go-copient-javascript.md`
- **Famille** : produit
- **Gravité** : basse. Rien n'est faux aujourd'hui : les mots de JavaScript repris dans la liste Rust
  (`function`, `class`, `this`, `undefined`, `typeof`, `instanceof`, `var`…) ne nomment presque jamais
  rien en Rust. Le jour où un dépôt Rust nomme un scope `function` ou `class` (un champ, une méthode), ses
  références ne seront pas relevées.
- **Atteignable en service** : rarement — un nom Rust qui serait un mot-clé de JavaScript
- **Touche rag3weaver** : oui, par codeparsers (les références des scopes Rust)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, au correctif de `new` (`RUST_STOP_WORDS`,
  `rust_scope_extraction_parser.rs:26`), avec l'orchestration

## Ce qu'il faudrait

Une liste de mots vides propre à Rust : ses mots-clés (`fn`, `let`, `mut`, `impl`, `match`…) et rien de
JavaScript. Le même relevé pour Python et C, dont les listes reprennent aussi celle de JavaScript. Un
témoin par langue : un scope nommé d'un mot JavaScript est référencé.

## Rust, 18 novembre 2026

La liste de Rust est tirée de la documentation de std, en données (`data/stop_words/rust.toml`, sa recette
`generer.py`, lue par `standard::stop_words_of`) ; plus aucun mot de JavaScript. Témoin :
`a_rust_call_named_like_a_javascript_keyword_is_a_reference` (`Settings::default()` est une référence).
Sur rag3weaver/src, 287 arêtes gagnées : `default` 221, `from` 41, `of` 10, `channel` 3 (`hub.channel(…)`)…
Une perte de masque : `var` n'est plus un mot vide, et neuf noms de champ d'un littéral de structure
(`ParCle { var: "n", … }`) se relient à la fonction `var` du projet : ticket
`codeparsers/tickets/2026-11-18-un-nom-de-champ-d-un-litteral-de-structure-rust-se-relie-a-une-fonction.md`.
Python et C gardent la copie de JavaScript : ce ticket reste ouvert pour eux.

## Python et C, 18 novembre 2026

Les deux listes viennent du langage, en données, par le même générateur (`data/stop_words/generate.py`) :
Python, les 35 mots-clés durs de `keyword.kwlist` (CPython 3.14.10 ; les mots-clés doux `match`, `case`,
`type`, `_` sont des identifiants légaux et n'y sont pas) ; C, les 103 mots que réserve la grammaire
`tree-sitter-c` 0.24.2 (ses jetons alphabétiques anonymes, `primitive_type`, `true`, `false`, `null`).
Témoins (codeparsers, `tests/python_c_stop_words.rs`) : un appel à `default()`, `of()`, `this()` et
`C().function()` en Python, à `from()`, `of()`, `function()` et `this()` en C, est une référence ; rouges
avant (seul le nom témoin `helper` était relevé), verts après.

Sonde de marque, avant = codeparsers 34c74e4, après = 72ab7a8 : les 321 fichiers Python de l'arbre figé
gagnent 3 arêtes `fichier`, toutes justes (deux variables `case` et une variable `export` du même fichier) et
n'en perdent aucune ; le corpus C (brotli, zstd, mbedtls, 324 fichiers, et 258 en-têtes `include/linux`) est
identique au couple près. L'effet est petit parce que les noms de JavaScript sont rares en Python et en C ; le
gain est d'avoir la liste du langage, pas celle d'un autre.

