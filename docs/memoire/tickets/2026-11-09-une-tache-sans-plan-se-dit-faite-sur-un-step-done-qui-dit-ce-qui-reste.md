# Une tâche sans plan se dit faite sur un step_done qui dit ce qui reste

- **État** : ouvert, le 9 novembre 2026. Relevé par la session mémoire (96) pendant l'essai de
  l'équipier sur les README ; pour everything declarative (l'équipier par pas).
- **Famille** : produit
- **Gravité** : une tâche réelle s'arrête après son premier pas sans avoir rien livré.
- **Atteignable en service** : oui, par toute tâche dont l'équipier n'ouvre pas de plan.
- **Touche rag3weaver** : oui, la fin d'un pas dans `src/daemon_host/teammate.rs`.

## Ce qu'on voit

La request de `readme-nouveautes-novembre` demandait un report de verdicts, puis une question par
ajout, une réponse du superviseur à chaque fois, et « ne déclare pas la tâche finie avant ma
réponse au report final ». L'équipier n'a pas ouvert de plan. Son premier pas a fini sur :

    step_done { item: "Analyse et verdict des nouveautés de novembre 2026",
                done: "Analyse complète des 31 lignes…",
                remaining: "Validation du verdict et soumission des ajouts un par un pour accord avant modification.",
                succeeded: true }

L'hôte a aussitôt écrit `ended : fait sans plan` et arrêté la tâche : ni report, ni question, ni
proposition. La règle est dans `teammate.rs` (la fin se juge après le pas) : aucun plan vivant et
un `step_done` déclaré valent fin, quel que soit `remaining`.

## Ce qu'il faut

- Un `step_done` dont `remaining` n'est pas vide **n'est pas une fin**, plan ou pas : le pas
  suivant part.
- Une request qui demande un livrable dans la boîte (un report, une question, l'accord du
  superviseur) ne se clôt pas sans lui ; au plus, la fin se dit « faite sans le livrable
  demandé » et attend le superviseur.
- Témoin : sans plan, un `step_done` avec `remaining` relance un pas ; sans `remaining`, la fin
  reste « fait sans plan » (le cas de la petite demande, `teammate_tests.rs`).
