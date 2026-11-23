# Un appel sur un receveur d'un type externe se rattache par le nom

- **État** : ouvert (23 novembre 2026)
- **Famille** : produit
- **Gravité** : moyenne. `usages Catalog::update` rend huit appelants dans l'index de rag3weaver, dont sept sont des
  `hasher.update(…)` de blake3 (empreintes, `local_id`, `manifest_print`). Un agent qui cherche les appelants d'une
  méthode du projet en reçoit d'une bibliothèque, sans marque.
- **Atteignable en service** : oui. Tout appel de méthode sur une variable d'un type d'une dépendance, quand une méthode
  du projet porte le même nom (`update`, `new`, `get`, `insert`…).
- **Touche rag3weaver** : oui (le rendez-vous par le nom et sa matérialisation, `code.rs`)
- **Pour** : session codeparsers, avec le lot des relations fabriquées (une treizième : la règle est de rag3weaver)
- **Signalé par** : un relevé d'`usages` sur l'index de rag3weaver, le 23 novembre 2026

## Constat

`let mut h = blake3::Hasher::new(); h.update(…)` (`backend.rs:2740`, `daemon_host.rs:275`, `claude_code_hook.rs:1125`,
`builtin_templates.rs:42`…) : l'appel `update` prend un rendez-vous par le nom, et la matérialisation le relie à
`Catalog::update`, la seule méthode `update` du projet.

## Ce qu'il faut

- Le receveur dit son type quand il se lit (`Hasher`, construit par `blake3::Hasher::new()`), et la racine du chemin
  (`blake3`) est une dépendance que le dépôt déclare : l'appel est un membre d'une bibliothèque externe, et il ne prend
  pas de rendez-vous. C'est la règle du correctif des paquets (`batch-independence`), appliquée au type du receveur et
  non à un chemin qualifié.
- À vérifier d'abord : ce que porte le rendez-vous (`qualifier_types`, `deferred`), et si la matérialisation se rabat
  sur le seul nom quand le type lu n'est pas un type du projet.
- Témoin rouge : un dépôt Rust qui déclare `blake3`, une méthode `Catalog::update`, et une fonction
  `let mut h = blake3::Hasher::new(); h.update(b"x");`. `usages Catalog::update` ne rend pas cette fonction.
