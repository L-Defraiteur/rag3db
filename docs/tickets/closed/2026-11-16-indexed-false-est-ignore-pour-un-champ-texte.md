# `indexed: false` est ignoré pour un champ texte

- **État** : corrigé (18 novembre 2026), dans lucivy main e3c9753 (PR #44), à publier avec la 4.3.10. Lot écrit par
  Codex le 16 novembre (`schema_warnings`, l'avertissement, la doc, trois témoins), qualifié le 18 par la session
  lucivy : `-p lucivy-core` vert sur le corpus rag3db et linux v7.2. Honorer `indexed: false` reste une décision de
  Lucie, hors de ce ticket.
- **Gravité** : faible, mais silencieuse. Un appelant qui déclare un champ texte `indexed: false`, pour le stocker
  sans le chercher, paie quand même son index entier (dictionnaire, postings, positions). Et rien ne le lui dit.
- **Atteignable en service** : seulement si un manifeste ou un appelant pose `indexed: false` sur un champ `text`.
  Aucun manifeste de rag3weaver ne le fait aujourd'hui.
- **Touche rag3weaver** : non.
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

`lucivy_core/src/handle.rs`, `build_schema`, branche `"text" =>` : le champ reçoit toujours
`TextFieldIndexing` (tokenizer `RAW_TOKENIZER`, `WithFreqs` en v3). Seul `stored` est lu, jamais `indexed`. Les
types `u64`, `i64` et `f64` lisent bien `indexed`. La branche `"string"` l'ignore aussi : elle pose toujours
`STRING`, donc un champ indexé. L'avertissement et la doc valent pour les deux.

## Ce qu'il faut faire (ce lot-ci)

- Un **avertissement** à la création, une fois par champ concerné, quand un champ `text` ou `string` porte
  `indexed: Some(false)` : `eprintln!("Warning: …")`, la forme que `handle.rs` emploie déjà (l. 231) ; ne pas ajouter de dépendance. En
  anglais, par exemple `field "<name>": indexed=false is not supported for text and string fields; the field is indexed`.
- **La doc** : la doc du champ `indexed` (`lucivy_core/src/query.rs:93`) et la section des champs de `lucivy_core/README.md` disent que `indexed: false` n'a pas d'effet sur un
  champ texte.
- **Ne pas** honorer `indexed: false` dans ce lot. Un champ texte stocké sans index change ce que les requêtes
  `contains` et les spans peuvent faire sur lui. C'est un choix de Lucie, à lui présenter à part, avec ce que ça
  coûte et ce que ça casse.

## Comment le prouver

- Rouge d'abord, dans un test neuf `lucivy_core/tests/test_text_field_indexed_false.rs` : créer un index avec un
  champ `text` à `indexed: false`, et exiger que la création le signale. Le moyen le plus simple de l'observer : une
  fonction pure qui rend les avertissements du schéma (`schema_warnings(&SchemaConfig) -> Vec<String>`), appelée
  par `build_schema` et testée directement. Rouge tant qu'elle n'existe pas ou rend une liste vide.
- Vert ensuite, puis `cargo test --release -p lucivy-core` en entier.
