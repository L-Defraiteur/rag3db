# L'attente d'index d'un pas : synchrone sous six secondes, une notification au-dessus

15 novembre 2026, soir. Point 7 de la
[vision du 15 novembre](../../orchestration/visions/15-11-2026-l-agent-est-un-agent-de-code-qui-gere-ses-worktrees.md).
Lucie : « pas le re-notifier à chaque petit changement qu'il cause lui-même ; quand il fait une
recherche, on estime si l'attente d'indexation dépasse 6 secondes ; si oui il est notifié et reçoit
que les index ne sont pas prêts ; si non retour immédiat, on attend en synchrone » (c7).

Page avant le code. L'estimation est côté arbre et recherche (le journal d'index : ce qui reste, le
débit observé) ; la part d'archi est le pas.

## La conception, en quatre lignes

1. **Sous le seuil, la recherche attend en silence.** Un outil de recherche appelé dans un pas lit
   l'estimation de l'index de la tâche. Si elle est au plus `search.index_wait_s` (6 par défaut,
   réglable au manifeste), l'appel attend l'index en synchrone, puis répond complet, sans un mot sur
   l'attente.
2. **Au-dessus, une réponse et une seule notification.** L'appel répond tout de suite, sur ce qui
   est déjà indexé, et le dit : « index pas prêt, environ N s ». L'hôte écrit ensuite une seule
   ligne dans la boîte de la tâche, de l'hôte, quand l'index est prêt. Le pas suivant la voit à sa
   frontière ; d'autres recherches sur le même retard n'en ajoutent pas une seconde.
3. **Rien pour ses propres écritures.** Les réindexations que les écritures du pas provoquent (le
   socle, `edit_file`, `run_command`) ne notifient rien. L'équipier sait ce qu'il vient d'écrire,
   et l'index se rattrape sous le seuil. Seul un retard qu'il n'a pas causé, ou qui dépasse le
   seuil, se dit, une fois.
4. **Rouge d'abord, sur le faux modèle et une estimation fournie par le témoin.**
   - Une estimation de 2 s : l'appel attend et répond complet, et la boîte n'a rien de plus.
   - Une estimation de 20 s : l'appel répond « index pas prêt, environ 20 s », puis une seule ligne
     de l'hôte arrive à l'index prêt.
   - Trois écritures du pas, puis une recherche : aucune notification.

## Ce qu'il faut d'arbre et de recherche

Une estimation lisible par l'hôte, pour l'index d'une tâche : le reste à indexer et le débit
observé, en secondes, tirés du journal d'index et non d'une constante. Plus une attente bornée (« prêt,
ou au plus N s »). Je pose le pas sur ce contrat dès qu'il existe.
