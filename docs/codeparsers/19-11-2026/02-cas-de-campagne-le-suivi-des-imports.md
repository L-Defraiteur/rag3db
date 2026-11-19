```json
{"parent": "ed4c4a913677fad7b622ab8763942ad929ea42e0", "tree": ".", "needs_lib": false, "witness": "tracking_the_imports_of_many_scopes_stays_cheap_and_exact", "test": "cargo test --test import_tracking_cost", "suites": ["cargo test"], "hints": ["tests/import_tracking_cost.rs", "src/scope_extraction/base_scope_extraction_parser.rs"], "ours": "dcec0a560a038ef3f921ffce2516fcff14a4f3b7"}
```

# Cas de campagne : le suivi des imports compile une expression régulière par import

Session codeparsers, 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie ; lanceur d'archi,
juge du banc). Un cas court, en Rust, dans le crate codeparsers seul.

## Le point de départ

- **Dépôt** : codeparsers, le sous-module de rag3weaver, pas rag3db. `parent` est un commit de ce dépôt.
- **Commit parent** : `ed4c4a913677` (19 novembre 2026). Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : `.`, le crate codeparsers entier, sans la lib rag3db.

## Le signalement, tel qu'il était

> Un profil par échantillonnage de l'analyse de rag3weaver/src montre qu'une grande part du temps part à construire
> des expressions régulières, sous `ensure_import_references_tracked` du parseur de base : pour chaque scope et
> chaque import externe pas encore relevé dans ce scope, elle compile `\bnom\b` puis cherche le nom comme mot entier
> dans le code du scope. Avec beaucoup d'imports et beaucoup de scopes, l'analyse d'un seul fichier prend des
> secondes. Le suivi doit rester exact (un import utilisé est relevé comme référence, un import inutilisé ne l'est
> pas) et ne plus coûter une compilation par scope et par import.

## Le témoin

- **Filtre** : `tracking_the_imports_of_many_scopes_stays_cheap_and_exact`, dans `tests/import_tracking_cost.rs`.
- **Commande** : `cargo test --test import_tracking_cost`, depuis la racine du crate (profil de test, sans
  `--release`).
- **Rouge au parent** : 300 imports externes (`use serde::NameN;`) sur 300 fonctions, l'analyse prend 11,97 s,
  au-delà de la borne d'une seconde. Les deux vérifications d'exactitude (l'import utilisé `Name7` relevé dans `f42`,
  rien dans `f41`) passent déjà.
- **Identique sur master** dès l'atterrissage du lot `parent-cache` (déposé au pousseur le 19 novembre), qui porte ce
  correctif en dessous.
- **Suites exigées** : `cargo test` du crate entier (73 lots au 19 novembre).

## Notre correctif (caché à Algareth, pour le juge)

`dcec0a560a03`, « perf(imports): suivre les imports d'un scope sans compiler une expression régulière par import —
find_whole_word, la même frontière que \b » : `src/scope_extraction/base_scope_extraction_parser.rs`, +27 −5.
Vert : 0,54 s.
