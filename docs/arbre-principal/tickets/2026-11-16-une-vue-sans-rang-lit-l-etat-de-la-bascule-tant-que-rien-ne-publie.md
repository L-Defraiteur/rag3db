# Une vue sans rang lit l'état de la bascule tant que rien ne publie

- **État** : ouvert (16 novembre 2026), arbre principal ; à faire dans 2a′-3.
- **Famille** : produit
- **Gravité** : une recherche sous `view: {layer: backend}` est périmée en silence sur une base vivante. Elle lit
  la génération 0, l'état de la bascule, pendant que l'appel sans vue lit l'index vivant. La réponse le dit depuis
  2b-C (`view: {layer, rank, pending, pending_rows}`), mais l'écart reste.
- **Atteignable en service** : oui, dès qu'une base basculée sert une vue (2b-C), et pour toute vue de worktree à
  venir.
- **Touche rag3weaver** : oui (`src/code_sync.rs`, la fin d'un index ; `Catalog::publier_la_generation`, qu'aucun
  chemin du produit n'appelle).
- **Pour** : arbre principal (2a′-3), au mot de c7 du 16 novembre.
- **Signalé par** : recherche et arbre principal, en réglant le sens de `view` sans rang sur l'écrivain d'ED
  (master c33574eb9), 16 novembre 2026.

## Ce que c'est

- **Une vue sans rang lit la dernière génération publiée**, n (page 2b′, promesse 1). L'appel sans vue lit le rang
  en attente, w = n + 1 (page 06 d'ED).
- **Rien du produit ne publie** : `publier_la_generation` n'a pas d'appelant (ED, 15 novembre). L'écrivain écrit
  tout au rang en attente, et le rang publié reste 0 depuis la montée aux générations.
- **D'où l'écart** : sur une base vivante, `view: {layer: backend}` rend l'index tel qu'il était à la bascule, et
  l'appel sans vue l'index d'aujourd'hui.

## Le remède

- **La couche du backend publie à chaque fin d'index** : quand le journal écrit « indexation terminée », w devient
  publié (n = w), et un rang neuf s'ouvre en attente. C'est `history` de la page 2a′-3, appliqué à la couche du
  backend.
- **« L'index est prêt »**, pour Algareth et l'attente d'index de la page 12 d'archi
  (`docs/rag3weaver-archi/15-11-2026/12-conception-l-attente-d-index-du-pas.md`), c'est une génération publiée.
  L'attente bornée attend cette publication, pas la fin d'un journal.
- **Pas de refus nommé entre-temps** (c7 a tranché, 16 novembre) : la réponse dit l'écart, et aucune session ne lit
  de vue avant 2a′-3.

## Le témoin

Rouge d'abord : un index fini sur une base à générations publie son rang. Ensuite, `view: {layer: backend}` sans
rang et l'appel sans vue rendent la même réponse, et `pending_rows` est faux.

## Ce qu'il faut pour le fermer

Le témoin rouge puis vert dans 2a′-3, et la vérification sur une base basculée : après un index, `rank` de la vue
vaut le rang publié neuf.
