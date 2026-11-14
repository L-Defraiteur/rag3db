# Les listes de builtins par langage sont figées dans le code

- **État** : ouvert (14 novembre 2026) — le pas suivant du lot de la provenance des imports ; les
  tableaux restent aujourd'hui.
- **Gravité** : forme (une liste de données vit dans du Rust : la mettre à jour demande de recompiler et
  de relire du code).
- **Atteignable en service** : non (aucune réponse fausse ; une version de langage plus récente rend
  `inconnue` un module qu'elle ajoute, jamais une devinette).
- **Touche rag3weaver** : non (codeparsers) — sauf le pointeur.
- **Famille** : produit

- **Pour** : session codeparsers

## Ce que c'est

Le lot de la provenance des imports (codeparsers `provenance-imports`, c7c4b2b) dit `builtin` par des
listes fixes par langage : `RUST_STD_CRATES`, `NODE_BUILTINS`, `C_STDLIB_HEADERS`, `CPP_STDLIB_HEADERS`,
`DOTNET_BCL_NAMESPACES` (dans `src/import_resolution/*_import_resolver.rs`, réveillées) et
`PYTHON_STDLIB_MODULES` (`sys.stdlib_module_names` de Python 3.14.9, figé dans
`src/import_resolution/provenance.rs`). Ce sont des **données** dans du Rust.

**La forme cible** (Lucie, par c7 le 14 novembre : garder la culture déclarative même dans le compliqué) :
un fichier de données par langage livré avec le crate, `data/builtins/<langage>.txt`, avec sa version et
sa source en tête (par exemple `# python 3.14.9, sys.stdlib_module_names`), lu à l'ouverture, qu'on met à
jour sans recompiler ni relire du Rust. La règle de Go (un premier élément de chemin sans point est la
bibliothèque standard) reste du code : c'est une règle, pas une liste.

## Pourquoi pas dans le lot

Ce n'est pas « trois lignes » : un fichier lu **à l'exécution** doit se trouver là où le crate est installé
(pas `CARGO_MANIFEST_DIR`, qui ne vaut qu'au bâti ; `include_str!` obligerait à recompiler), avec une
règle quand il manque ; et les résolveurs de `src/import_resolution/` (du code mort, qui portent ces
listes) sont à retirer ou à rebrancher sur les mêmes fichiers, pas les deux.

## Ce qu'il faut pour le fermer

1. Où vivent les fichiers à l'exécution : un dossier de données du crate, désigné par une option de
   `RelationshipResolverOptions` (rag3weaver le passe, comme `project_files`) ou une variable
   d'environnement, et le repli quand il manque (les tableaux d'aujourd'hui, ou `inconnue` partout — à
   trancher).
2. `data/builtins/{rust,node,c,cpp,python,dotnet}.txt`, version et source en tête ; les tableaux retirés.
3. Les résolveurs morts de `src/import_resolution/` retirés, ou rebranchés sur les fichiers.
4. Le témoin `tests/provenance_imports.rs` inchangé et vert ; un témoin de plus : un nom ajouté au fichier
   devient `builtin` sans recompiler.
