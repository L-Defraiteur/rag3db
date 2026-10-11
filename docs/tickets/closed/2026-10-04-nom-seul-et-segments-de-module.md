# Les « nom seul » et les segments de module

- **État** : corrigé — codeparsers `a6111cc` (11 oct. 2026, P1 de la stèle du produit) ; témoin `codeparsers/tests/nom_seul_rust.rs`
- **Gravité** : réponse fausse (une part des arêtes « nom » est fausse)
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (rendez-vous, usages, Liens)

## Ce que c'est

Deux familles d'arêtes posées par le seul nom, que ni le type ni l'import ne
confirment (sonde `tests/sonde_marques.rs`, 4 octobre, après le palier A) :

1. **Le nom seul** (2 768 arêtes, 28 % des « nom ») : une référence ni
   appelée ni qualifiée. Mêlé : des types justes dans une signature
   (`Vec<PortDef>` → `PortDef` de port.rs) et des homonymes faux — un
   paramètre `f: &mut fmt::Formatter` relié au `f` de trace_nodes.rs.
2. **Les segments de module** : dans `crate::dataflow::X`, `dataflow` est
   relié au `pub mod dataflow;` de lib.rs. Vrai, mais ça n'apprend rien et
   ça encombre les voisinages.

## Recette minimale

```rust
// a.rs
pub fn f() {}
// b.rs
impl std::fmt::Display for B { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { Ok(()) } }
```

Attendu : pas d'arête `fmt → f` (le paramètre n'est pas la fonction).

## Témoin

La sonde, classe « nom seul (type, valeur) » et « nom par chemin de module ».

## Pour le fermer

1. Un paramètre ou une liaison locale qui porte le nom le cache (codeparsers
   le fait pour les locaux Rust des motifs ; à étendre aux paramètres
   nommés dans la signature, qui ne sont pas des usages).
2. Un segment de chemin qui nomme un module n'est pas une référence, ou une
   référence d'un genre à part que les marches ne suivent pas.

## Correction (11 octobre 2026)

codeparsers `a6111cc` :
1. **Le paramètre** : un bloc `impl` lisait ses références sur tout son corps,
   paramètres des méthodes compris (`f` de `fn fmt(&self, f: &mut Formatter)`,
   `name` de `fn create(&self, name: &str)`) ; ils sont exclus. Arêtes d'appel
   de rag3weaver comparées une à une : 96 retirées, toutes de cette forme,
   fausses ; aucune gagnée.
2. **Les segments de module** : un préfixe de chemin en minuscules
   (`dataflow` dans `crate::dataflow::X`, `fmt` dans `fmt::Formatter`) n'est
   plus une référence — dans le code comme dans les jetons d'une macro ; un
   préfixe en majuscule (un type) en reste une. Sur rag3weaver, les
   non-résolues baissent de près de 7 000.
**Reste**, hors de ce ticket : les types justes d'une signature
(`Vec<PortDef>` → `PortDef`) restent des références par le nom quand le nom
est unique — c'est juste et marqué « nom », pas un faux.
