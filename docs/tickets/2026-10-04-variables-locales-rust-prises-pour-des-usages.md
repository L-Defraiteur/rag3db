# Une variable locale Rust passe pour l'usage d'une fonction homonyme

- **État** : corrigé dans codeparsers `75077f7` + `46cc492` — en attente du pointeur
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`usages`, `impact`, liens — vu par la section Liens)

## Ce que c'est

Dans `read_sse` (`openai_llm.rs`), `let s = match by_id.get(id) { Some(s) => *s, … }` :
les références à `s` ne sont pas reconnues comme une variable locale ; le
rendez-vous les relie à la seule fonction `s` du projet
(`fn s(v: &str) -> CypherValue`, `code.rs`). La section Liens montrait
`read_sse —utilise→ s (code.rs) ←utilise— run`.

## Recette minimale

```rust
// a.rs
pub fn s(v: &str) -> usize { v.len() }
// b.rs
pub fn f(x: Option<u32>) -> u32 {
    let s = match x { Some(s) => s, None => 0 };
    s + 1
}
```

Attendu : aucune arête `f → s`.

## Cause

L'extraction Rust exclut des références les paramètres et les symboles que
`collect_local_symbols` relève par le champ `name` ; une liaison de `let`
(champ `pattern`), un motif de `match` ou d'`if let`, un paramètre de
fermeture (`|p|`) n'en ont pas — ils restent des références « inconnues ».
Déjà noté dans `extension/rag3weaver/docs/3-octobre-2026-20h30/02`.

## Pour le fermer

Côté codeparsers : relever les identifiants liés par les motifs (`let`,
`match`, `if let`, `while let`, `for`, paramètres de fermeture) comme
locaux du scope qui les contient. Ce n'est pas une règle de rendu : un nom
d'une lettre peut être une vraie fonction.
