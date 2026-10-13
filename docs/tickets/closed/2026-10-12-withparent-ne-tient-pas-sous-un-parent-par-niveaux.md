# Un enfant lancé WithParent n'est pas annulé avec un parent par niveaux

- **État** : corrigé (12 octobre 2026, session recherche, dans le lot de l'annulation sous tous les exécuteurs, page `extension/rag3weaver/docs/12-octobre-2026-recherche/05-l-annulation-sous-tous-les-executeurs.md`)
- **Gravité** : blocage : un enfant déclaré lié survit à son parent
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`src/dataflow`)
- **Ouvert le** : 12 octobre 2026, session recherche, par le relevé de la page 05 ; ouvert et fermé dans le même commit

## Ce que c'était

`Tie::with_parent_of(ctx)` accroche l'annulation de l'enfant au jeton du parent. Sous un parent par niveaux, le jeton n'était jamais annulé.

## La cause

Le jeton d'annulation d'un nœud n'était fait que du hub de sa liaison de flux, absente sous l'exécuteur par niveaux : `on_cancel` n'y faisait rien et `is_cancelled` y restait faux.

## Le témoin

Le témoin du lien au parent (`background/tests.rs`, `a_child_tied_to_its_parent_is_cancelled_with_it`) passe sous l'exécuteur concurrent forcé ; avec le jeton armé, le même lien tient sous les niveaux (`annulation_tests.rs`, témoin 1, pour le jeton lui-même).
