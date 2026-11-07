# La marque « nom » relie un appel de méthode standard à un homonyme

- **État** : ouvert (7 novembre 2026) — relevé par la sonde de la garde de match ; la prochaine
  mesure de cette famille.
- **Gravité** : réponse fausse (une arête vers une méthode qui n'est pas celle appelée).
- **Atteignable en service** : oui, tout dépôt dont l'index contient un homonyme d'une méthode de la
  bibliothèque standard (ici lucivy, indexé avec rag3weaver).
- **Touche rag3weaver** : oui (`src/code.rs`, la résolution par le nom) et codeparsers (le type lu).

## Ce que c'est

Un appel de méthode dont le receveur a un type **hors de l'index** (un type de la bibliothèque
standard : `Vec`, `Duration`) n'a pas de définisseur à son type ; la résolution retombe sur le seul
nom et relie l'appel à un homonyme indexé ailleurs. De même pour une **variante d'enum**
(`CypherValue::Float(f)`), qui n'est pas un scope : le nom va à un autre `Float`. C'est voisin, mais
distinct, de [des receveurs restent sans type](2026-10-04-des-receveurs-restent-sans-type.md) : ici le
type se lit, il n'est seulement pas dans l'index.

## Les cas (code-lourd, 7 novembre 2026)

Les cinq « nom » fausses des 19 arêtes que le correctif de la garde de match a fait apparaître
(rust-analyzer, juge strict) :

| mention | appel | notre cible | ce que c'est |
|---|---|---|---|
| `generic_search_nodes.rs:2091` (`RerankNode`) | `results.split_off(pool)` | `ld-lucivy/ownedbytes/src/lib.rs:107` | `Vec::split_off` |
| `generic_search_nodes.rs:2091` (`execute`) | `results.split_off(pool)` | `ld-lucivy/ownedbytes/src/lib.rs:107` | `Vec::split_off` |
| `rag3db_connection.rs:338` (`Rag3dbConnection`) | `!reste.is_zero()` (dans une garde) | `ld-lucivy/lucivy-fst/src/raw/mod.rs:1296` | `Duration::is_zero` |
| `rag3db_connection.rs:338` (`open_for_writing`) | `!reste.is_zero()` (dans une garde) | `ld-lucivy/lucivy-fst/src/raw/mod.rs:1296` | `Duration::is_zero` |
| `record_nodes.rs:746` (`pousser_cellule`) | `CypherValue::Float(f) =>` | `ld-lucivy/src/query/boolean_query/block_wand.rs:313` | la variante `CypherValue::Float` (`ir/src/value.rs:20`) |

## La recette

La sonde des CONSUMES sur code-lourd (`tests/sonde_aretes_par_relation.rs`, `SONDE_REQUETE`), puis
`scripts/oracle_lsp.py rust` sous `ORACLE_STRICT=1` sur les arêtes « nom » d'un mentionneur Rust.

## Ce qu'il faut pour le fermer

1. La mesure de la famille : la part des « nom » d'un mentionneur Rust dont le type du receveur est lu
   mais hors de l'index (et des variantes d'enum), jugée par l'oracle.
2. Puis la règle générique, à mesurer contre elle : un appel dont le type du receveur est lu et hors
   de l'index ne se relie pas par le seul nom (il s'abstient et le dit, comme une mention invisible).
