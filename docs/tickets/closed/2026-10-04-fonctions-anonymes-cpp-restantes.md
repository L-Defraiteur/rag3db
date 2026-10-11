# 738 définitions C++ restent anonymes

- **État** : corrigé — codeparsers `35faa0f` (11 oct. 2026, P4 de la stèle du produit) ; témoin `codeparsers/tests/macros_d_export_cpp.rs`
- **Gravité** : réponse fausse (un scope sans nom)
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (un nom de scope `AnonymousFunction`)

## Ce que c'est

La branche codeparsers `declarations` (`c3a3656`) a rendu leur nom aux
définitions qualifiées (`kz::Foo::qux`, `Foo::~Foo`) : 2 476 →
738 `AnonymousFunction` sur les 2 890 fichiers C/C++ du dépôt. Les 738
restants n'ont pas été regardés (opérateurs ? `template_function` ?
déclarateurs imbriqués dans des pointeurs de fonction ?).

## Témoin

`examples/sonde_declarations.rs` (codeparsers) — le compte, pas encore les
cas.

## Pour le fermer

Tirer cinquante cas, classer, corriger `extract_function_name` ; mesurer
le compte.

## Correction (11 octobre 2026)

Relevé sur rag3db (`src` et `test`, 1 669 fichiers, 21 329 scopes) : 287
anonymes (les lots du 10 et du 11 en avaient déjà nommé une part). La cause
majeure : les **classes exportées** — `class RAG3DB_API ExtraTypeInfo { … }`
se lisait, pour tree-sitter qui ne connaît pas la macro, comme une
définition de fonction, la classe et ses méthodes perdues. codeparsers
`35faa0f` masque la macro avant l'analyse (même longueur, les textes restent
l'original) et nomme les formes restantes : opérateur de conversion
(`operator int8_t`), spécialisation de gabarit de classe et de fonction,
`typedef struct / enum { … } Nom;` ; et une méthode gabarit d'une classe a
désormais son scope.
**Après : 0 anonyme** sur rag3db (le message du commit dit « 2 » : compté
avant le correctif du `typedef enum`, et un faux positif dont le nom contient
« Anonymous »). Banc : appels reliés entre fichiers 41,5 → 50,7 % sur
rag3db/src ; +198 classes, +1 362 méthodes.
