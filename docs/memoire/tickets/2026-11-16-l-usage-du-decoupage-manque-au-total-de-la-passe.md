# L'usage de l'appel de découpage manque au total de la passe d'archive

- **État** : ouvert, le 16 novembre 2026.
- **Famille** : produit
- **Gravité** : une mesure fausse par défaut. `usage_total` sous-compte les jetons d'une passe
  d'archive dès que `segment_over` est posé, et le banc lit ce total pour décider d'une dépense
  chez Vertex.
- **Atteignable en service** : oui, avec `archive` et `segment_over > 0`, sous `trace: true`.
- **Touche rag3weaver** : oui. `src/dataflow/llm_nodes.rs` (LlmNode), et dans le gabarit memory
  `graphs/archive.mmd`, `graphs/archive_segment.mmd`, `scripts/archive_segment_build.rhai` et
  `scripts/archive_rapport.rhai`.
- **Pour** : Codex, ou la session mémoire (96).
- **Signalé par** : la session mémoire (96), à la passation du 16 novembre 2026.

## Ce qu'on voit

Le lot `usage` (sur master depuis 0653a6aef) pose sur chaque élément étiqueté par `Decide` un
`usage {ms, prompt_tokens, cached_prompt_tokens, completion_tokens}`. La trace d'`archive` le rend
par section, et `scripts/archive_rapport.rhai` en fait `usage_total`.

Le lot `segment` (sur master depuis 53a83d089) ajoute un appel au modèle par `LlmNode`, dans
`archive_segment.mmd`, pour chaque section longue. `LlmNode` ne pose pas d'`usage` : son
`Usage` part sur le port `llm` (`LlmOutput`) et dans les métriques du nœud, mais rien ne le
porte jusqu'au rapport. Ces appels manquent donc au total.

## Ce qui serait juste

1. `LlmNode`, quand `response_schema` est posé, ajoute à l'objet rendu sur `json` un champ
   réservé `_usage`. Sa forme est exactement celle de `Decide` : `ms` toujours, et les trois
   compteurs seulement si `prompt_tokens > 0`, sinon absents, pas à zéro. Voir
   `src/dataflow/decide_nodes.rs`, `by_llm`.
2. `archive_segment_build.rhai` reporte ce `_usage` sur la première sous-section rendue, sous le
   nom `segment_usage`.
3. `archive_rapport.rhai` additionne `segment_usage` dans `usage_total`, avec un compte à part :
   `segment_calls`.
4. Ce qui ne se touche pas : la forme d'`usage` de `Decide` et le sens de `without_usage`.

## Le témoin, rouge d'abord

Dans `tests/e2e_archiviste.rs`, un cas `segment_usage_is_counted_in_the_pass_total`, sur le
modèle de `a_long_section_is_split_into_topics_then_filed_topic_by_topic`. Le faux serveur rend le
découpage avec un bloc usage (l'aide `avec_usage` existe dans ce fichier). Le témoin attend que
`usage_total.prompt_tokens` compte l'appel de découpage, et que `segment_calls` vaille 1.

Les témoins unitaires de `llm_nodes.rs` vérifient `_usage` sur `json`, et son absence sans
`response_schema`.

## Taille

3 à 4 heures.
