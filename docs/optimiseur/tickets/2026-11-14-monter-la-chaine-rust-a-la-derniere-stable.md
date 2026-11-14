# Monter la chaîne Rust à la dernière stable

- **État** : ouvert (14 novembre 2026), à planifier une nuit calme (décision de Lucie et c7).
- **Famille** : produit
- **Gravité** : coût ; la chaîne épinglée prend du retard sur stable tant que personne ne la monte.
- **Atteignable en service** : non, c'est une montée volontaire.
- **Touche rag3weaver** : oui, `rust-toolchain.toml`, `.github/workflows/rag3weaver-workflow.yml`, `.github/workflows/paquet-npm.yml`, et les dépréciations du crate.
- **Pour** : optimiseur (G).
- **Ouvert le** : 14 novembre 2026

## Ce que c'est

Le dépôt épingle la chaîne Rust à **1.98.1** (le lot `rust-toolchain.toml`, dernier de la pile du 14 novembre,
sur le mot de Lucie : « oui épinglons en une version »). La dernière stable est **1.100.0**, déjà installée par
rustup aux deux postes (avec 1.99.0 au portable). Monter, c'est un lot à part.

## Le coût, relevé le 14 novembre

- **Les targets cargo** : une autre version de rustc invalide tout un target, que le premier `cargo` rebâtit en
  entier.
  - Portable : 17 dossiers de target, **130 Gio**, dont 13 utilisés en deux jours.
  - luciepc : 25 dossiers, **211 Gio**, dont 17 actifs. Les plus gros : lucivy-check-target 41,5 Gio,
    target-G/passe et target-activation 23,1 Gio chacun.
  - En tout, **~340 Gio réécrits** et ~30 rebâtis complets du crate, de l'ordre de 10 à 20 min chacun à 5 cœurs.
    Ce chiffre n'est pas chronométré.
- **Les dépréciations** : une seule est connue, `Atomic::fetch_update` (au profit de `try_update`), vue par la CI
  sous le jeu sans feature, dans `src/read_pool.rs` (le correctif de recherche, rang 11). Le compte sous les huit
  jeux est **à faire d'abord**.

## Le déroulé

1. `check_features` sous 1.100.0, sans rien changer (`cargo +1.100.0`, sous `poste lourd`) : le compte des
   dépréciations sous `-D warnings`, jeu par jeu.
2. Le lot : `rust-toolchain.toml` et les deux workflows à 1.100.0, les dépréciations corrigées d'un coup.
3. La passe complète (check_features, lib entière, navigateur, e2e) sous la nouvelle chaîne, une nuit où aucune
   passe ne tourne. Les targets se rebâtissent au premier usage.
4. Les deux postes s'alignent d'eux-mêmes par le fichier.

## Pour le fermer

Le lot sur master, la passe complète verte, et la CI verte sous la même version que les postes.
