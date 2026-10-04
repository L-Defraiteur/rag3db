# Le type d'un champ déclaré dans un autre fichier ne se lit pas

- **État** : ouvert
- **Gravité** : réponse fausse (une arête manque)
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (la voie des rendez-vous)

## Ce que c'est

Depuis le résolveur unique (codeparsers `d61b83a`), le type d'un champ ou
d'un retour (`self.store.get()`) se lit à l'analyse quand le champ est
déclaré **dans le même fichier**. Une `struct` déclarée dans `a.rs` dont
l'`impl` est dans `b.rs` laisse la référence sans `qualifier_type` : le
rendez-vous s'abstient sur un nom ambigu, l'arête manque.

## Recette minimale

```rust
// a.rs
pub struct Svc { pub store: Store }
// b.rs
impl Svc { pub fn run(&self) { self.store.get(); } }
// c.rs — deux `get`
pub struct Store; impl Store { pub fn get(&self) {} }
pub struct Other; impl Other { pub fn get(&self) {} }
```

Attendu : `run → Store::get`.

## Cause

La référence garde `qualifier_deferred: FieldOf { owner: Svc, field: store }`,
que seule la résolution entre fichiers savait lire ; le rendez-vous ne la
lit pas.

## Pour le fermer

Côté rendez-vous : lire `qualifier_deferred` — le type du champ `store` de
`Svc` est un fait du fichier de `Svc` (son membre `Property`), à porter sur
le Scope ou le Symbol. Mesurer au banc.
