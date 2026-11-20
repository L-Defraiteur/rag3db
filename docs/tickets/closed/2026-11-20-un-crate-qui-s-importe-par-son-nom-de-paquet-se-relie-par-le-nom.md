# Un crate qui s'importe par son nom de paquet se relie par le nom

- **État** : corrigé (20 novembre 2026) — branche rag3db `codeparsers/package-name-root-2`, codeparsers 6c05283, règles de l'analyse 11
- **Famille** : produit
- **Gravité** : basse : l'arête existe et `usages` la compte, mais elle porte la marque « nom » (devinée) au lieu
  d'« import »
- **Atteignable en service** : oui, dans les tests et les exemples d'un crate Rust (`use rag3weaver::Catalog` dans
  `extension/rag3weaver/tests/*.rs`)
- **Touche rag3weaver** : oui (`code.rs`, `module_designe_fichier`)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, au correctif de `usages Catalog` (c7, 19 novembre)

## Constat

Dans les tests d'un crate, `use monlib::Catalog;` puis `Catalog::new()` : `monlib` est bien un paquet du projet
(provenance `project`, lue dans `Cargo.toml`), mais le module de l'import, `monlib::Catalog`, ne désigne aucun
fichier — `module_designe_fichier` retire `crate`, `self` et `super` en tête, pas le nom du paquet. Le rendez-vous
vers `Catalog` se résout alors par le seul définisseur, marqué « nom ». Témoin du comportement actuel :
`usages_d_un_type_compte_ses_sites_type_methode_d_un_type_importe` (e2e_usages), qui imprime « CONSUMES Catalog nom ».

## Ce qu'il faudrait

Lire un chemin qui commence par le nom d'un paquet Rust du projet comme `crate::` de ce paquet (sa racine `src/`),
pour les modules d'import et les chemins qualifiés. Témoin rouge d'abord : l'arête `essai → Catalog` est marquée
« import ».

## Fermé

20 novembre 2026, session codeparsers, tous les langages où la forme existe. Les manifestes gardent, pour chaque
paquet du dépôt, son dossier et ses points d'entrée (`Manifestes::paquets_du_depot` : `src/lib` d'un crate ; `types`,
`module`, `main`, `src/index`, `index` d'un paquet npm) ; un module d'import qui commence par le nom du paquet
(`monlib::a::B`, `"monpaquet"`, `"monpaquet/x"`) devient ces fichiers candidats (`modules_d_un_paquet_du_depot`). En
Python, le nom du paquet est déjà son dossier : ce qui manquait, c'est la réexportation de `__init__.py` — codeparsers
la marque désormais (avec `export { X } from` en TypeScript), donne à un `__init__.py` ses scopes de fichier pour la
porter, et rag3weaver suit les réexportations relatives (`.lib`, `./catalog`).

Témoin `un_paquet_du_projet_importe_par_son_nom_se_resout_par_l_import` (e2e_usages) : Rust, Python, TypeScript et
TypeScript réexporté, rouges vus puis verts ; témoins codeparsers `tests/reexports_python_ts.rs`, rouges vus puis verts.
Sonde de marque (lisant désormais les manifestes, comme l'index) : sur le crate rag3weaver entier, 5 493 arêtes passent
de « nom » à « import » (même ligne, même cible) et 325 s'ajoutent (tests, codeparsers, tty, la démo Node.js) ;
`tools/python_api` : +7 (les réexportations de son `__init__.py`) ; aucune retirée nulle part. Suites e2e_usages 45,
e2e_code_sync 25, e2e_postgres 39 vertes ; codeparsers 76 lots sur 76.
