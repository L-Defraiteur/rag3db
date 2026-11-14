# La chaîne Rust de la CI flotte, celle des postes non

- **État** : ouvert (14 novembre 2026) ; la décision entre les deux issues est à Lucie (posée par c7).
- **Famille** : produit
- **Gravité** : blocage ; la CI de master rougit sans qu'aucun lot ait changé, dès qu'une version stable de Rust ajoute un avertissement.
- **Atteignable en service** : oui, à chaque sortie d'une version stable de Rust.
- **Touche rag3weaver** : oui, `.github/workflows/rag3weaver-workflow.yml` (et le choix d'un `rust-toolchain.toml`).
- **Pour** : optimiseur (G).
- **Ouvert le** : 14 novembre 2026

## Ce que c'est

La CI de master est rouge depuis 67dabc121 (le push d'embarquement), au pas `check_features`, et aucun lot n'y
est pour rien. Le workflow `rag3weaver-workflow.yml` prend `dtolnay/rust-toolchain@stable` **sans version** :
il a pris une chaîne plus récente que celle des postes, qui déprécie `Atomic::fetch_update` (au profit de
`try_update`), et le `-D warnings` du jeu sans feature rougit sur `extension/rag3weaver/src/read_pool.rs:261`
(lecture de c7). Recherche corrige la ligne au rang 11 de la pile.

Les postes, eux, ne bougent que si on les monte. Relevé du 14 novembre :
- **portable** : rustup, canal `stable` par défaut, **rustc 1.98.1** (2026-09-01) ;
- **luciepc** : rustup, canal `stable` par défaut, **rustc 1.98.0** (2026-08-18) ;
- les deux postes ne sont donc **pas** non plus à la même version entre eux.

`paquet-npm.yml` épingle déjà `toolchain: 1.98.1` (depuis 9782b26a6, le 10 octobre) : un seul des deux workflows
est épinglé.

## Les deux issues

### 1. Épingler : un `rust-toolchain.toml` à la version des postes

Un fichier `rust-toolchain.toml` à la racine du crate, et à la racine du dépôt si besoin
(`[toolchain] channel = "1.98.1"`, avec `components = ["clippy", "rustfmt"]` si la CI s'en sert), lu par rustup
aux deux postes comme par `dtolnay/rust-toolchain` en CI ; `rag3weaver-workflow.yml` passe à
`toolchain: 1.98.1`, comme `paquet-npm.yml`.

- **Ce que ça gagne** : la CI et les postes compilent avec la même chaîne. Un rouge de CI veut dire un défaut du
  lot, jamais une montée de Rust. Les deux postes s'alignent aussi, puisque rustup lit le fichier : luciepc passe
  de 1.98.0 à 1.98.1 au premier `cargo`.
- **Ce que ça coûte** :
  - la montée de Rust devient un lot à part : changer la version, rejouer la passe complète (check_features, lib
    entière, navigateur, e2e), corriger les dépréciations d'un coup ;
  - si personne ne le fait, on prend du retard sur stable ;
  - au premier `cargo` après la pose, rustup télécharge la chaîne épinglée aux deux postes (environ 300 Mo, une
    fois) ;
  - un dépôt voisin (codeparsers, lucivy) qui compile dans le même arbre suit la même chaîne s'il n'a pas la
    sienne.

### 2. Laisser flotter : la CI sur `stable`, les postes montés au rythme de stable

`rag3weaver-workflow.yml` reste sur `@stable`. Les postes suivent par `rustup update` (et non `pacman -Syu` :
rustup gère la chaîne aux deux postes) à chaque nouvelle version stable, environ toutes les six semaines.

- **Ce que ça gagne** : on suit Rust sans lot dédié, et une dépréciation se voit le jour où elle arrive.
- **Ce que ça coûte** :
  - la CI rougit sans lot fautif à chaque version qui ajoute un avertissement sous `-D warnings`, comme
    aujourd'hui. Il faut un correctif urgent sur master, qui passe devant la file ;
  - entre la sortie et la montée des postes, la CI et les postes ne compilent pas la même chose : un vert de
    poste ne garantit pas un vert de CI ;
  - chaque montée des postes invalide les targets cargo (un rebâti complet de chaque target, plusieurs dizaines de
    Gio d'écriture à luciepc) ;
  - il faut une passe sous la nouvelle chaîne avant de la poser, comme pour une lib.

## Pour le fermer

La décision de Lucie, puis soit le `rust-toolchain.toml` et le workflow épinglé (issue 1), soit la règle de
montée écrite dans `docs/procedures/` et les postes alignés (issue 2). Dans les deux cas, les deux postes à la même
version.
