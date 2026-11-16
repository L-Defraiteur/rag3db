# Les passes des lots ne jouent pas e2e_code_sync : master a rougi sans qu'aucune passe le dise

- **État** : fait (16 novembre 2026, branche required-suites) : tools/poste/required_suites.txt dit quels chemins exigent e2e_code_sync (code.rs, code_sync.rs, le catalogue, l écrivain, la suite elle-même) ; passe_epinglee.sh la joue, même sous SUITES, et le résumé dit les suites exigées et toutes les suites jouées ; push_garde.sh refuse une passe qui ne l a pas jouée verte ; rouge d abord aux deux témoins (4 rouges sur la garde de master). Le défaut lui-même reste à ED et à l arbre.
- **Famille** : produit (outillage : `extension/rag3weaver/scripts/run_e2e.sh`, `passe_epinglee.sh`)
- **Gravité** : un rouge de master non vu. Onze témoins de la synchronisation et du point de reprise sont rouges sur master e264210dc7 (lib 373b5477b comme 52101b7fd) ; aucune des passes de la nuit du 15 au 16 ne les a joués.
- **Atteignable en service** : oui (la synchronisation d'une source rend Ok avec `files_ingested: 0` et `scopes_written: 0` ; une source tronquée est acceptée sans `force`)
- **Touche rag3weaver** : oui
- **Pour** : G (optimiseur) pour la passe ; le défaut lui-même est départagé par ED et l'arbre (écrivain c33574eb9, 2b′ 988651d39e, 2b-C 457892de98)
- **Signalé par** : codeparsers, 16 novembre 2026, 2 h 3x, par la passe complète de sa pile (e2e_code 45 passés, 11 échoués, les mêmes onze sur master seul)

## Ce qu'on voit

Les passes des lots de la nuit disent « e2e_code 36/36 » : c'est le binaire `e2e_code`. La suite `e2e_code_sync`
(20 témoins : `synchroniser_une_source_retire_les_fichiers_supprimes`, `le_plan_seul_ne_retire_rien…`,
`une_source_tronquee_est_refusee_sans_force`, `une_edition_apres_un_point_de_reprise…`,
`apres_un_point_de_reprise_les_deux_sens…`) n'est pas dans la passe épinglée ni dans le rejeu court, alors que
l'écrivain des générations et 2b′ touchent `code_sync.rs`. Le rouge est entré sur master sans un mot ; c'est la passe
complète de codeparsers, qui joue `e2e_code` avec tous ses binaires, qui l'a vu.

## Ce qu'il faudrait

- `passe_epinglee.sh` et le rejeu court jouent `e2e_code_sync` dès qu'un lot touche `code.rs`, `code_sync.rs`, le
  catalogue ou l'écrivain (la même règle que pour `e2e_postgres`), et le résumé de passe nomme chaque binaire joué.
- Le juge de la passe refuse un résumé qui dit « e2e_code » sans dire ses binaires : une suite non jouée est rouge.
- Témoin rouge d'abord : une passe sur un lot qui touche `code_sync.rs` et ne joue pas `e2e_code_sync` doit rougir
  (ou refuser) ; vert avec la suite jouée.
