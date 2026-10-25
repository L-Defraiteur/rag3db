# Une demande de reclassement à search_code est ignorée en silence

- **État** : corrigé le 25 octobre 2026 (session « recherche », pièce 2 de la recherche de
  l'IDE) ; ouvert le même jour.
- **Gravité** : réponse fausse en silence. L'appelant croit avoir un ordre relu ; il a l'ordre de
  fusion, et rien ne le lui dit.
- **Atteignable en service** : oui, tout `search_code` avec `options.rerank`.
- **Touche rag3weaver** : oui (`templates/tools/search_workspace.mmd`, `RerankNode`).

## Ce que c'est

Le graphe de `search_code` (`search_workspace.mmd`) n'avait pas de `RerankNode`. Un
`options.rerank` passé par l'appelant grossissait seulement le sur-fetch des signaux
(`budget_de_recherche`) : rien n'était reclassé, aucun avertissement n'était rendu, avec ou sans
relecteur configuré.

## La recette

`search_code` avec `{"options": {"rerank": {"candidates": 20}}}` : la réponse a l'ordre de la
fusion, `rerankedCount` vaut 0, et aucun avertissement.

## Le correctif

- `RerankNode(keep_signal=true)` entre la fusion et la pagination de `search_workspace.mmd`,
  actif seulement à la demande. Sans relecteur, la méta le dit (« rerank demandé, aucun reranker
  configuré — ordre de fusion conservé »).
- `RerankNode` garde ce que chaque signal a dit à travers l'enrichissement du pool, et y ajoute sa
  ligne (score et rang du relecteur). Une liste vide passe sans consulter le relecteur.
- La mesure, sur de vraies questions de `code-lourd` : `tests/sonde_reclassement.rs`, et ses
  chiffres dans la page `extension/rag3weaver/docs/12-octobre-2026-recherche/06-la-recherche-de-l-ide-dit-ce-qu-elle-sait.md`.

## Les témoins

- `tests/e2e_lectures.rs` : `search_code_reclasse_a_la_demande_et_dit_quand_il_ne_peut_pas`
  (rouge sur master).
- `src/dataflow/generic_search_nodes.rs` : `the_reranker_adds_its_line_to_what_each_signal_said`.
