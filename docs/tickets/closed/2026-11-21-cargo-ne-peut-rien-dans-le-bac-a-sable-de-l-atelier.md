# cargo ne peut rien dans le bac à sable de l'atelier

- **État** : corrigé (21 novembre 2026) — branche rag3db `atelier-cargo`, l'atelier livré déclare la chaîne Rust en lecture
- **Famille** : produit
- **Gravité** : moyenne : dans le chat de l'atelier, toute commande cargo (`cargo init`, `cargo build`, `cargo test`) échoue avant de rien faire
- **Atteignable en service** : oui, à tout atelier posé par `rag3weaver-backend atelier`
- **Touche rag3weaver** : oui (`templates/backends/atelier/backend.json`, le bac à sable des commandes)
- **Pour** : session recherche
- **Signalé par** : session recherche, scénario 2 de la nuit du 20 au 21 novembre (un projet de zéro depuis l'IDE)

## Le pas exact

Dans le chat de l'atelier, sur un hôte isolé bâti de master a60f9f7a71 : « crée une bibliothèque Rust qui compte les
mots, et son dépôt git ». La carte `cargo init --lib` accordée, la commande échoue : « accès refusé sur le fichier de
configuration local de rustup ». L'agent écrit alors `Cargo.toml` à la main.

## La cause

Le bac à sable des commandes (Landlock) n'ouvre que le dossier de l'atelier, plus ce que le manifeste déclare en
`workspace.sandbox.extra_read`. Le backend de code livré déclare la chaîne Rust (`~/.rustup`, `~/.cargo/bin`,
`~/.cargo/registry`, `~/.cargo/git`, `~/.cargo/config.toml` ; la panne du 8 novembre, essai 5 de c7) ; l'atelier livré
ne déclarait que `network`. Ce n'était pas un artefact du HOME isolé du scénario : sans lecture déclarée, `~/.rustup`
est hors domaine, lien ou pas.

## Le correctif

L'atelier livré déclare la même chaîne. Témoin : `builtin_templates::tests::the_delivered_workshop_lets_cargo_read_its_chain`
(rouge sur master, vert après).

## Pour un atelier déjà posé

Le manifeste posé garde sa copie : l'accueil de l'IDE le dit (« gabarit local différent du livré »). Reposer l'atelier
(`rag3weaver-backend atelier`), ou ajouter `extra_read` à son `workspace.sandbox` à la main.
