# Les outils d'index sur un dépôt TypeScript étranger : six défauts relevés par l'agent

Ouvert le 24 novembre 2026, 12 h. Source : l'étude écrite par un agent rag3weaver dans le dépôt de Lucie `tisserande`
(TypeScript, React, Three.js, Vitest ; 193 fichiers), `tisserande/docs/evaluation-outils-exploration.md`, commit
a524e26 de ce dépôt. C'est le premier banc « dépôt étranger » de la vision produit.

| # | Défaut | Vérifié | À qui |
|---|---|---|---|
| 1 | `usages` voit deux définitions de `computeOBB` : la vraie (`OBB.ts:121`) et le bloc `describe("computeOBB", …)` de `OBB.test.ts:33`, puis range tous les appels en « nom ambigu ». Le parseur en fait un scope marqué test (`base_scope_extraction_parser.rs:353`, `TestMark`), nommé du titre et typé comme une fonction. Le parseur dit la vérité ; c'est la résolution des définitions qui doit écarter un scope marqué test (gabarit ou requête de `usages`, `impact`). Sur un projet testé par Vitest ou Jest, presque chaque fonction a son `describe`. | oui, par le code | recherche |
| 2 | `search_code` : sans `scope: "src/"`, le Markdown écrase le code sur une question d'implémentation. | à mesurer | recherche |
| 3 | `search_code` écarte comme « trop fréquents » des jetons courts qui portent le sens (`ik`, et `ou`, `de`). | à mesurer | recherche |
| 4 | `search_code` avec `mode: "scan"` échoue à l'exécution. | à rejouer | recherche |
| 5 | `outline` d'un composant React (`Viewport.tsx`, 956 lignes) : un seul bloc, aucun hook ni gestionnaire. | à vérifier | codeparsers |
| 6 | `callees` ne résout pas un appel par interface (`node.execute(ctx)`) vers ses implémentations. | connu, par construction | codeparsers, à concevoir |

Ce qui marche, selon l'étude : `search_code` sur le vocabulaire métier, `usages` sur un nom sans homonyme, `impact`
pour trouver le fichier de test à rejouer, `outline` sur un module TypeScript, `grep_files`.

Chaque défaut se prouve rouge d'abord, sur un extrait minimal de ce dépôt rangé en corpus de test.

## Le filtre par défaut de `search_code` (Lucie, 24 novembre 2026, 12 h 05)

Pour le défaut 2 : sans `scope` donné, si le dossier a un `src/`, la recherche s'y borne par défaut, et le résultat le
dit en tête : « filtre `src/` appliqué d'office ; pour chercher ailleurs, donne `scope` (une autre valeur, ou vide pour
tout le dépôt) ».

Une précision à trancher à l'écriture (c7) : beaucoup de dépôts ont leur code hors de `src/` (`lib/`, `packages/`,
`crates/`, `extension/`, la racine). Le défaut pourrait être « les fichiers de code » (les langues que le parseur
reconnaît) plutôt qu'un chemin, avec `src/` en plus quand il existe ; la même ligne d'en-tête le dit. Le défaut et son
texte vivent dans le gabarit de l'outil, modifiables sans bâti.

