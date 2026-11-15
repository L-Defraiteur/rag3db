# La marque « nom » relie un appel de méthode standard à un homonyme

- **État** : fermé (15 novembre 2026) — deux règles, codeparsers `fad1b6f` ; voir « La correction ».
  mesure de cette famille.
- **Gravité** : réponse fausse (une arête vers une méthode qui n'est pas celle appelée).
- **Atteignable en service** : oui, tout dépôt dont l'index contient un homonyme d'une méthode de la
  bibliothèque standard (ici lucivy, indexé avec rag3weaver).
- **Touche rag3weaver** : oui (`src/code.rs`, la résolution par le nom) et codeparsers (le type lu).
- **Famille** : produit

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

## La correction (15 novembre 2026)

La recette s'est montrée plus étroite que prévu. Sur les cinq cas, le type du receveur **ne se
lisait pas** : `reste` vient de `REOPEN_WAIT.saturating_sub(…)`, `results` d'une chaîne
`ctx.take_input(…).and_then(…).ok_or(…)?`. La variante, elle, portait l'import
`crate::value::Value`, qui ne désignait pas l'homonyme. Le seul nom concluait donc.

Les témoins rouges sont dans `tests/e2e_usages.rs` (`…_ne_va_pas_a_un_homonyme`, trois cas) : rouges
avant, verts après. Le témoin `une_methode_du_projet_sur_une_locale_sans_type_garde_son_lien_par_le_nom`
dit ce qui reste. Côté codeparsers, c'est `tests/marque_nom.rs`.

Les deux règles (décision de c7, 15 novembre) :

- **(a) Un chemin sur un type qui n'est pas un appel** (`Value::Float(f) =>`, `LogicalTypeID::STRING`,
  `Foo::MAX`) lit son type, comme un appel le faisait déjà. Une variante n'est pas un scope, donc le
  type lu s'abstient au lieu d'aller à l'homonyme.
- **(b) Un appel de méthode de la bibliothèque standard sur une locale dont aucun type ne se lit**, ni sur
  place ni ailleurs, porte la marque `receiver_type_unread`. Le rendez-vous de rag3weaver le traite comme
  `untyped_receiver` : visible, jamais relié entre fichiers. C'est une abstention dite. Dans le fichier,
  le lien par le nom de l'analyseur reste (décision du 11 octobre 2026). La liste est en données,
  `codeparsers/data/std_methods/<langue>.toml`, générée par `generer.py` depuis la bibliothèque standard
  du poste : la doc de std de rustc 1.98.1 (1 266 noms), l'AST de clang 23 sur libstdc++ (193), `dir()`
  de Python 3.14. Une méthode propre au projet (`backend.reload()`, `n.aller()`) garde son lien « nom ».

La forme large de (b), toute locale sans type lu, a aussi été mesurée. Elle ôtait 959 liens justes
de plus sur rag3weaver/src, vers des méthodes du projet. Elle est écartée.

### La sonde (`tests/sonde_marque_nom.rs`)

Corpus figé de rag3db-garde (343ead799), indexé en mémoire. Avant : codeparsers ff82d22 et le
`code.rs` de master. Après : la branche.

| corpus | marque | avant | après |
|---|---|---|---|
| rag3weaver/src (290 fichiers) | nom | 10 702 | 10 114 |
| | import | 11 699 | 11 650 |
| | fichier | 18 950 | 18 934 |
| | type | 7 091 | 7 116 |
| | rendez-vous non typés (abstentions dites) | 5 920 | 9 918 |
| src/ du moteur (1 550 fichiers) | nom | 5 092 | 2 705 |
| | type | 10 120 | 10 214 |
| | fichier | 6 078 | 6 080 |

Les arêtes perdues se lisent une à une (TSV de la sonde) :

- **src/ du moteur : 2 329**, toutes par (a). Ce sont des valeurs d'enum (`LogicalTypeID::STRING`,
  `PhysicalTypeID::INT64`, `case LogicalTypeID::NODE:`) reliées par le nom à la fonction statique
  `LogicalType::STRING()`, ou à un homonyme. Toutes fausses. 37 arêtes passent à « type »
  (`&Foo::initSharedState`).
- **rag3weaver/src : 646.**
  - 373 chemins par un type : `Side::Missing`, `Flow::Stop`, `BurnRole::Embedder` relié au trait
    `Embedder`, `FinishReason::ToolCall` relié à la struct `ToolCall`. Les 49 marquées « import » en
    font partie. Toutes fausses.
  - 273 méthodes std sur une locale sans type lu : `join` 95, `starts_with` 67, `lines` 38, `recv` 23,
    `keys` 14, `max` 12, `wait_timeout` 11, `metadata` 4…

**Le prix** : une méthode du projet homonyme d'une méthode std perd son lien entre fichiers, et dans le
fichier quand c'est le rendez-vous qui le posait. Sur les 273, une vingtaine au plus sont justes :
`catalog.drain()` (3), `files.append(…)` (2), `back.description()`, `outil.bind(…)`, une partie des
`wait_timeout` (le `Condvar::wait_timeout` est std, celui de `node.rs` non). Le ticket
[des receveurs restent sans type](../../codeparsers/tickets/2026-10-04-des-receveurs-restent-sans-type.md)
les rend en lisant le type de la locale.

Le 15 novembre au soir, le ticket des receveurs (codeparsers `65b1a93`) en a rendu une partie : `files.append`
(2), les `c.wait_timeout` des tests, `back.description()`. Restent `catalog.drain()` (un motif de tuple) et
`outil.bind(…)` (une locale redéclarée), écrits dans ce ticket.

