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

## Correction partielle en branche — 11 novembre

`cx-ed-reste-explicite-3`, source `51cf79843` : le dernier step_done depuis la dernière
request fournit une notion unique de reste explicite, relue depuis la boîte durable pour
continuer ou terminer. Une question ouverte garde priorité sur cette continuation.
Rouge initial sur `06083905c` : un seul pas au lieu de deux, fin « fait sans plan » malgré
remaining non vide (0,67 s). Première correction `0331e2246` insuffisante : la fin est empêchée
mais la boucle attend sans travail ; test arrêté, aucun vert revendiqué sur cette version.

Ciblés verts sur `51cf79843` : continuation et boîte déjà consommée 2/2 (0,72 s), petite
demande sans plan et pause/reprise du plan 1/1 (1,13 s). Le report est réellement observé,
et le reste est rappelé au modèle. La reprise testée restaure une boîte avec curseur déjà lu ;
ce n'est pas une preuve d'un arrêt forcé du processus à toutes les instructions possibles.
Preuves luciepc `I-luciepc/reste-explicite-20261111/`, rouge.log et vert-3-*.log.
Passe complète et Gemini encore à jouer, aucun push master de cette correction.

Le ticket reste ouvert : un remaining vide ne prouve pas l'existence du livrable demandé ni
l'accord explicite du superviseur. Le contrat structuré d'achèvement et sa preuve de parcours
restent à définir ; aucune autorisation nouvelle n'est déléguée au juge.
