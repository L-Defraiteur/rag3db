# `new` est un mot vide de la liste commune : en Rust, aucun appel `T::new()` n'est relevé

- **État** : ouvert (11 octobre 2026) — trouvé en écrivant les témoins du nom propre (session codeparsers)
- **Gravité** : réponse fausse (des appels manquent : `usages` d'un constructeur `new` rend trop peu)
- **Atteignable en service** : oui, sur tout dépôt Rust (et C++ pour `new` comme nom de fonction)
- **Touche rag3weaver** : oui, par codeparsers (les références des scopes)

## Ce que c'est

La liste commune des mots vides de codeparsers (`base_scope_extraction_parser.rs:36`, recopiée
en C, C++, C#) contient `new`, mot-clé de JavaScript, TypeScript, C++ et C#. Elle sert aussi au
Rust, où `new` n'est qu'un nom de fonction, le plus fréquent des constructeurs : `Inner::new()`
n'est jamais une référence, et `usages Inner::new` ne rend aucun appelant.

## Recette

`struct Inner; impl Inner { fn new() -> Self { Inner } } fn f() -> Inner { Inner::new() }` :
le scope `f` n'a aucune référence `new`.

## Ce qu'il faut pour le fermer

Des mots vides par langage : `new` reste vide là où c'est un mot-clé (JS, TS, C++, C#), pas en
Rust (ni en Go, Python). Un témoin par langage, puis le banc : en Rust, beaucoup de liens par le
type en plus (`T::new()` porte son type), à vérifier sur un échantillon.
