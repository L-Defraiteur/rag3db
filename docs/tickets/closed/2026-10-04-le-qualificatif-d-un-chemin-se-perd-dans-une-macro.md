# Le qualificatif d'un chemin se perd dans une macro

- **État** : corrigé dans codeparsers 5778d92 — en attente du pointeur (arbre principal)
- **Gravité** : réponse fausse (une arête juste est marquée « par le nom »)
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (marque de résolution ; `impact` ne peut pas filtrer)

## Ce que c'est

Dans les arguments d'une macro Rust, un appel par chemin perd son
qualificatif : `assert_eq!(crate::b::run(), 2)` rend une référence `run`
sans `qualifier` ni `import_origin`, quand `crate::b::run()` hors macro
rend le module `crate::b`. Le rendez-vous la relie par le seul nom, et la
marque dit `nom`. Les tests en sont pleins.

## Recette minimale

```rust
// c.rs
#[cfg(test)]
mod tests {
    #[test]
    fn dans_macro() { assert_eq!(crate::b::run(), 2); }
    fn hors_macro() -> u32 { crate::b::run() }
}
```

`analyze` : `pending_import_modules` porte `hors_macro run ["crate::b"]`,
rien pour `dans_macro`.

## Témoin

Aucun test permanent ; la recette ci-dessus en sonde jetable (4 octobre).
`tests/e2e_impact_fichier.rs::le_dehors_du_fichier_et_les_tests_qui_le_traversent`
échoue si le filtre d'arête est déclaré dans `impact_fichier.mmd`.

## Cause probable

tree-sitter-rust ne lit pas les arguments d'une macro comme des
expressions : un `token_tree`, où `crate`, `::`, `b`, `::`, `run` sont des
jetons. codeparsers y relève les identifiants, pas le chemin.

## Pour le fermer

Dans codeparsers, au relevé des identifiants d'un `token_tree` : une suite
`ident (:: ident)+` suivie de `(` est un appel par chemin — qualificatif =
les segments de tête. Test de la recette, puis la mesure du banc des
relations avec le filtre dans `impact`.

## Corrigé (4 octobre, soir)

codeparsers 5778d92 : le qualificatif se relit sur les voisins de gauche
dans les jetons d'une macro (`crate::b` pour `crate::b::run`, `o` pour
`o.f()`, `self.a` pour `self.a.f()`), la tête d'un chemin ne sort pas
seule ; type du qualificatif et origine d'import suivent. Témoin :
`codeparsers/tests/macros_rust.rs` — la même expression hors macro et dans
`assert!` rend les mêmes références. `e2e_impact_fichier` passe avec le
filtre d'arête dans son gabarit.
