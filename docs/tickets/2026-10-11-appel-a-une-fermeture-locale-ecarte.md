# L'appel à une fermeture locale est écarté, une fonction passée en argument est un « other »

- **État** : ouvert (T5 du B9 Rust, session recherche, 11 oct. 2026)
- **Gravité** : réponse incomplète (faible)
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (impact ne traverse pas une fermeture locale ; `map(aide)` n'est pas un appel)

## Ce que c'est

Dans `let double = |x| x * 2; … double(x)`, la fermeture a bien son scope
(`Lambda double`), mais l'appel `double(x)` n'est relevé nulle part ; et
`map(aide)` relie `aide` en `other`, pas en `call`.

## Recette minimale

```rust
pub fn aide(x: i32) -> i32 { x + 1 }
pub fn applique() -> i32 {
    let double = |x: i32| x * 2;
    let v: Vec<i32> = vec![1, 2].into_iter().map(aide).collect();
    v.into_iter().map(|x| double(x)).sum()
}
```

Attendu : `applique → aide` en `call` ; l'appel `double(x)` rattaché à la
fermeture `double`. Obtenu : `other: applique → aide` ; rien pour `double(x)`.

## Témoin

Pas encore écrit : le cas de la session recherche (`fermetures.rs`) en est
la forme, à poser dans un test codeparsers comme `tests/boucles_rust.rs`.

## Cause

`double` est une locale (`let`) : la règle « une variable liée cache le nom
nu » écarte l'appel. `usage_of` ne sait pas qu'un nom passé en argument
désigne une fonction (il ne connaît que la forme de l'arbre).

## Pour le fermer

- Un appel à une locale liée à une fermeture qui a son scope : le relier à
  ce scope (dans le fichier, marque « fichier »).
- Un nom passé en argument qui se résout vers une fonction : `call` plutôt
  qu'`other` — au résolveur, qui sait ce que le nom désigne.
