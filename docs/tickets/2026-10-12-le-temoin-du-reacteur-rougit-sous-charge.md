# Le témoin du réacteur `each_runs_per_ring_and_debounce_waits_for_quiet` rougit sous charge

- **État** : ouvert
- **Gravité** : test instable (un rouge sans défaut du code)
- **Atteignable en service** : non (le test seul)
- **Touche rag3weaver** : oui (`src/dataflow/reactor.rs`, ses tests)
- **Ouvert le** : 12 octobre 2026, session everything declarative, pendant le lot des jetons
- **Pour** : la recherche (porteur du réacteur)

## Ce qui est vu

Dans la lib entière (1 571 tests en parallèle, sous `poste lourd`), une fois :

```
dataflow::reactor::tests::each_runs_per_ring_and_debounce_waits_for_quiet ... FAILED
assertion `left == right` failed: un seul réveil pour les deux, après 30 ms de calme
  left: 0
 right: 2
```

Joué seul, le même test passe trois fois sur trois, sur le même arbre. Le lot qui tournait
(`TokenEvents`, `Spot`, `Event::Token`) ne touche pas au réacteur.

## Ce qu'il faut pour le fermer

Le test attend un réveil après 30 ms de calme : sous la charge des témoins voisins, l'attente
qu'il se donne ne suffit pas. Il faut une attente par condition (le réveil observé), bornée
largement, plutôt qu'une durée fixe.
