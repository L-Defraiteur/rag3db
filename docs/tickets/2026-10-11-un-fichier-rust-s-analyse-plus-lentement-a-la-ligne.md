# Un fichier Rust s'analyse 2,6 fois plus lentement à la ligne qu'un fichier C++

- **État** : ouvert (11 octobre 2026) — mesuré par la session codeparsers en cherchant les 6 s de la réindexation à chaud
- **Gravité** : lenteur (une réindexation à chaud d'un gros fichier Rust coûte près d'une seconde d'analyse à elle seule)
- **Atteignable en service** : oui, à chaque édition d'un gros fichier Rust vue par la montre
- **Touche rag3weaver** : oui, par codeparsers (analyse d'un fichier Rust)

## Ce que c'est

`parse_project` pour un seul fichier, résolution comprise, médiane de 5 essais (codeparsers 353d2c4,
exemple jetable `temps_un_fichier`) :

| fichier | lignes | temps |
|---|---|---|
| `src/processor/processor.cpp` | 92 | 5,4 ms |
| `src/storage/table/node_table.cpp` | 1 387 | 116 ms |
| `extension/rag3weaver/src/code.rs` | 3 628 | 790 ms |

Rapporté à la ligne, `code.rs` coûte environ 0,22 ms contre 0,08 ms pour `node_table.cpp` : **2,6 fois**
(et non « sept fois à taille comparable », comme le disait la première version de ce ticket —
`code.rs` est 2,6 fois plus long). C'est le seul Rust de la mesure : un second fichier Rust est à
mesurer avant de conclure à une faute du Rust plutôt que de ce fichier. Suspect : les liaisons de receveurs Rust (`typed_bindings_full`,
`deferred_bindings`, `closure_bindings`), qui parcourent l'arbre d'une fonction pour chaque scope,
peut-être de façon quadratique sur un fichier aux longues fonctions.

## Ce qu'il faut pour le fermer

Un relevé par phase de l'analyse d'un fichier Rust (extraction des scopes, liaisons, références,
résolution) sur `code.rs`, puis la phase qui coûte rendue linéaire ; témoin : le même temps rapporté
à la ligne qu'un fichier C++, à 1,5 fois près.
