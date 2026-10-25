# La recherche fusionnée perd ce que chaque signal a dit, et search_code ne rend pas sa méta

- **État** : corrigé le 25 octobre 2026 (session « recherche », pièce 1 de la recherche de
  l'IDE) ; ouvert le même jour.
- **Gravité** : réponse incomplète. Une page ou un client ne peut montrer ni les scores par
  signal, ni un repli, ni les compteurs sans relire du markdown.
- **Atteignable en service** : oui, toute recherche fusionnée, et `search_code` dans l'IDE.
- **Touche rag3weaver** : oui (`FuseResultsNode`, `UnifiedResult`, le gabarit de code).

## Ce que c'est

`FuseResultsNode` remplaçait le score de chaque résultat par le score fusionné : le score brut et
le rang de chaque signal, pourtant tenus pendant la fusion, étaient perdus. Et `search_code` ne
déclarait pas de port `metadata` : sa `SearchMeta` (compteurs, avertissements, état d'index)
n'existait que réécrite dans `presentation`.

## La recherche

`search_code` sur un backend de code indexé avec ses vecteurs : la réponse porte un score par
résultat et une étiquette (« bm25+vector »), et `metadata` vide.

## Le correctif

- `UnifiedResult.signals` : `{label, score, rank}` par signal, rempli par `FuseResultsNode`.
- Le port `metadata` de `search_code` dans le gabarit de code : `metadata["render.meta"]`.
- Page `extension/rag3weaver/docs/12-octobre-2026-recherche/06-la-recherche-de-l-ide-dit-ce-qu-elle-sait.md`.

## Les témoins

- `tests/e2e_lectures.rs` : `la_recherche_rend_chaque_signal_et_sa_meta` (rouge sur master).
- `src/dataflow/generic_search_nodes.rs` : `fuse_keeps_each_signal_score_and_rank`.
