# `view` dans les outils : ce qu'une vue lit, et ce que la réponse en dit

Session recherche, 16 novembre 2026. C'est la part de recherche dans 2b-C, en pile sur 2b′
d'arbre principal (`docs/arbre-principal/15-11-2026/01-2b-prime-le-chargeur-qui-compose-la-pile.md`).
La vue courante choisie par le contexte, la part (2) d'arbre, viendra ensuite.

## Ce que l'appel reçoit

Tout outil qui lit accepte `view: {"layer": <id ou nom de couche>, "rank"?: <génération>}`.
- **Où c'est lu** : une fois, dans `call_tool_on` (`backend.rs`), et jamais comme paramètre d'un graphe.
- **Où c'est annoncé** : dans le schéma (`tools/list`) des outils qui n'écrivent pas de fichier et ne
  lancent pas de commande.
- **`layer`** se lit d'abord comme un id de couche. Inconnu sous cet id, il se lit comme un nom, par
  l'index `couche_nom:<dépôt>:<nom>` des suivies, dans l'instantané de l'appel.
- **`rank`** omis vaut la dernière génération **publiée** de la couche.

## Ce qui se refuse

Chaque refus est nommé, et rien n'est lu hors de la vue demandée.
- **Une vue inconnue** : couche inconnue, rang absent, parente manquante, cycle ou profondeur au-delà
  de `tracked.views.max_depth`. Elle se refuse avant que le graphe ne lise.
- **Une vue sans `layer`, ou avec une clé inconnue**, se refuse aussi.
- **Un outil qui ne lit pas dans un instantané** (il écrit, ou un de ses nœuds n'y est pas sûr) refuse
  `view` en se nommant, avant toute exécution.

## Comment la vue se lit

La pile de la vue se charge par `vues::charger_la_pile_sur`, **sur la connexion de l'instantané de
l'appel**. La vue et ses lignes viennent donc de la même lecture, sans le mutex du catalogue. Elle compose
ensuite la visibilité de l'appel (`Visibilite::avec_la_pile`). Une pile d'un étage sur `backend` rend
la même réponse qu'avant ; banc l'a jugé au bit près.

## Ce que la réponse dit

```json
"view": {"layer": "backend", "rank": 0, "depth": 1, "pending": 1, "pending_rows": true}
```

- `rank` : la génération lue ; `depth` : le nombre d'étages de la pile.
- `pending` : le rang en attente de la couche, `rank + 1`.
- `pending_rows` : s'il existe des lignes de cette couche posées ou retirées après `rank`.

**À savoir aujourd'hui** :
- **Sans vue, l'appel lit l'index vivant**, c'est-à-dire le rang en attente w = n + 1, où l'écrivain
  écrit.
- **`view: {layer: backend}` lit le rang publié n.** Or rien du produit ne publie encore une génération :
  la publication à chaque fin d'index viendra avec 2a′-3. n reste donc 0, et une vue sans rang lit l'état
  de la bascule.
- **`pending_rows: true` dit cet écart.** Les lignes écrites depuis existent, mais la vue ne les lit
  pas : pour l'état vivant, il faut appeler sans `view`.

## Les témoins (`backend.rs`, module `tests`)

- `a_backend_view_gives_the_same_answer_and_reports_the_view_read` : sous la vue du backend, la même
  réponse au bit près, et la vue lue dite. Il ne vaut que tant que rien n'est écrit au rang en attente.
- `an_unknown_view_is_refused_before_any_read` : une vue inconnue, ou sans couche, est refusée.
- `a_two_layer_view_hides_what_the_child_removes` : un enfant posé à la main, qui retire `outils.rs`, le
  masque à `usages`. Son témoin du témoin a été fait : la composition remplacée par la visibilité
  d'ouverture, il rougit.
- `a_view_reports_the_pending_rank_and_whether_it_holds_rows` : un fichier écrit après l'index se voit
  sans vue, mais pas sous `view: backend`. La réponse dit `pending` et `pending_rows: true`.
