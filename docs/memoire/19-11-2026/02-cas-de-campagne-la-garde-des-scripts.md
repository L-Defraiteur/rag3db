# Cas de campagne : la garde des scripts prend « description » pour un script

```json
{
  "parent": "f64525645beee1104d214a6b69e3fb8031332fdb",
  "tree": "extension/rag3weaver",
  "paths": [
    "extension/rag3weaver",
    "tools/rust_api"
  ],
  "submodules": {
    "extension/rag3weaver/codeparsers": "bd133850f89d6b12a3ca2b2f6ce238785ed37e3a"
  },
  "needs_lib": false,
  "witness": "dataflow::foreach_node::tests::substituted_description_key_is_not_taken_for_a_script",
  "test": "cargo test -j 8 --lib -- --exact dataflow::foreach_node::tests::substituted_description_key_is_not_taken_for_a_script",
  "suites": [
    "cargo test -j 8 --lib"
  ],
  "hints": [
    "extension/rag3weaver/src/dataflow/foreach_node.rs"
  ],
  "ours": "b544095c9ce3e7cb5cb134fe37534cd86c44b686"
}
```

Le bloc est celui du lanceur (forme d'archi). `paths` porte `tools/rust_api` parce que le crate déclare la lib
rag3db par un chemin hors de son dossier (`../../tools/rust_api`, optionnelle) : cargo lit son manifeste même
quand la dépendance n'est pas prise. `submodules` donne le sous-module `codeparsers` à son commit du parent, pour
la même raison ; `git archive` ne l'extrait pas. Sans les features (le défaut du crate est vide), le cas ne
demande pas la lib rag3db.

**Vérifié le 19 novembre à luciepc**, sur l'arbre archivé au parent avec le sous-module extrait, dans un target
propre : le témoin est rouge au parent (`0 passed; 1 failed`), vert avec notre correctif (`1 passed`).

Session mémoire (96), 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie ; lanceur
d'archi, juge du banc). C'est un cas court, en Rust, dans le dataflow du crate.

## Le point de départ

- **Commit parent** : `f64525645b` (18 novembre 2026). Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : `extension/rag3weaver`, avec `tools/rust_api` et le sous-module `codeparsers` à son commit.

## Le signalement, tel qu'il était

> La garde des scripts prend la clé « description » pour un script.
>
> « le nœud « genre » substitue $groupe dans sa clé « description » — un script lit l'élément par son entrée,
> jamais par substitution ». La garde est juste dans son principe : un texte de document substitué dans du code
> serait exécuté. Mais elle reconnaît une clé de script par `k.contains("script")`, et « description » contient
> « script ».
>
> Gravité : refus à tort. Un gabarit juste est refusé au chargement. Tout graphe cité qui substitue `$element`
> dans une clé contenant « script » (`description`, `transcript`…) le rencontre. Un `CreateRefTypeNode` qui
> substitue `$element.x` dans sa `description`, dans un graphe cité, doit se charger.

## Le témoin

- **Filtre** : `dataflow::foreach_node::tests::substituted_description_key_is_not_taken_for_a_script`, dans
  `extension/rag3weaver/src/dataflow/foreach_node.rs`. Identique sur master.
- **Rouge au parent** : la garde refuse un graphe cité dont un nœud substitue `$element` dans sa clé
  `description`.
- **Suites exigées par les chemins** : aucune au parent (`tools/poste/required_suites.txt` n'y nomme que
  `e2e_code_sync`, pour d'autres chemins). La lib du crate entière, sans features.

## Notre correctif (caché à Algareth, pour le juge)

`b544095c9c`, « fix(rag3weaver): la garde des scripts d'un graphe cité ne lit que la clé script — description
n'est plus prise pour un script » : `extension/rag3weaver/src/dataflow/foreach_node.rs`, +2 −1.
