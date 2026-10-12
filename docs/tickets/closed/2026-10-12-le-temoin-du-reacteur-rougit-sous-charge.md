# Le témoin du réacteur `each_runs_per_ring_and_debounce_waits_for_quiet` rougit sous charge

- **État** : corrigé (12 octobre 2026, session recherche : des attentes par condition)
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

## Le correctif

Deux durées fixes, deux courses :
- `quiet == 0` juste après que `each` a vu les deux messages : sous charge,
  `each` peut traîner plus de 30 ms, et l'anti-rebond a déjà tiré (un rouge
  dans l'autre sens, pas encore vu) ;
- 50 ms de sommeil, puis `quiet == 2` : sous charge, le fil du réacteur n'a pas
  encore tourné (le rouge vu).

Le témoin attend désormais par condition, borné à 5 s : `each` a vu les deux,
et `quiet` s'est réveillé. Il vérifie ensuite ce que l'anti-rebond promet,
sur un instant porté par le réveil lui-même, et non sur la durée d'un
sommeil :
- le premier réveil porte les **deux** événements ;
- il vient **au moins 30 ms après le dernier envoi**.

Ces deux vérités tiennent sous n'importe quelle charge.
