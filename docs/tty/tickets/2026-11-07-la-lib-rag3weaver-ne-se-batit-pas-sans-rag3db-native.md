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

## Reprise mémoire du 12 novembre — qualification du témoin de liens seulement

Pendant le ciblé du ticket M8, sur la source59b3f0ff7/base6e5f9458c, `cargo test --no-default-features --lib template_plan::tests::a_rejected_candidate_does_not_extract_a_builtin_cache -- --exact --nocapture` échoue en compilation E0432 sur l’import de Rag3dbConnection dans record_nodes.rs:4570. Zéro témoin M8 exécuté ; journal luciepc `~/.cache/rag3weaver-build/cx-memoire-m8-rouge.log`. Le ciblé natif M8 reste en file, source figée.

L’orchestration autorise une correction isolée : seule l’addition de `#[cfg(feature = "rag3db-native")]` sur `l_annulation_d_un_lot_de_liens_ne_retire_que_ce_qu_il_a_cree`. Branche `cx-memoire-cfg-temoin-natif`, porteur mémoire pour ce sous-cas. Aucune modification de backend.rs, des features par défaut, des assertions ou de la suite des accords. Le commit isolé `80faa48385ad4600122c545b876255a7c53050eb` reste à vérifier par compilation minimale et exécution effective du témoin natif sous poste ; aucun de ces résultats n’est déclaré acquis ici.

Le ticket global reste ouvert : le cas backend/daemon et la couverture des jeux de features sont distincts. La qualification du seul témoin de liens ne suffit pas à les fermer. Les consommateurs peuvent intégrer le seul commit de cfg, sans reprendre les sources M8.
