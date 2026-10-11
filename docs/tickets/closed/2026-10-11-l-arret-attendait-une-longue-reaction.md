# L'arrêt de l'hôte attendait la fin d'une longue réaction

- **état** : corrigé (le commit des trois défauts de la montre, 11 octobre 2026)
- **gravité** : blocage (vingt minutes d'arrêt, le journal de la base qui grossit)
- **atteignable en service** : oui — SIGTERM pendant une réaction qui retire des milliers de fichiers
- **touche rag3weaver** : oui (et seulement)

## Ce que c'est

Vu sur luciepc : SIGTERM à 17 h 58, l'hôte encore en train de s'arrêter à 18 h 05 ;
`stop_reactions` attendait la fin de `reindex_on_change`, qui retirait des milliers de
fichiers un par un.

## Le témoin

`backend::tests::stopping_cuts_a_long_reaction_and_the_restart_resumes_it` : 400 fichiers
retirés, l'arrêt rend la main en moins de 3 s, la réconciliation reprend le reste au
redémarrage.

## La correction

Un drapeau d'arrêt servi aux réactions (`REACTION_STOP_SERVICE`), levé par `stop_reactions`
après l'arrêt des montres ; `ReindexFileNode` le lit entre deux fichiers. Ce qui est fait
reste commité, le reste garde son retard, et la réconciliation le reprend.
