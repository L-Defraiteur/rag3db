# Un run arrêté par son hôte ne dit pas pourquoi

- **État** : corrigé (12 octobre 2026, session recherche, dans le lot de l'annulation sous tous les exécuteurs, page `extension/rag3weaver/docs/12-octobre-2026-recherche/05-l-annulation-sous-tous-les-executeurs.md`)
- **Gravité** : diagnostic : la trace ne distingue pas un arrêt d'une fin
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`src/dataflow`)
- **Ouvert le** : 12 octobre 2026, session recherche, par le relevé de la page 05 ; ouvert et fermé dans le même commit

## Ce que c'était

`RunFinished.cause` n'était posée que par un run lancé en fond. Un run arrêté par sa poignée, par un échec sous le concurrent ou par un interblocage la laissait vide.

## La cause

La cause n'était connue que du registre des runs en fond ; le runtime la garde désormais (`last_stop`) pour `RunFinished`.

## Le témoin

`annulation_tests.rs`, `a_levels_run_stops_between_nodes_and_says_why` : `RunFinished.cause` vaut `requested`. Les causes sont désormais des identifiants (`requested`, `parent_finished`, `timeout`, `failed:<nœud>`, `deadlock`, `host_stopped`), avec `cancelled_by`.
