# Un conteneur garde les références de ses membres

- **État** : corrigé dans codeparsers `766e4bd` — en attente du pointeur
- **Gravité** : réponse fausse (des arêtes conteneur → homonyme)
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`usages`, `impact`, liens)

## Ce que c'est

Un scope conteneur — `mod tests` en Rust, un `namespace` C++, une classe —
porte les références de tout ce qu'il contient, en plus de ses membres qui
les portent déjà : `tests → name` relie le module `tests` de
`dataflow/graph_tool.rs` à la méthode `GraphTool::name`, parce que des
fonctions de test y ont un paramètre nommé `name`.

## Recette minimale

```rust
pub struct T;
impl T { pub fn name(&self) -> u32 { 1 } }
#[cfg(test)]
mod tests {
    fn find(name: &str) -> bool { name.is_empty() }
}
```

`codeparsers` (fichier seul) : `CONSUMES tests → name`.

## Témoin

Pas encore de test rouge ; relevé sur `src/dataflow/graph_tool.rs` (sites
1825 à 2848 de `tests → name`).

## Cause

L'extraction des références d'un conteneur parcourt tout son nœud
(`extract_identifier_references` sur le `mod_item`, le `namespace`), sans
exclure ce que portent ses scopes enfants ; `include_child_refs: false` ne
joue qu'au résolveur, pour les classes.

## Pour le fermer

Retirer des références d'un conteneur celles qui tombent dans un scope
enfant (positions, comme `sans_noms_declares` en C++), mesuré : relations
retirées, banc des relations avant / après. Touche tous les langages : un
lot à lui.
