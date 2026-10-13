# LlmNode et Decide ne voient pas l'annulation sous l'exécuteur par niveaux

- **État** : corrigé (12 octobre 2026, session recherche, dans le lot de l'annulation sous tous les exécuteurs, page `extension/rag3weaver/docs/12-octobre-2026-recherche/05-l-annulation-sous-tous-les-executeurs.md`)
- **Gravité** : coût : le modèle génère jusqu'au bout après l'arrêt du run
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`src/dataflow`)
- **Ouvert le** : 12 octobre 2026, session recherche, par le relevé de la page 05 ; ouvert et fermé dans le même commit

## Ce que c'était

`EmitSink` (le puits de `LlmNode`) et `CancellableSink` (celui de `Decide`) ne s'arrêtaient que sur une annulation vue par le jeton. Sous les niveaux, `emit` sans canal jette l'élément et rend `Ok` : le puits continuait.

## La cause

Le jeton d'annulation d'un nœud n'était fait que du hub de sa liaison de flux, absente sous l'exécuteur par niveaux : `on_cancel` n'y faisait rien et `is_cancelled` y restait faux.

## Le témoin

`annulation_tests.rs`, `an_llm_node_stops_generating_under_levels` : un modèle lent de 100 fragments s'arrête désormais avant la fin. `EmitSink::on_token` regarde le jeton d'abord.
