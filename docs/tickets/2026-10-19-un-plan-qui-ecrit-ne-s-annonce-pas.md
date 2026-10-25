# Un Plan qui écrit ne s'annonce pas

- **État** : ouvert (19 octobre 2026, session mémoire longue ; détaché du ticket fermé « des
  écritures ne passent pas par le journal »)
- **Gravité** : réponse fausse en silence, pour qui écoute `catalog`.
- **Atteignable en service** : non. `Catalog::run_plan` n'a aucun appelant en service, seulement
  les tests de `src/plan.rs` et `tests/e2e_chainage.rs`. Le PlanNode de l'arbre principal ne fait
  que lire, par `run_read_plan`.
- **Touche rag3weaver** : oui, seulement.
- **Ouvert le** : 19 octobre 2026.

## Ce que c'est

`crate::plan::run_plan` exécute un graphe de formes de l'IR, écritures comprises. Il ouvre son
`BEGIN` sur la connexion (`src/plan.rs:317`), sans le catalogue. `Catalog::run_plan` prend
`&self`, donc il ne peut rien publier. Ce qu'un Plan écrit ne passe ni par le journal du drain
ni par `Catalog::publier`.

## Recette

Pas de Cypher : c'est l'absence d'un événement. Un Plan qui pose une ligne d'une entité
déclarée, et un abonné à `catalog` qui ne reçoit rien.

## Témoin

À écrire avec le premier appelant écrivant : le Plan, puis l'événement attendu (rien sous un
ROLLBACK).

## Ce qu'il faut pour le fermer

Une décision de forme, à prendre avec le premier appelant qui écrit :
- soit `Catalog::run_plan` passe en `&mut self`, ouvre sa transaction par le catalogue et
  annonce au COMMIT ce que ses formes d'écriture ont rendu ;
- soit les formes d'écriture d'un Plan rendent les uuids écrits, et le catalogue les publie.
