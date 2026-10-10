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

| forme | non résolues | dont storage/transaction | coût | lot qui la ferme | exemples |
|---|---|---|---|---|---|
| méthode sur un receveur (x.f(), x->f()) | 20713 | 4559 | 75816 | B2 receveurs typés | `ssString.c_str` (export_db.cpp), `ss.str` (export_db.cpp), `ss.str` (export_db.cpp) |
| nom seul | 45326 | 10280 | 27803 | paramètres et locaux | `pragma` (factorization_rewriter.h), `once` (factorization_rewriter.h), `groupsPos` (factorization_rewriter.h) |
| import | 13120 | 1997 | 15117 | #include → fichier | `include` (factorization_rewriter.h), `logical_operator_visitor` (factorization_rewriter.h), `h` (factorization_rewriter.h) |
| appel nu | 3320 | 448 | 11304 | B1 rendez-vous (autre fichier) / locaux | `__builtin_sub_overflow` (subtract.cpp), `BinderException` (cost_function.cpp), `ColumnPredicateSet` (map_extend.cpp) |
| macro ou constante (MAJUSCULES) | 9861 | 2477 | 6169 | macros #define | `__GNUC__` (subtract.cpp), `FileFlags.WRITE` (export_db.cpp), `FileFlags.CREATE_IF_NOT_EXISTS` (export_db.cpp) |
| lecture de champ (x.y, x->y) | 7564 | 1498 | 4531 | champs : ne pas relier (comme Rust) | `it.first` (export_db.cpp), `it.second` (export_db.cpp), `info.options` (export_db.cpp) |
| appel par chemin (Ns::f, T::f) | 224 | 74 | 894 | B1 chemins / rendez-vous | `TransactionManager::Get(*clientContext).getLockManager` (transaction.cpp), `storage::StorageManager::Get(*clientContext).getWAL` (transaction.cpp), `Catalog::Get(context).getTableCatalogEntry` (transaction.cpp) |
| membre par chemin (T::x) | 25 | 20 | 45 | membres statiques, enums | `common::Timestamp::getCurrentTimestamp().value` (transaction.cpp), `common::Timestamp::getCurrentTimestamp().value` (transaction.cpp), `common::Timestamp::getCurrentTimestamp().value` (transaction.cpp) |
| type | 1 | 1 | 3 | B1 | `getResidencyState` (column_chunk.h) |
| std:: (externe) | 659 | 253 | 0 | externe : à dire compris | `std.stringstream` (export_db.cpp), `std.make_pair` (map_path_property_probe.cpp), `std.wstring` (extension.cpp) |

## rag3weaver `src/` et `tests/` (Rust, 252 fichiers)

| forme | non résolues | dont storage/transaction | coût | lot qui la ferme | exemples |
|---|---|---|---|---|---|
| méthode sur un receveur (x.f(), x->f()) | 35473 | 0 | 106419 | B2 receveurs typés | `name.to_string` (schema_nodes.rs), `"schema".into` (schema_nodes.rs), `name.to_string` (schema_nodes.rs) |
| appel nu | 4976 | 0 | 14928 | B1 rendez-vous (autre fichier) / locaux | `into` (schema_nodes.rs), `to_string` (sonde_copy_vecteur.rs), `to_string` (sonde_copy_vecteur.rs) |
| type | 7626 | 0 | 11439 | B1 | `_` (schema_nodes.rs), `serde_json.Value` (schema_nodes.rs), `serde_json.Value` (schema_nodes.rs) |
| nom seul | 18433 | 0 | 9216 | paramètres et locaux | `node_name` (schema_nodes.rs), `cible` (schema_nodes.rs), `gabarit` (schema_nodes.rs) |
| appel par chemin (Ns::f, T::f) | 2340 | 0 | 7020 | B1 chemins / rendez-vous | `Rag3dbConnection::in_memory().unwrap` (sonde_copy_vecteur.rs), `Rag3dbConnection::in_memory().unwrap` (sonde_copy_vecteur.rs), `Rag3dbConnection::in_memory().unwrap` (sonde_copy_vecteur.rs) |
| import | 5760 | 0 | 5760 | #include → fichier | `std` (schema_nodes.rs), `sync` (schema_nodes.rs), `Arc` (schema_nodes.rs) |
| lecture de champ (x.y, x->y) | 5381 | 0 | 2690 | champs : ne pas relier (comme Rust) | `serde_json.Value` (schema_nodes.rs), `serde_json.Value` (schema_nodes.rs), `PortType.Map` (schema_nodes.rs) |
| membre par chemin (T::x) | 1447 | 0 | 1447 | membres statiques, enums | `super::node_registry::Choices.Targets` (schema_nodes.rs), `super::node_registry::Choices.Targets` (schema_nodes.rs), `rag3weaver_ir::Direction.Incoming` (react_nodes.rs) |
| macro ou constante (MAJUSCULES) | 2334 | 0 | 1167 | macros #define | `CSV_NULL` (sonde_copy_vecteur.rs), `CSV_NULL` (sonde_copy_vecteur.rs), `SCOPE` (sonde_identifiants_lucivy.rs) |
| héritage | 366 | 0 | 732 | B1 | `Default` (react_nodes.rs), `std::ops::BitOr` (disponibilite.rs), `std::ops::BitOrAssign` (disponibilite.rs) |
| std:: (externe) | 5946 | 0 | 0 | externe : à dire compris | `std.any` (schema_nodes.rs), `std::any.Any` (schema_nodes.rs), `std.any` (schema_nodes.rs) |

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
