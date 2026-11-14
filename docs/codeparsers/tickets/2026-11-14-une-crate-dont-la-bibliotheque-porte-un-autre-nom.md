# Une crate dont la bibliothèque porte un autre nom n'est pas reconnue à l'import

- **État** : ouvert (14 novembre 2026, codeparsers) — souhait
- **Famille** : produit
- **Gravité** : basse. Rien n'est faux : l'import reste `inconnue`, jamais une provenance devinée.
- **Atteignable en service** : oui, rarement — une crate dont `[lib] name` diffère du nom de paquet
  (`rust-crypto` → `crypto`).
- **Touche rag3weaver** : oui (`manifestes.rs` lit le nom du paquet, pas celui de sa bibliothèque)
- **Signalé par** : session codeparsers, à la relecture des règles d'import des sept langues (point 8)
- **Pour** : session codeparsers

## Ce qu'on voit

Le Cargo.lock et les `[dependencies]` donnent le nom du **paquet** ; l'import Rust écrit le nom de la
**bibliothèque** (`use crypto::…`). Une crate qui déclare `[lib] name = "crypto"` sous le paquet
`rust-crypto` reste `inconnue`. Un paquet renommé côté dépendant (`foo = { package = "bar" }`) est,
lui, bien lu par sa clé.

## Ce qu'il faudrait

La table des renommages connus (la même que pour Python, en données) ou la table déclarée du manifeste
du backend (`imports`), qui couvrent ce cas sans lire les sources des crates.
