# `rag3weaver-backend ide --help` frappe un code d'entrée

- **État** : ouvert (22 novembre 2026, 23 h 30), rag3weaver archi ; reporté au matin par c7 (le MVP de la nuit).
- **Famille** : produit
- **Gravité** : `ide` ignore ses arguments : `--help` ou une option inconnue frappe un code humain à usage unique et l'imprime.
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (`src/bin/rag3weaver-backend.rs`).
- **Pour** : archi.
- **Signalé par** : c7 et archi, la nuit du terminal.

## Ce que c'est

Relevé par G le 22 nov. au soir. `ide --help` et une option inconnue doivent donner l'aide ou un refus, sans rien frapper ; `--help` pour chaque sous-commande du binaire. Lot `ide-args`, rouge d'abord.
