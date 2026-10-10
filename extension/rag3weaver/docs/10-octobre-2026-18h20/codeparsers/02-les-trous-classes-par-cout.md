# Les trous de codeparsers sur nos dépôts, classés par coût

10 octobre 2026. Mesuré par le banc de couverture (codeparsers `9b1460d`,
`cargo run --release --example banc_couverture -- <nom> <racine>... --trous`),
en fichier seul (le mode de rag3weaver). Chaque référence non résolue est
rangée par sa forme ; le coût pondère ce qu'elle retire aux outils
(`usages`, `impact`, voisinage) : un appel 3, un héritage 2, un type 1,5, un
import 1, un champ, une macro ou un nom seul 0,5, `std::` 0 ; **un trou dans
`storage/` ou `transaction/` pèse double**, c'est là qu'on cherche les
verrous.

## rag3db `src/` (C++, 1 543 fichiers)


## rag3weaver `src/` et `tests/` (Rust, 252 fichiers)


## Ce que ça dit, et l'ordre qui en sort

1. **La méthode sur un receveur non typé est le premier coût partout**
   (C++ 20 713 trous dont 4 559 dans storage/transaction ; Rust 35 473,
   surtout des méthodes std sur des valeurs non typées). C'est B2, et c'est
   aussi ce qui fera monter « qui appelle NodeTable::update ».
2. **Un artefact à fermer d'abord, certain et court** : en C++, le scope de
   fichier lit ses noms par une expression régulière sur le texte, directives
   du préprocesseur comprises — `#pragma once`, `#include "storage/x.h"`
   rendent des références `pragma`, `once`, `include`, `h`, `storage`… Elles
   gonflent « nom seul » et « import ». Une ligne de directive ne nomme rien
   (comme une ligne d'attribut en Rust) ; le `#include` reste une
   `ImportReference`.
3. **Les lectures de champ C++** (7 564) se relient par le nom comme en Rust
   avant le 4 octobre : `x.y` non appelé ne désigne pas une fonction.
4. **Les macros** (9 861 en C++, `KU_ASSERT`, `FileFlags.WRITE`) : à relier à
   leur `#define` quand il est dans le dépôt, sinon à dire externes.
5. **`#include` → fichier** : aujourd'hui aucune relation de fichier à
   fichier ; c'est ce qui donnerait « qui inclut ce .h ».
6. **`std::`** (659 en C++, 5 946 en Rust) : compris, mais compté non résolu
   — à dire externe plutôt que trou.

Ordre proposé : l'artefact du préprocesseur (petit, certain), puis B2 (les
receveurs C++ typés, avec la mesure « appelants de NodeTable::update »),
puis les champs C++, puis B1 / `#include`, puis B3 (LOCKS).
