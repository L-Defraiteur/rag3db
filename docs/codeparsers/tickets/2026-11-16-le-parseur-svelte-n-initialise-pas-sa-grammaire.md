# Le parseur Svelte n'initialise pas sa grammaire

- **État** : ouvert — chemin incomplet établi par lecture, témoin minimal non exécuté
- **Famille** : produit
- **Gravité** : panique possible sur une API de parsing annoncée
- **Atteignable en service** : via le worker non-code ; non raccordé au routeur des huit langages de code
- **Touche rag3weaver** : oui, dépendance codeparsers ; aucun correctif runtime dans le lot de banc
- **Pour** : codeparsers, lot séparé après témoin rouge
- **Signalé par** : audit du banc multilangage le 16 novembre, codeparsers bd133850f89d6b12a3ca2b2f6ce238785ed37e3a

## Constat et témoin minimal à jouer

Dans `src/svelte/svelte_parser.rs`, `initialize` crée tree_sitter::Parser mais
la ligne `set_language` est commentée. `parse_file` appelle ensuite
`parser.parse(content, None).expect("Failed to parse Svelte")`.
Le worker `parallel/generic_worker.rs` expose ce chemin pour Svelte.

```rust
let mut parser = codeparsers::svelte::svelte_parser::SvelteParser::default();
parser.parse_file("minimal.svelte", "<p>Hello</p>", Default::default());
```

Lecture seulement : aucune panique observée et aucun témoin vert revendiqué.
Le banc refuse cette façade avant l'appel connu incomplet, avec statut explicite ;
il ne masque pas cette absence par zéro capture ou AST valide.

## Fin attendue

Voir le témoin rouge sous portée bornée, décider d'une grammaire opérationnelle
ou d'un refus public cohérent, puis témoins de non-panique et de captures sourcées.
Une absence de panique seule ne prouve pas la couverture Svelte.
Ticket parent : `2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md`.

## Relu le 18 novembre (session codeparsers)

Confirmé par lecture de codeparsers bd13385 : `src/svelte/svelte_parser.rs:55` porte `set_language` en commentaire, et `parse_file` (l. 69-70) fait `expect` sur l'analyseur puis sur l'arbre. Aucun témoin joué ; le rouge reste à voir.

Le parent `2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md` est clos et reste, avec le rapport de la campagne, sur la branche `cx-codeparsers-language-corpus` (non portée : son pointeur 1b35cfb, un outil du banc, ne va pas sur master).
