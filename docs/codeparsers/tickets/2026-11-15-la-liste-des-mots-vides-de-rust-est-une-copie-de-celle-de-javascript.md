# La liste des mots vides de Rust est une copie de celle de JavaScript

- **État** : ouvert (15 novembre 2026)
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
