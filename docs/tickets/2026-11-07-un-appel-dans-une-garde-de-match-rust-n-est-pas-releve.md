# Un appel dans une garde de `match` Rust n'est pas relevé

- **État** : ouvert (7 novembre 2026), pris par codeparsers (cause trouvée, correctif en cours).
- **Gravité** : réponse fausse (`usages` et `impact` rendent 0 pour une fonction appelée).
- **Atteignable en service** : oui, tout dépôt Rust qui appelle une fonction dans une garde
  `match … { motif if appel(…) => … }`.
- **Touche rag3weaver** : par codeparsers (le relevé des références Rust).
- **Famille** : produit

## Ce que c'est

Les identifiants d'une garde de `match` sont pris pour des liaisons du motif et ôtés des
références : l'appel qu'ils portent n'existe pas pour l'index.

## Les cas (base de code de Lucie, index de b720282fb, 7 novembre 2026)

- `vue_portee` (`extension/rag3weaver/src/ide.rs:409`) n'est appelée qu'à `ide.rs:498`,
  `("GET", p) if vue_portee(p) => {` : `usages vue_portee` rend **0 usage**, `impact` 0.
- `parametres` (`ide.rs:358`), appelée normalement à `ide.rs:447` dans le même fichier : 2 usages.

Trouvé par G en rendant la vue du symbole de l'IDE sur l'hôte vivant (7970) : la vue n'avait
aucune section d'usages pour une fonction pourtant appelée.

## La cause (codeparsers)

`src/scope_extraction/rust_scope_extraction_parser.rs:1702`, `collect_pattern_bindings` porte la
règle `("match_pattern", _) => true` : tous les enfants d'un `match_pattern` y sont des motifs, y
compris son champ `condition`, qui est la garde `if …`. Les identifiants de la garde (`vue_portee`,
`p`) sont collectés comme des liaisons locales.

## Ce qu'il faut pour le fermer

- Le correctif : `("match_pattern", Some("condition")) => false`, avant la règle générale.
- Le témoin, dans codeparsers, rouge d'abord, sur quatre formes : une garde simple, une garde
  avec `&&`, `if let P = v` (déjà juste : seul le `pattern` de `let_condition` est un motif),
  `if let … && cond` et `while let`.
- Le lot rag3db qui avance le pointeur de codeparsers, avec la sonde sur code-lourd avant et
  après (combien de CONSUMES Rust reviennent, et qu'aucune locale de motif ne devient une
  référence) ; il ferme ce ticket.
