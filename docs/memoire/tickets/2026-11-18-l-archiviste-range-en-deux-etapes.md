# L'archiviste range en deux étapes : extraire les sujets d'une fenêtre, puis les ranger

- **État** : ouvert, le 18 novembre 2026. Le code démarre après que Lucie a vu la page de
  conception, ou demain matin sans l'attendre.
- **Famille** : produit
- **Gravité** : une forme qui n'est pas celle voulue. Aujourd'hui, une décision par tour mêle
  « de quoi parle ce tour » et « sous quelle fiche ».
- **Atteignable en service** : oui, par l'outil `archive` du gabarit memory.
- **Touche rag3weaver** : oui, le gabarit memory (`graphs/archive.mmd` et un graphe neuf
  d'extraction, des scripts Rhai) et `tests/e2e_archiviste.rs`. Pas de Rust neuf attendu :
  `LlmNode` sous `response_schema` et `ForEachNode` avec `flatten` sont sur master (53a83d089).
- **Pour** : la session mémoire (96).
- **Signalé par** : Lucie, le 18 novembre 2026, par c7. La conception est dans
  `docs/memoire/18-11-2026/02-l-archiviste-en-deux-etapes.md`.

## Ce qu'il faut faire

1. **Le mode `extract`**, par un paramètre d'`archive` (`mode`, valant `section` ou `extract`).
   Le défaut reste `section`, la forme d'aujourd'hui, jusqu'à la mesure du banc.
2. **L'étape 1**, un graphe neuf `graphs/archive_extract.mmd` :
   - les tours numérotés de la fenêtre, plus `context` ;
   - un `LlmNode` sous le schéma `{topics: [{title, turns: [{from, to}], said}]}` ;
   - un Rhai qui fait de chaque sujet un élément `{title: "turns a-b · <title>", text: said + les
     tours cités, …}`.
   Un intervalle hors de la fenêtre est refusé en le nommant. Ce graphe reprend la mécanique de
   `archive_segment.mmd` et de son `archive_segment_build.rhai`.
3. **L'étape 2** : le `ForEachNode` de `archive_decide` d'aujourd'hui, sur ces éléments. Le rappel
   se fait par `title` et `said`.
4. **Ce qui ne se touche pas** : le mode `section`, `usage` (l'appel d'extraction compte dans
   `usage_total`, voir le ticket de l'usage du découpage), et le Decide lui-même.
5. Tout code neuf est en anglais : clés, nœuds, tests, prompt.

## Le témoin, rouge d'abord

Dans `tests/e2e_archiviste.rs` : `extract_mode_files_one_l1_per_topic_not_per_turn`.

- **Le document** : 6 tours.
- **Le faux serveur** rend l'extraction de 3 sujets (dont un sur deux intervalles), puis 3 Decide.
- **Ce qui est attendu** :
  - 4 requêtes en tout ;
  - la première porte le schéma `topics`, et aucune fiche candidate ;
  - 3 L1 de section `turns a-b · <title>` ;
  - le texte de chacun porte son `said`.

Un second cas : une extraction non JSON est refusée en nommant `response_schema`.

## Taille

1 à 2 jours, la mesure du banc comprise.

## Seconde phase : la boîte à outils du contexte (Lucie, 18 novembre au soir)

- **L'extraction sort aussi `slice_summary`**, dans le même appel. Le rapport de tranche
  (`previous_report`) est composé depuis la trace. Les deux s'écrivent comme apports sur la fiche
  de la session, pour que la tranche suivante les relise.
- **Le contexte de chaque étape est composé d'ingrédients déclarés**, dans les `bindings` de
  l'outil `archive`, sous `archivist.context` :
  - `previous_report`, `previous_slice_summary`, `current_slice_summary`, `previous_summaries`,
    `ordered_bilan`, `candidate_fiches` et `candidate_categories`, chacun `{enabled, max_tokens}` ;
  - `overlap_percent` ;
  - `archivist.window_tokens`.

  Le défaut est S 50, et la liste est au § (e) de la page de conception. Une variante du banc
  (F, S, R, leurs mélanges) est une configuration.
- **Les témoins**, sur le faux serveur : chaque ingrédient est présent allumé, absent éteint, et
  coupé et dit au-delà de sa borne (`context_parts` dans la trace). Un cas par variante.
- **La taille** : environ deux jours de plus que la première phase.

## Troisième phase : ce que voit l'agent après la compaction (Lucie, 18 novembre au soir)

`compose_restart` devient une composition d'ingrédients déclarés sous `restart.context`, chacun
`{enabled, max_tokens}`, dans le budget `restart_budget_tokens` :

- `identity`, `task`, `ordered_bilan`, `session_l1` et `others_l1`, qui existent déjà ;
- trois ingrédients neufs : `global_summary` (les `slice_summary`, bornés, ou un appel en
  `mode: llm`), `last_report` et `raw_tail` (passé par l'hôte).

Trois profils pour le banc, sous `restart.profile` : `l1_only`, `summary_only` et
`l1_and_summary` (le défaut proposé). Ils sont jugés contre la compaction de Claude Code. La forme
rendue ne change pas. La liste est au § (f) de la page de conception.

- **Les témoins** : un par ingrédient (présent, absent, coupé et dit), un par profil, dans
  `tests/e2e_compose_restart.rs`.
- **La taille** : environ une journée.
- **La part de l'hôte** est chez archi : il passe `raw_tail` et appelle `compose_restart` au point
  de branchement, à 80 % de la jauge.
