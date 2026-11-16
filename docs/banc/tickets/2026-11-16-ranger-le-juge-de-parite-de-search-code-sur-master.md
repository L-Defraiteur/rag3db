# Ranger le juge de parité de search_code sur master

- **État** : ouvert.
- **Famille** : produit
- **Gravité** : moyenne. Le juge qui a servi à toutes les parités du 14 au 16 novembre ne vit que
  sur une branche à 1 400 commits de retard. Chaque mesure le recopie à la main dans un worktree.
- **Atteignable en service** : non, c'est un outil de mesure.
- **Touche rag3weaver** : oui, `tests/`, `scripts/` et `Cargo.toml`.
- **Pour** : une session Codex (passation du 16 novembre).
- **Signalé par** : banc, 16 novembre.
- **Ouvert le** : 16 novembre 2026

## Le constat

La branche `banc-parite-search-code` (tête 3d422fad11) porte le juge : la sonde
`extension/rag3weaver/tests/sonde_parite_search_code.rs`, le comparateur
`extension/rag3weaver/scripts/parite_search_code.py`, le jeu
`extension/rag3weaver/tests/donnees/jeu-recherche-2a2.json`, et l'entrée `[[test]]` de
`Cargo.toml`. Les scripts de luciepc les copient dans un worktree du moment, puis greffent
l'entrée `[[test]]` et `dead_code = "allow"` dans `Cargo.toml` (voir
`~/.cache/rag3weaver-build/b-prime.sh`). Aucune mesure n'est rejouable depuis master seul.

## Ce qu'il faut faire

- Une branche `cx-banc-juge-de-parite` depuis master, qui reprend **ces quatre éléments seulement**.
- **Ne pas reprendre** les autres changements de la branche : `src/catalog.rs`, `src/code.rs`, les
  sondes `sonde_clause_sans_filtre.rs` et `sonde_recherche_sans_filtre.rs`, et les deux pages de
  docs. Ils sont d'une autre époque, et leur équivalent est déjà sur master sous une autre forme
  (`migrer_vers_les_generations`, `marquer_des_lignes_invisibles`).
- La sonde doit compiler sur master **sans** `dead_code = "allow"` : retirer ce qui n'est plus
  appelé, ou le marquer localement.
- Les API qu'elle appelle existent sur master : `code::scope_form`,
  `code::migrer_vers_les_generations`, `Catalog::publier_la_generation`, `rang_publie`, `au_rang`,
  `Catalog::embarquer_le_retard_hors_verrou` et `COUCHE_DU_BACKEND` (dans `generations.rs`). Si un
  chemin d'import a bougé, corriger l'import, pas l'API.
- Les noms existants restent en français (`releve_les_reponses_de_search_code`…). Tout ce qui est
  neuf est en anglais.
- Ajouter un témoin du comparateur, `extension/rag3weaver/scripts/test_parite_search_code.py`. Il
  donne deux relevés minuscules écrits à la main : identiques (VERT), un rang échangé (ROUGE), un
  score qui diffère (ROUGE en `reponses`). Code de sortie 0 ou 1.

## La preuve

- Le témoin du comparateur, vu rouge d'abord : écrire les cas avant de toucher au script, et les
  faire rougir en cassant une comparaison à la main.
- `cargo test --test sonde_parite_search_code --features rag3db-native,code --no-run` compile, sous
  `poste lourd`, sur un target à soi.
- Rejouer un relevé réel par master, avec le juge rangé : le relevé tous signaux de B′ (commande du
  ticket `2026-11-16-rejouer-la-parite-de-la-migration-au-pointeur-lucivy-4-3-9.md`). Il doit
  rendre le même fichier que `banc-parite/sorties/b-prime/bprime.json`, aux scores près.
- La passe et le push suivent `docs/procedures/push-d-un-lot.md`, au « à toi » de l'orchestration.
  Les tests ignorés ne jouent pas dans la lib : la passe doit seulement compiler la cible.

## Taille

Trois à quatre heures.
