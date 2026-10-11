# Une transition faite par une réaction n'applique pas les écritures que le cycle de vie déclare

- **état** : ouvert (11 octobre 2026, lot review_anchored en une passe)
- **gravité** : réponse fausse (une date de transition et une révision manquent)
- **atteignable en service** : oui, dès qu'une réaction transitionne (gabarit `memory`, `review_anchored`)
- **touche rag3weaver** : oui (et seulement)

## Ce que c'est

Le gabarit `memory` déclare, à côté de son cycle de vie, les écritures qui vont avec une
transition (`backend.json`, `writes`) : `updated_at`, `revision`, et
`transition_dates [{field: state, to: to_review, timestamp_field: reviewed_at}]`.
`EntityRecordNode` (les verbes `put_*`) les applique. `ReactTransitionNode`, lui, n'écrit
que le champ d'état, par `Catalog::update_many` (avant ce lot, `Catalog::update` par
fiche) : une fiche passée « à revoir » par la réaction a `reviewed_at` nul, `updated_at`
inchangé, `revision` inchangée. `e2e_reaction_montee` le constate sans le dire
(`expected_revision: 1` après la transition).

Un agent qui trie ses fiches à revoir par date de passage, ou qui relit la révision pour
écrire sans écraser, se trompe.

## Ce qu'il faut pour le fermer

La politique d'écriture du backend (celle qu'`EntityRecordNode` applique) appliquée aussi
par la réaction, au même endroit pour les deux : une seule fonction qui, pour une ligne et
une transition, rend les champs à poser. Elle reste groupée : les C fiches gardent un drain
et un `SET` par forme. Témoin : après la réaction, `reviewed_at` posé, `revision` montée,
`updated_at` avancé, sur chaque fiche transitionnée.
