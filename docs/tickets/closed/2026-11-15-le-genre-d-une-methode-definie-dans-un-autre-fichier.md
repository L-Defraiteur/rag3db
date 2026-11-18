# Le genre d'une méthode définie dans un autre fichier que sa classe reste « function »

- **État** : fermé (18 novembre 2026) — la part d'ED, `docs/tickets/closed/2026-11-16-l-ecrivain-decide-le-genre-d-une-methode-definie-dans-un-autre-fichier.md`, porte le correctif et son témoin.
- **Famille** : produit
- **Gravité** : basse. Le genre est faux d'un mot ; la navigation ne s'appuie pas sur lui (elle lit
  `parent_name`), mais l'arbre compact montre une fonction là où il y a une méthode.
- **Atteignable en service** : oui, sur tout dépôt C++ où les méthodes sont définies dans un .cpp et leur
  classe déclarée dans un .h
- **Touche rag3weaver** : oui (`code.rs`, le rendez-vous des parents d'ailleurs ; l'écrivain des générations)
- **Pour** : ED (everything declarative) et archi (rag3weaver archi)
- **Signalé par** : session codeparsers, au lot de l'appartenance des méthodes, avec l'orchestration

## Ce qu'on voit

`void Foo::f() {}` arrive de codeparsers en `function`, puisque `kz::f()` (une fonction d'un namespace)
a la même forme. Depuis ce lot, `code.rs` en fait une `method` quand le parent résolu **dans le fichier**
est une classe. Quand la classe est dans un autre fichier, le parent se résout au rendez-vous, après la
publication de la ligne, et la ligne reste `function`.

## Pourquoi ce n'est pas fait

Réécrire au rendez-vous le `scope_type` d'une ligne publiée ferait naître une version hors de l'écrivain
des générations : `scope_type` est un champ de version (`SCOPE_VERSION_FIELDS`).

## Ce qu'il faudrait

L'orchestration, le 15 novembre : « le genre d'une méthode définie dans un autre fichier se décide à la
publication, par l'écrivain, ou se lit par la navigation sans le genre ».
