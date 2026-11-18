# La liste des mots vides de Rust est une copie de celle de JavaScript

- **État** : ouvert (15 novembre 2026) — Rust corrigé le 18 novembre (branche `codeparsers/rust-stop-words-4`) ; restent Python et C
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

