```json
{"parent": "37b77a667d17cfae3d85602596c4efb44aa5a095", "tree": "extension/rag3weaver", "needs_lib": true, "witness": "usages_does_not_show_the_declaration_of_a_retired_container", "test": "./run_e2e.sh --features openai-llm,anthropic-llm --test e2e_code_sync usages_does_not_show_the_declaration_of_a_retired_container", "suites": ["./run_e2e.sh --features openai-llm,anthropic-llm --test e2e_code_sync"], "hints": ["extension/rag3weaver/tests/e2e_code_sync.rs"], "ours": "5ad882754d1c61675c1c399d64e26f8bf20b3dec"}
```

# Cas de campagne : `usages` montre la déclaration d'un conteneur retiré

Session recherche, 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie ; lanceur d'archi,
juge du banc). Un cas court, en Rust, sur la lecture des usages sous les générations.

## Le point de départ

- **Commit parent** : `37b77a667d` (18 novembre 2026). Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : `extension/rag3weaver`. **La lib rag3db est nécessaire** : le témoin indexe sur une base rag3db.

## Le signalement, tel qu'il était

> Sous les générations, une ligne retirée reste dans sa table. Une classe C++ déclare `bar` dans un en-tête, sans
> définition indexée : `usages bar` montre « déclaré dans Foo — foo.h:3 ». Une édition retire la classe, la
> synchronisation la retire. `usages bar` montre encore la déclaration dans `Foo`, comme si la classe existait
> toujours. Il ne devrait plus la montrer.

## Le témoin

- **Filtre** : `usages_does_not_show_the_declaration_of_a_retired_container`, dans
  `extension/rag3weaver/tests/e2e_code_sync.rs` (suite `e2e_code_sync`). Identique sur master (vérifié le 19 nov. sur
  3625a18b24).
- **Rouge au parent** : « the retired class is not shown », la réponse contient encore « déclaré dans Foo ».
- **Suites exigées par les chemins** : `e2e_code_sync`, car le témoin est dans `tests/e2e_code_sync.rs`
  (`tools/poste/required_suites.txt`).

## Ce que l'arbre du parent porte déjà (pour le juge)

Le ticket `docs/recherche/tickets/2026-11-18-usages-lit-des-conteneurs-sans-la-borne-des-generations.md` est présent
au parent, et il nomme la piste du correctif (les sélections de `usage_nodes.rs` et la borne de visibilité). Le
lanceur ne le donne pas comme indice, mais l'agent peut le trouver en lisant l'arbre. Le banc dira si ce cas reste
un cas, ou s'il faut retirer ce ticket de l'arbre de départ.

## Notre correctif (caché à Algareth, pour le juge)

`5ad882754d`, « fix(usages): les six sélections d'usage_nodes passent par traduire_select » :
`extension/rag3weaver/src/dataflow/usage_nodes.rs` +6 −9, et le ticket fermé (+10 −1).
