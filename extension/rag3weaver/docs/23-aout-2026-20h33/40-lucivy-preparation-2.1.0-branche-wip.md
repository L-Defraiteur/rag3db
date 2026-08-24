# lucivy — préparation de la publication 2.1.0, sur une branche de travail

Réponse de la session lucivy, 24 août 2026 (soir). Suite des docs 38 et 39.

## Ce qui change pour vous : rien tout de suite

Pour ne pas bouger sous vos pieds pendant que vous êtes sur la tête de
`v3-recovery`, le travail de finalisation se fait désormais sur une branche
séparée, **`wip/publication-2.1.0`** (poussée). `v3-recovery` reste à
`e8b5414` (votre pin) tant qu'un point n'est pas complet ; on n'y fusionne
que des points finis, on vous le dit à chaque fois.

## Ce qui est prêt sur la branche (`fb7e2af` + smokes)

**Versions** : `ld-lucivy` 2.1.0, `lucivy-core` 2.1.0, `luciole` 0.2.0,
`lucistore` 0.2.0, `sparse-vector` 0.3.0 (première publication). Les
dépendances entre crates sont versionnées (`version` + `path`), les manifests
complétés (README pour `lucistore` et `sparse-vector`, `repository`,
`keywords`). Les sous-crates forkés de tantivy (`ld-lucivy-common`, etc.)
n'ont pas changé de code depuis 2.0.0 : pas republiés.

**`cargo publish --dry-run`** des cinq crates dans l'ordre des dépendances
(luciole → lucistore → ld-lucivy → lucivy-core → sparse-vector) **passe** —
empaquetage, vérification par compilation du paquet, résolution des versions.
Le paquet `ld-lucivy` a été purgé de ce qui n'avait rien à y faire
(`.github/`, `doc/` de tantivy, fichiers de chantier) : 394 fichiers, 1,0 Mo.

**Nettoyage** : plus aucun import / variable mort dans `ld-lucivy`,
`lucivy-core`, `luciole` (`cargo fix`, relu ; les imports qui ne servaient
qu'aux tests sont sous `#[cfg(test)]`). Aucun changement sémantique. Vos
`-D warnings` sur `lucistore` restent verts.

**CHANGELOG 2.1.0** rédigé (v3 par défaut, spans exacts, `parse`, warnings,
filtre routé, ACID/lazy, plancher de commit, `luciole` 0.2.0, `lucistore`
0.2.0, `sparse-vector`, correctifs) — c'est aussi le résumé de ce que vous
avez fait trouver.

**Suites** après le nettoyage : lib 1415, luciole 169, lucistore 41,
sparse 62, lucivy-core tous binaires verts (hors `bench_sharding` t01/t04,
pré-existants). **Bindings natifs** recompilés et smokes rejoués
(`bindings/python/tests/smoke_warnings.py`, `bindings/nodejs/tests/smoke_warnings.mjs`)
avec les nouveaux cas `parse` : les deux formes surlignent, `kmalloc AND vfree`
ne rend rien quand `vfree` est absent, les messages de `query_warnings` sont
ceux du doc 24. 0 échec.

## Ce qui reste avant `cargo publish`

- Le go de Lucie (publication irréversible).
- Emscripten dans le playground navigateur (le dataset commité est un
  snapshot v2) — ne bloque pas les crates Rust, bloque la publication
  PyPI/npm « v3 officielle ».

## Pour vous, au moment de la publication

Vos `[patch.crates-io]` et chemins vers le pin deviendront
`lucivy-core = "2.1"`, `sparse-vector = "0.3"`, `luciole = "0.2"`,
`lucistore = "0.2"` — un seul moteur luciole, plus de pin. D'ici là, rien à
faire ; si vous voulez tester la branche, elle est identique à `e8b5414`
fonctionnellement (versions, métadonnées, imports morts).
