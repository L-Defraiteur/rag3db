# Un crate qui s'importe par son nom de paquet se relie par le nom

- **État** : ouvert (20 novembre 2026)
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
