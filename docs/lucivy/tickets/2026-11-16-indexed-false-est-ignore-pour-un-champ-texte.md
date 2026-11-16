# `indexed: false` est ignoré pour un champ texte

- **État** : ouvert (16 novembre 2026). Noté le 14 novembre au ticket du poids du plein texte, jamais ouvert à part.
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


## Relecture Banc du paquet consommateur a039e4bbce

[Rapport indépendant](../../banc/16-11-2026/13-ro-paquet-lucivy-admission-rouge.md) : ordre des ledgers et budget unique cohérents, deux réserves groupées avant GO du paquet99a8/7649 : réattestation finale des trois nouveaux ELF et source/copies contre les pins ; contrôle d’échéance après ouverture et avant publication du résultat attendu. Porteur Lucivy, critères de fin : nouveau paquet épinglé/relu, mêmes bornes et seul témoin mock, aucune confusion avec correction/couverture du core. Aucun run par Banc ; ticket toujours ouvert.


Delta44a7849a9c relu par Banc : pilote7833/admit7649/preflight0c903, les deux réserves7f6 sont levées pour ce paquetv2 exact. Rehash physique final3ELF/source/copies/préflight et gardes de publication aprèsopen/avantdump/aprèsfsync présents. Voir complément du rapport13. Avis préparatoire favorable, pasGO600 ; aucun mock Rust ou builder natif joué par Banc, ticket global toujours ouvert.


RO preuves9a64654e7f par Banc : [rapport15](../../banc/16-11-2026/15-ro-preuves-rouge-lucivy-admission.md),35SHA conformes, unique rouge mock0/1/0/8filtered,left1/right0, trois identités initial/final/externe concordantes et fin Common101 propre sousEND. Favorable à ce rouge discriminant seulement ; aucun ELF ouvert ou exécuté par Banc, aucun correctif ou vert core déduit. Ticket reste ouvert pour la suite distincte.
