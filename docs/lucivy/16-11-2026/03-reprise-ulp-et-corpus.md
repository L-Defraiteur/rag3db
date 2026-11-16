# Reprise ulp et issue de recherche du corpus — 16 novembre

## État et périmètres

Audit fait contre rag3db origin/master927941a67a5805450d49f1e04b39fdd542a6059b
et lucivy origin/main a1c0dfddfb97666aee148480f8eb3e98d00885ce.
Les changements13→16 (barrière publiée4.3.4, dictionnaire4.3.5/8, BM25filtré4.3.7/9)
ne sont pas des lots à rejouer. Pointeur4.3.9 sur master depuiscb4a1757a4.

Entrée3 : ticket `2026-11-14-un-ulp-d-ecart-de-score-entre-profils-en-parse.md`,
branche `cx-lucivy-parse-sum-order` poussée à1b9f47d4f87dddbe76cdf5535a95de6cdff11bdb,
worktree portable `~/git_workspaces/cx-lucivy-parse-sum-order`.
Fichiers réservés : src/query/score_combiner.rs, src/query/boolean_query/block_wand.rs,
lucivy_core/tests/test_parse_scores_identical_across_layouts.rs. Le WIP de
passation est repris tel quel, sans modification de source ni nouveau test joué.

Entrée4 distincte : ticket `2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`,
branche `cx-lucivy-indexed-false-warning`07ccd12e484ea20f4ae85d23639f9da7316acbfd,
worktree portable de même nom. APIvide+test préparés, NON compilés/joués.
Réservation handle.rs/test_text_field_indexed_false.rs, puis query.rs et READMEcore
lors du correctif APRÈS rouge. Aucun conflit de fichier avec entrée3.

## Corpus : ce qui est réellement trouvé

La recette common::recipe du WIP demande noyau7.2, comparatif épinglé8d3ae59288f1.
~/lucivy_bench/linux-7.2, /tmp/lucivy-cmp et /tmp/lucivy-cmp-90k absents aux deux
postes. Les autres racines documentées linux-bench/rag3db-bench/parity_native
ne sont pas présentes aux emplacements contrôlés. Pas de recherche dans les
bases privées ni de lecture .env/.vault.

Cache public à luciepc : ~/.cache/lucivy-corpora, dont archive
linux-linux-2.6.0.tar.gz42Mo ; provenance de son nom et URL définies par
playground/tools/build_corpus.py et playground/corpora.json (kernel.org2.6.0).
Cache clone rag3db présent : ~/.cache/lucivy-tests/rag3db-clone.
Aucune de ces constatations ne valide leur substitution au noyau7.2 : ni
extraction, téléchargement, nouveau lien ni configurationfallback effectué.
Les tests dictionary_index/derived_in_ram/positions_off lisent /tmp/lucivy-cmp
en dur, donc V3_CORPUS ne résout pas à lui seul ce blocage.

## Couverture acquise/manquante et recette proposée, PAS lancée

Acquis sur WIP : ancien témoin rouge puisvert1/0/0, lib1480/0/24ignorés,
clippy lib et crates rc0. Core s’est arrêté après échec du corpus, donc pasvert.
Les24ignorés demandent inventaire/qualification de portée, pas une fausse
affirmation de libintégralement verte. JournauxPC cache/rag3weaver-build/lucivy-ulp.

Après coordination corpus ET créneau distinct (pas pendant G/Arbre) :
- nouvelle autorité/budget/logs et targetprivé `cx-lucivy-target-16nov`, jamais
  target-lucivy-cmp d’une autre session ; poste lourd, maxj8 et deuxcœurslibres,
  plafond mémoire relu selon règles actuelles, aucune ancienne autorité réactivée ;
- consigner provenance/SHA/nombre de fichiers du corpus réellement admis,
  liens nommés conformes et cache rag3db existant, sans auto-download caché ;
- témoin exact : `cargo test --release -j8 -p lucivy-core --test test_parse_scores_identical_across_layouts -- --nocapture` ;
- suite manquante demandée par passation : `cargo test --release -j8 -p lucivy-core --no-fail-fast`,
  sortie en fichier, résumé de TOUSbinaires, tests sautés non comptésverts ;
- lib/features/clippy et groundtruth selon CI ; vérifier les assertions de score
  sensibles à f32→f64. Ne pas changer BM25 ni relâcher une tolérance sans mesure ;
- profil-léger parse sur copie documentée (jamais base Lucie) après fenêtreMESURE
  coordonnée, puis PR. Pas de push main direct ni tag, PR42 refusée à remonter.

Pour entrée4 : même poste/targetprivé après créneau, mais témoin pureconfig CPU
sans corpus ni index : `cargo test --release -j8 -p lucivy-core --test test_text_field_indexed_false -- --nocapture`.
Il doit être vu rouge sur scaffoldvide avant avertissements/doc ; aucune raison
de modifier le test ulp ni d’honorer indexed:false dans ce lot.
