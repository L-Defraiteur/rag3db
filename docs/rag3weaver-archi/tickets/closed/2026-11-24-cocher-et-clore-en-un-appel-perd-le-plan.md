# Cocher la dernière tâche et clore le plan en un appel perd le plan

- **État** : fermé le 28 novembre 2026 par le lot `edit-plan-item-before-close` (session rag3weaver-archi). Voir « Fermeture ».
- **Famille** : produit
- **Gravité** : moyenne. Chaque tour qui clôt un plan perd trois appels au modèle (et leur temps), et le plan clos
  est rangé pendant qu'il reste actif sur disque.
- **Atteignable en service** : oui. Vu dans trois tirages de la mesure du 24 novembre (04 h 35, 04 h 44, 06 h 39),
  à chaque fois sur le même enchaînement.
- **Touche rag3weaver** : oui (`edit_plan` de l'équipier, `daemon_host/teammate.rs`)
- **Pour** : archi
- **Signalé par** : c7, sur les journaux de la mesure du 24 novembre

## Ce qui s'est passé

Le modèle termine un plan en un seul appel : `{"close": {"outcome": "accomplished", …}, "item": {"id": …, "status":
"done"}}`. `edit_plan` traite les clés dans un ordre fixe, et `close` passe avant `item` :

1. `close` retire le plan actif et le range (`keep_closed`, la liste `closed` qui part vers la mémoire) ;
2. `item` ne trouve plus de plan actif et refuse : « item : il n'y a pas de plan actif » ;
3. l'appel échoue, `plan.json` n'est pas réécrit : le plan est encore actif sur disque, alors qu'il est déjà rangé.

Le modèle enchaîne alors `resume` (refusé : « un plan est déjà actif »), puis `item` seul, puis `close` seul, qui
range le plan une seconde fois.

## Fermeture

Le plan que l'appel clôt est gardé de côté le temps de l'appel. Un `item` ou un `remove` du même appel s'applique à
lui quand il n'y a plus de plan actif, et il n'est rangé qu'une fois tout l'appel validé : un appel refusé ne clôt
rien. Les deux témoins (`teammate/edit_plan_in_one_call_tests.rs`) passent par `ours`, la porte de chaque appel
d'outil du modèle.
