# La lib ne compile pas sans la feature `code`

- **État** : corrigé (11 octobre 2026) par l'arbre principal : l'arrêt des indexations (`stop_indexings_of`, marche 0) est sous `#[cfg(feature = "code")]` dans `Backend::shutdown` et dans l'hôte du démon ; `cargo check --features rag3db-native` seul repasse, et il entre dans les contrôles de l'arbre principal avant tout hash.
- **Gravité** : blocage (une combinaison de features ne se bâtit plus)
- **Atteignable en service** : non (les binaires livrés portent `code`)
- **Touche rag3weaver** : oui (`src/backend.rs`)

## Ce que c'est

`Backend::shutdown` appelle `crate::dataflow::index_nodes::stop_indexings_of`
sans garde, alors que le module `index_nodes` n'existe que sous la feature
`code` : la lib seule, ou avec toute combinaison sans `code`, ne compile plus.

## Recette

```bash
cd extension/rag3weaver
cargo check --release --features rag3db-native
```

```
error[E0433]: cannot find `index_nodes` in `dataflow`
    --> src/backend.rs:2942:42
     |
2942 |         let attendues = crate::dataflow::index_nodes::stop_indexings_of(&self.catalog);
     |                                          ^^^^^^^^^^^ could not find `index_nodes` in `dataflow`
     |
note: found an item that was configured out
    --> src/dataflow/mod.rs:49:9
```

Vu sur master b4f801d11 (11 octobre 2026, après-midi) ; la ligne était 2897 à 56a680169.

## Témoin

`scripts/check_features.sh` (les quatre jeux de features de `cargo check --tests`),
s'il compte un jeu sans `code` ; sinon la commande ci-dessus.

## Cause

L'arrêt des indexations en cours à la fermeture (marche 0 de la forme C) a été ajouté
dans `Backend::shutdown`, qui est hors de la feature `code`, en appelant un module qui
y est.

## Pour le fermer

Garder l'appel sous `#[cfg(feature = "code")]` (sans `code`, il n'y a pas
d'indexation à arrêter), ou déplacer `stop_indexings_of` dans un module toujours
présent ; puis vérifier `cargo check --features rag3db-native` et
`scripts/check_features.sh`.
