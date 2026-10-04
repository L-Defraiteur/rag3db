# Un appel par chemin vers un type externe prend rendez-vous avec un homonyme du projet

- **État** : ouvert — non revérifié depuis le résolveur unique (63d154730)
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
