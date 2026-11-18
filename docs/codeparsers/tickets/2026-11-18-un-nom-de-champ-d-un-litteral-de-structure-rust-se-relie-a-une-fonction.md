# Un nom de champ d'un littéral de structure Rust se relie à une fonction

- **État** : ouvert (18 novembre 2026) — trouvé par la sonde du lot des mots vides de Rust ; non corrigé
- **Famille** : produit
- **Gravité** : réponse fausse, rare : une arête « nom » vers une fonction homonyme d'un champ
- **Atteignable en service** : oui, quand un champ initialisé dans un littéral porte le nom d'une fonction du projet
- **Touche rag3weaver** : oui, par codeparsers (les références des scopes Rust)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, sonde de marque de la repose (5), 18 novembre 2026

## Constat

Dans `ParCle { var: "n", label: …, prop: …, champ: … }` (`src/dialect.rs:843`, `:2239`, `:2256`,
`src/cypher_blob_store.rs:337`) et `MermaidError::UnknownVariable { line, var: racine.to_string() }`
(`src/dataflow/mermaid.rs:286`, `:350`, `:704`), le nom du champ `var` est relevé comme une référence et se
relie par le nom à `fn var` de `src/dataflow/clic_tsx.rs:75`. Neuf arêtes fausses sur rag3weaver/src.
Elles étaient masquées jusqu'ici parce que `var` était un mot vide (la liste de JavaScript) ; le lot des
mots vides de Rust les rend visibles. Le `var(&e.from_node)` de `clic_tsx.rs:134`, un vrai appel, est juste.

## Ce qu'il faudrait

Le `field_identifier` du côté nom d'un `field_initializer` n'est pas une référence à une fonction (c'est un
membre du type du littéral) ; le forme raccourcie `{ var }` reste une référence à la locale. Témoin rouge
d'abord : `struct P { var: u8 } fn var() -> u8 { 0 } fn f() -> P { P { var: 1 } }` ne relie pas `f` à `var`.
