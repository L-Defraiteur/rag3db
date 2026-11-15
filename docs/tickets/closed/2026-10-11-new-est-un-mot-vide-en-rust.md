# `new` est un mot vide de la liste commune : en Rust, aucun appel `T::new()` n'est relevé

- **État** : corrigé (15 novembre 2026) — codeparsers 87126e4 (`new-rust`) : `new` sort des mots vides de Rust, de Python et de C ; témoins `tests/new_n_est_pas_un_mot_vide.rs`, rouges sur le code d'avant
- **Gravité** : réponse fausse (des appels manquent : `usages` d'un constructeur `new` rend trop peu)
- **Atteignable en service** : oui, sur tout dépôt Rust (et C++ pour `new` comme nom de fonction)
- **Touche rag3weaver** : oui, par codeparsers (les références des scopes)
- **Famille** : produit

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

## La correction

`new` sort des listes de mots vides de Rust (`RUST_STOP_WORDS`), de Python et de C, qui le reprenaient de
JavaScript ; il reste vide en C++ et en C# (un mot-clé) et en Go (une fonction intégrée, `new(T)`).

**Le banc**, sur le code de rag3weaver (`extension/rag3weaver/src`, 288 fichiers Rust, résolution entre
fichiers), avant → après : références relevées 263 402 → 271 732 (+8 330, dont 9 685 `new`) ; relations
162 226 → 166 923 ; relations vers un scope `new` 542 → 2 894 (×5,3) ; scopes qui appellent un `new`
403 → 1 596. Sur un échantillon de 25 relations vers `new` tirées à pas régulier, les 20 `CONSUMES` visent
le `new` du bon type (`Agent::new` → agent.rs, `MockLlm::new` → llm.rs, `EventBus::new` → events.rs…).

Les autres mots de JavaScript que la liste Rust reprend ont leur ticket (basse gravité) :
[la liste des mots vides de Rust est une copie de celle de JavaScript](../../codeparsers/tickets/2026-11-15-la-liste-des-mots-vides-de-rust-est-une-copie-de-celle-de-javascript.md).
