# La lib rag3weaver ne se bâtit pas sans `rag3db-native`

- **État :** corrigé 9231f07dc, c74e3310d et 5a4049b9e ; qualification F8 sur 9ec421069, publiée par cdcc032df. Constat initial : Trouvé par la session tty le 7 nov. 2026. Le terminal l'a contourné en sortant son client dans un crate à part (`extension/rag3weaver/tty`).
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

## Observation arbre-principal du 12 novembre, ciblé minimal après cfg isolé

Source privée **390b70185d3c466b3d1c5379c2d14b5f36ee71b6**, cfg mémoire80faa
intégré séparément. Cargo test --no-default-features --lib, filtre aperçu transfert,
j8/Mem32/lib29 : **rc101 en112,69s, aucun test joué**. L'import natif manquant
n'est plus signalé ; deux helpers cfg(test) deviennent du dead_code fatal :
read_pool.rs:196 Instantane::vieillir et backend.rs:4005 Backend::call_tool.
Leurs appels sont dans les témoins natifs ; aucun de ces deux fichiers n'est
modifié par ce lot. Attribution statique à cette surface de montage préexistante,
pas de rouge métier du transfert, pas de preuve de correction globale du ticket.

Preuves luciepc : cx-transfert-apercu-cible-v2/{cargo.log,source.json,resultat.json,VERDICT}.
Source propre inchangée, portée inactive et place arbre rendue. Pas de retry
inchangé ni de allow(dead_code) ajouté. Coordination demandée à C7 : qualification
sélective des helpers par leurs propriétaires, ou ciblé natif du même aperçu
sans prétendre prouver le minimal ; aucun backend partagé modifié par arbre.


## Clôture documentaire du 13 novembre 2026 — porteur tty

Les critères sont satisfaits sans imposer le moteur par défaut :

- `9231f07dc` isole l'ouverture de vérification dans `base_de_verification` :
  connexion native sous `rag3db-native`, refus explicite sans cette feature.
  Les ouvertures natives de stockage actuelles sont également sous cette feature.
- `c74e3310d` qualifie le témoin de `record_nodes.rs` par `rag3db-native`.
  Le helper `Backend::call_tool` est désormais sous `cfg(all(test, feature = "rag3db-native"))`.
- `5a4049b9e` ajoute les jeux vide et `daemon` à `JEUX` dans
  `scripts/check_features.sh`. Chaque entrée appelle `check --tests
  --no-default-features --features "$jeu"` : la bibliothèque normale et ses
  cibles de test sont vérifiées, sans prétendre exécuter les tests.

Preuve publiée : [qualification finale Recherche](../../recherche/13-11-2026/13-preuves-finales-union-garde-target.md),
commit `cdcc032df`, APP exact `9ec421069320b2f0cca4099fa27f83059e944de9`.
Relecture directe des archives locales hors worktree :
`~/.cache/rag3weaver-build/cx-preuves-garde-target-union-v2-final/client/features.log`
porte `VERT  sans-feature`, `VERT  daemon`, puis `CHECK_FEATURES VERT 8/8`.
Les journaux `client/features/sans-feature.log` et `daemon.log` finissent par
`Finished dev profile`, respectivement en 18,97 s et 11,28 s.
Le manifeste publié dans `garde-target-union-final-preuves/manifest.json`
référence ces journaux. Les rouges historiques ci-dessus restent des rouges
historiques ; aucune exécution nouvelle n'a été lancée par tty pour cette clôture.

Le crate tty séparé est conservé : c'est une frontière de client portable,
son déplacement dans la bibliothèque n'est pas un critère de ce ticket.
Aucun reste à faire pour ce défaut ; aucun lot tty-4 ou thème n'est lancé.
