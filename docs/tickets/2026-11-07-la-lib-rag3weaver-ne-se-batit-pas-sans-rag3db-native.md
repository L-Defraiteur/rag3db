# La lib rag3weaver ne se bâtit pas sans `rag3db-native`

- **État :** ouvert. Trouvé par la session tty le 7 nov. 2026. Le terminal l'a contourné en sortant son client dans un crate à part (`extension/rag3weaver/tty`).
- **Gravité :** bâti refusé (aucune réponse fausse).
- **Atteignable en service :** non. Tout bâti réel pose `rag3db-native`.
- **Touche rag3weaver :** oui.
- **Famille** : produit

## Ce que c'est

`default = []`, mais `cargo build --lib` sans `rag3db-native` échoue : du code hors `cfg(test)` appelle `crate::Rag3dbConnection`, qui n'existe que sous `rag3db-native`. Un client qui veut la lib sans le moteur (un binaire pour Windows ou macOS, un outil léger) ne peut donc pas la prendre.

## La recette

```bash
cd extension/rag3weaver && cargo build --lib -j 8 --features daemon
# error[E0433]: cannot find `Rag3dbConnection` in `crate` --> src/backend.rs:4411:27
```

Les tests de la lib (`cargo test --lib`) ajoutent `src/dataflow/record_nodes.rs:4567` (`use crate::{…, Rag3dbConnection}`).

## Le témoin

Aucun pour l'instant. Il faudrait une ligne `--no-default-features` et une ligne `--features daemon` dans `scripts/check_features.sh`, rouges aujourd'hui.

## Ce qu'il faut pour le fermer

1. Mettre sous `#[cfg(feature = "rag3db-native")]` ce qui appelle `Rag3dbConnection` hors du moteur : `src/backend.rs:4411` et son appelant, `src/dataflow/record_nodes.rs:4567`.
2. Ajouter les deux lignes au témoin de `check_features.sh`.

Une alternative : déclarer que la lib exige le moteur, en mettant `rag3db-native` dans `default` ou en écrivant un `compile_error!` explicite. Ce choix revient à Lucie.
