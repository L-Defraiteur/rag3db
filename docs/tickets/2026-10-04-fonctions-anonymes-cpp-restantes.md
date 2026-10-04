# 738 définitions C++ restent anonymes

- **État** : ouvert — cause non cherchée
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
