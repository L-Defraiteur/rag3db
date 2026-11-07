# Un appel dans une garde de `match` Rust n'est pas relevé

- **État** : corrigé — codeparsers 672a0d5 (garde-de-match), le pointeur monté dans le lot qui ferme ce
  ticket (7 novembre 2026).
- **Gravité** : réponse fausse (`usages` et `impact` rendent 0 pour une fonction appelée).
- **Atteignable en service** : oui, tout dépôt Rust qui appelle une fonction dans une garde
  `match … { motif if appel(…) => … }`.
- **Touche rag3weaver** : par codeparsers (le relevé des références Rust).

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

## Comment il a été corrigé (codeparsers, branche garde-de-match)

- **Deux endroits, pas un.** Le premier correctif (581d6aa, `rust_scope_extraction_parser.rs` :
  `("match_pattern", Some("condition")) => false`) ne suffisait pas : `collect_local_symbols` de la
  base prend le champ `pattern` d'un `match_arm`, qui est le `match_pattern` entier, garde comprise,
  et `collect_pattern_bindings` y descendait dans tous les enfants nommés. Le second (672a0d5) n'y
  parcourt plus le champ `condition`. Le témoin rouge l'a montré après le seul premier.
- **Le témoin** (`tests/garde_de_match_rust.rs`) : une garde simple et une garde avec `&&` (rouges
  sur 7c5ff80 : la fonction dont les seuls appels sont dans ses gardes n'avait aucune référence),
  `if let`, `while let` et une fermeture passée en argument (déjà justes, figés). Suite entière de
  codeparsers : 43 binaires, 263 passés, 0 échoué.

## La sonde sur code-lourd (7 novembre 2026, luciepc)

Deux réindexations sur le même master (2721843b5, lib 04063ee06), codeparsers 7c5ff80 puis 672a0d5 :
**19 CONSUMES Rust de plus, 0 de moins** (81 219 → 81 238 ; marques : fichier 12, nom 5, import 2).
rust-analyzer, juge strict, sur les 19 :

- 9 justes, dont 8 appels de garde que l'index ne voyait pas (`is_checkpoint_crossing`,
  `indexing_in_progress`, `est_un_noeud_reactif`, `is_permanent_quota`, `answers`…) ;
- 2 « décl » qui n'en sont pas : `let Some(hash) = self.hash(&path)` — l'arête vise bien la méthode,
  l'oracle interroge le premier `hash` de la ligne, la liaison ;
- 5 fausses, toutes « nom » : une méthode d'un type de la bibliothèque standard lue dans une garde
  (`reste.is_zero()`, `results.split_off(…)`) ou une variante d'enum (`CypherValue::Float`), reliée par
  le seul nom à un homonyme de lucivy — la faiblesse connue de la marque « nom », que le relevé rend
  visible, pas un défaut de ce correctif : le ticket
  [la marque « nom » relie un appel de méthode standard à un homonyme](../2026-11-07-la-marque-nom-relie-un-appel-de-methode-standard-a-un-homonyme.md) les porte ;
- 3 muettes (deux dans le code de lucivy, une ligne de mention fausse).

Aucune locale de motif n'est devenue une référence ; aucune arête n'a été ôtée.
