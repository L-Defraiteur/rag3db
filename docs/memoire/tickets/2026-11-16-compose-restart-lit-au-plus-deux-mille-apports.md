# compose_restart lit au plus deux mille apports

- **État** : ouvert, le 16 novembre 2026.
- **Famille** : produit
- **Gravité** : une perte silencieuse. Au-delà de 2 000 apports touchés, les plus anciens
  manquent au bilan et au contexte d'après, sans que la réponse le dise.
- **Atteignable en service** : oui. Le banc compose sur ~1 900 sections cumulées, au bord du
  plafond.
- **Touche rag3weaver** : oui. Le gabarit memory : `plans/compose_restart.json` (limite liée au
  paramètre `limit`, max 2000), `scripts/compose_restart_params.rhai` et
  `scripts/compose_restart.rhai`.
- **Pour** : Codex, ou la session mémoire (96).
- **Signalé par** : la session mémoire (96), à la passation du 16 novembre 2026.

## Ce qu'on voit

Le Plan sélectionne en une forme les apports de la session (`source` dans `sources`), ceux des
fiches d'identité et de tâche, et ceux des autres sessions sur les mêmes fiches. Les lignes sont
triées par `created_at` décroissant, avec `limit` 2000 au plus (le Plan exige un plafond `max`).
Une session longue et ses fiches voisines dépassent ce nombre. Les apports les plus anciens de la
session tombent alors hors de la page, et ce sont justement les premiers pas du bilan.

## Ce qui serait juste

1. Deux formes séparées, et non une seule :
   - les apports de la session, triés par `created_at` croissant, paginés jusqu'au bout par
     `offset` (la forme est déjà ordonnée par `_uuid` en dernier, ce qu'exige la pagination) ;
   - les apports des autres sessions, les plus récents d'abord, bornés par le budget.

   Un Plan n'a qu'une forme de lignes en sortie : soit deux PlanNode chaînés par des Rhai, soit
   une boucle de pages par un ForEachNode.
2. La réponse dit `truncated: true` avec le nombre d'apports non lus si un plafond est atteint.
   Rien ne doit tomber en silence.
3. Ce qui ne se touche pas : la forme rendue (`text`, `tokens`, `bilan`, `parts`,
   `left_to_recall`, `over_budget`, `estimate`) et l'ordre de priorité de la composition.

## Le témoin, rouge d'abord

Dans `tests/e2e_compose_restart.rs`, un cas `compose_restart_reads_every_session_apport_past_the_page`.
Il pose 2 100 apports de session, par `put_apport` et `relier_apport` sur quelques fiches, puis
appelle `compose_restart` avec un grand budget. Il attend que `bilan` compte 2 100 lignes, et que
la première soit le tour le plus ancien.

L'écriture de 2 100 apports demande l'embarqueur : la suite tourne sous `RAG3WEAVER_EMBED_SERVICE`,
comme le reste de `e2e_compose_restart`.

## Taille

4 à 6 heures.
