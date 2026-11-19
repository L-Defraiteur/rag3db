```json
{
  "lanceur": {
    "parent": "30ad9a2eec4fd75c6b92dca58821c3f86c81574e",
    "tree": "extension/rag3weaver",
    "needs_lib": false,
    "witness": "catalog::retard::tests::the_writing_mark_is_per_layer_and_the_guard_clears_it",
    "test": "cargo test --lib -- --exact catalog::retard::tests::the_writing_mark_is_per_layer_and_the_guard_clears_it",
    "suites": [
      "./run_e2e.sh --test e2e_code_sync"
    ],
    "hints": [
      "src/catalog/retard.rs"
    ],
    "ours": "e475942260aca58f01c1ded5d4c127938fe04180"
  }
}
```

# Cas de campagne : la marque d'écriture repliée sur la cellule

Arbre principal, 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie). C'est un cas court, en
Rust, dans le crate rag3weaver. Le témoin n'a pas besoin de la lib rag3db à l'exécution.

## Le point de départ

- **Commit parent** : `30ad9a2eec` (18 novembre 2026, 14 h 41). Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : `extension/rag3weaver`. Le fichier en jeu est `src/catalog/retard.rs`.

## Le signalement, tel qu'il était

> Lucie, 18 novembre : un autre worktree ne doit pas voir « indexation en cours » quand le sien est à jour. Le registre
> du retard (`Lag`) est désormais tenu par cellule **et par couche** (`LagKey { cell, layer }`). Il lui faut une marque
> « un index écrit cette couche » : `Lag::set_writing(&LagKey, bool)`, `Lag::is_writing(&LagKey)`, et une garde,
> `Lag::writing_guard(LagKey)`, qui l'efface en tombant, quelle que soit la sortie de l'index. Un index qui écrit la
> couche `a` ne doit marquer ni la couche `b` de la même cellule, ni la couche du backend.

## Le témoin

- **Filtre** : `catalog::retard::tests::the_writing_mark_is_per_layer_and_the_guard_clears_it`, dans
  `extension/rag3weaver/src/catalog/retard.rs`. Il est identique sur master (vérifié le 19 nov. sur 63e24ff7a3+).
- **Suite** : `cargo test --lib` du crate `extension/rag3weaver`, avec les features de la passe.
- **Rouge au parent** : l'assertion « layer b is marked while only layer a is written » échoue. Au parent, la marque
  est repliée sur la couche du backend de la cellule, et `is_writing` répond pour toute couche de la cellule.
- **Suites exigées par les chemins** : `e2e_code_sync`, pour `src/catalog/*` (`tools/poste/required_suites.txt`). Elle demande la lib rag3db ; le témoin, non.

## Les indices présents dans l'arbre

- `LagKey` et `LayerId` sont déjà là, avec `note_in` et `done_in` par couche.
- Les deux méthodes portent un commentaire `RED STUB` qui dit ce qu'elles font de faux.

## Notre correctif (caché à Algareth, pour le juge)

`e475942260`, « feat(views): a view knows its own index state — views::view_state reads the lag and the writing mark
of its layer, never another's ; the index marks the layer it writes » :

- `src/catalog/retard.rs` +3 −6 ;
- `src/views.rs` +1 −3 ;
- `src/dataflow/index_nodes.rs` +9.

Pour le témoin seul, `retard.rs` suffit : `set_writing` garde la clé telle quelle, et `is_writing` lit `contains(key)`.
