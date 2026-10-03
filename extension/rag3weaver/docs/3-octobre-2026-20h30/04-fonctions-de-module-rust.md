# Les fonctions d'un module Rust deviennent des scopes

3 octobre 2026, codeparsers `1f9d653`. Trouvé en préparant la marque de test :
l'extracteur Rust ne prenait une fonction libre que sans parent, pour écarter
les fonctions imbriquées dans un corps, et écartait du même coup **toutes les
fonctions d'un `mod`** — dont les 348 `#[test]` de `src/dataflow`, dont le
corps n'existait que dans le scope du module `tests`. C'est un défaut
d'extraction pour la recherche entière ; il part seul, avant la marque de
test, pour que son effet soit attribuable.

## Mesure sur `src/dataflow` (contre `9a1fae3`)

Par `rag3weaver::code::analyze`, la même analyse qu'à l'ingestion.

| | Avant | Après |
|---|---|---|
| Scopes | 1 886 | 2 301 (+415 fonctions de module) |
| Texte propre, total | 1 317 568 o | 1 315 750 o |
| Texte propre des modules (`namespace`) | 315 554 o | 55 813 o |
| Texte propre des fonctions | 167 185 o | 425 108 o |
| Relations | 15 419 | 20 049 |
| `CONSUMES` depuis des fonctions | 469 | 2 114 |
| `CONSUMES` depuis des modules | 530 | 534 |
| Rendez-vous (`MENTIONS`) | 20 799 | 24 560 |

**Pas de texte en double.** Le texte propre d'un scope remplace chaque enfant
direct par sa ligne de signature (décidé le 6 septembre pour `impl` /
méthode) ; ce calcul est fait par contenance, donc vaut tel quel pour
`mod` / fonction : le module ne garde que les signatures de ses fonctions,
et le total baisse un peu. Rien n'est découpé ni embarqué deux fois.

**Les arêtes, comme pour `impl` / méthode.** Le module garde les références de
ses fonctions (530 → 534 `CONSUMES`), comme une classe garde celles de ses
méthodes aujourd'hui ; les fonctions portent les leurs en plus. C'est
l'attribution existante, pas une nouvelle.

## Ce que cela change pour les autres

- `e2e_code` et le banc de recherche indexent ce corpus : leurs comptes
  bougent (des scopes et des relations en plus), et les références du banc
  d'avant ne sont plus comparables.
- Des centaines de fonctions de test deviennent cherchables ; la marque de
  test (lot suivant) permettra de les peser moins.
