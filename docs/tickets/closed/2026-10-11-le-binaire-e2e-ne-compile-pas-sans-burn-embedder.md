# Le binaire e2e ne compile pas sans burn-embedder

- **État** : corrigé (le commit qui ouvre ce ticket le ferme ; relevé par G, porté
  par la session recherche)
- **Gravité** : blocage — `cargo check --tests` rouge sur master sous un jeu de
  features employé (celui du paquet npm)
- **Atteignable en service** : non (tests)
- **Touche rag3weaver** : oui (`tests/`)

## Ce que c'est

Depuis le binaire e2e unique, chaque suite est un module de `tests/e2e/main.rs` :
une suite dont le `#![cfg]` oublie une feature qu'elle emploie ne casse plus
seulement son propre binaire, elle casse **tout** le binaire sous un jeu qui
n'a pas cette feature. `e2e_mesure_joignabilite` appelle
`common::burn::GRANITE_278M`, qui n'existe que sous `burn-embedder`, alors que
son `cfg` ne demandait que `rag3db-native`, `code` et `daemon`.

## La recette

```
cargo check --tests --features rag3db-native,code,daemon,openai-llm
```

rend `error[E0433]: cannot find `burn` in `common``
(`tests/e2e_mesure_joignabilite.rs:141`). Rejoué avec l'ancien `cfg` remis
seul : rouge ; avec le correctif : vert.

## Le témoin

`scripts/check_features.sh` : `cargo check --tests` sous les quatre jeux
employés (la batterie, la batterie avec `openai-llm`, le jeu sans embarqueur
local, le minimum natif). Il rend 1 dès qu'un jeu est rouge. La liste et la
règle sont dans `extension/rag3weaver/docs/11-octobre-2026-recherche/03-les-e2e-en-un-binaire.md`, §8.

## La cause et le correctif

Le `cfg` de la suite demande désormais `burn-embedder`. Au passage, les
avertissements que ces jeux faisaient lever sont tus : `use crate::common;`
inutilisé dans 26 suites sous certains jeux, et la macro
`test_de_cette_suite!` inutilisée quand aucune suite qui se relance n'est
compilée.
