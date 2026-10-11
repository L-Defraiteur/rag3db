# Un appel par chemin vers un type externe prend rendez-vous avec un homonyme du projet

- **État** : corrigé — codeparsers `f91fc58` (11 oct. 2026) ; témoin `e2e_usages::un_appel_par_chemin_vers_un_type_externe_ne_prend_pas_rendez_vous`
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (la voie des rendez-vous, `code.rs`)

## Ce que c'est

`Tokenizer::from_file(...)` — `Tokenizer` vient d'une crate externe — est
relié à la seule fonction `from_file` du projet (`gcp_auth.rs`) : vu par la
section Liens sur les `from_bytes` des embarqueurs (le 3 octobre).

## Recette minimale

```rust
// a.rs
use tokenizers::Tokenizer;
pub fn charge() { let _ = Tokenizer::from_file("t.json"); }
// b.rs
pub fn from_file(p: &str) -> u32 { p.len() as u32 }
```

Attendu : aucune arête `charge → from_file`.

## Cause

Le rendez-vous résout par définisseur unique sans regarder le qualificatif
(`Tokenizer`) : ni le type, ni son import (`import_origin` le dit
externe).

## Pour le fermer

Côté rendez-vous : un qualificatif de chemin qui nomme un type importé
d'une crate externe (ou absent du projet) s'abstient. Mesurer au banc des
relations.

## Correction (11 octobre 2026)

codeparsers `f91fc58` : le qualificatif d'un appel par chemin qui finit par
un nom de type devient le type du receveur (`qualifier_type = Tokenizer`).
`choose_target` ne relie un receveur typé qu'à une méthode de ce type : le
projet n'en a pas, aucune arête. Témoin (la recette ci-dessus) :
`e2e_usages::un_appel_par_chemin_vers_un_type_externe_ne_prend_pas_rendez_vous`,
rouge avec codeparsers `1fc380a` (une arête `charge → from_file`), vert
après. Sur rag3weaver, 330 faux liens de cette forme écartés au banc
(types externes et variantes d'enum).
