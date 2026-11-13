# Une tâche sans plan se dit faite sur un step_done qui dit ce qui reste

- **État** : corrigé le 13 novembre 2026 (6eb8c11c6), par everything declarative. Relevé le 9 novembre
  par la session mémoire (96) pendant l'essai de l'équipier sur les README.
- **Famille** : produit
- **Gravité** : une tâche réelle s'arrête après son premier pas sans avoir rien livré.
- **Atteignable en service** : oui, par toute tâche dont l'équipier n'ouvre pas de plan.
- **Touche rag3weaver** : oui, la fin d'un pas dans `src/daemon_host/teammate.rs`.

## Ce qu'on voit

La request de `readme-nouveautes-novembre` demandait un report de verdicts, puis une question par
ajout, une réponse du superviseur à chaque fois, et « ne déclare pas la tâche finie avant ma
réponse au report final ». L'équipier n'a pas ouvert de plan. Son premier pas a fini sur :

    step_done { item: "Analyse et verdict des nouveautés de novembre 2026",
                done: "Analyse complète des 31 lignes…",
                remaining: "Validation du verdict et soumission des ajouts un par un pour accord avant modification.",
                succeeded: true }

L'hôte a aussitôt écrit `ended : fait sans plan` et arrêté la tâche : ni report, ni question, ni
proposition. La règle est dans `teammate.rs` (la fin se juge après le pas) : aucun plan vivant et
un `step_done` déclaré valent fin, quel que soit `remaining`.

## Ce qu'il faut

- Un `step_done` dont `remaining` n'est pas vide **n'est pas une fin**, plan ou pas : le pas
  suivant part.
- Une request qui demande un livrable dans la boîte (un report, une question, l'accord du
  superviseur) ne se clôt pas sans lui ; au plus, la fin se dit « faite sans le livrable
  demandé » et attend le superviseur.
- Témoin : sans plan, un `step_done` avec `remaining` relance un pas ; sans `remaining`, la fin
  reste « fait sans plan » (le cas de la petite demande, `teammate_tests.rs`).

## Ce qui a été fait (6eb8c11c6)

- Un `step_done` dont `remaining` n'est pas vide relance le pas suivant, plan ou pas ; l'hôte le
  dit dans la boîte (« pas fini : reste « … » ; le pas suivant part »). MAX_STEPS et MAX_STALLED
  bornent.
- Une request déclare ce qu'elle exige : `expects: "report"` ou `"approval"` (par l'IDE, la boîte
  de Codex, `mail send --kind request --expects …`). Sans le livrable, l'hôte écrit « fait sans le
  livrable demandé (…) : j'attends le superviseur » et l'équipier attend la boîte sans appeler le
  modèle. Sans `expects`, rien ne change : la petite demande finit « fait sans plan ».
- Témoins : `teammate_tests` (`a_step_done_that_says_what_remains_is_not_the_end`,
  `a_request_that_expects_a_report_does_not_end_without_it`,
  `a_request_that_expects_approval_does_not_end_without_it`, `expects_is_a_request_field_with_two_values`)
  et `mail::tests::send_porte_expects_sur_une_request`, vus rouges puis verts ; `e2e_equipier` vert
  sur le vrai Gemini.
- **Limite connue** : un `remaining` non vide qui ne dit rien qui reste (« rien », « aucun ») relance
  40 pas au plus. Le premier jeu Gemini du lot a tourné 15 min sans finir, cause non prouvée (la boîte
  partait avec le tempdir) ; le message système dit depuis que `remaining` reste vide quand rien ne
  reste, et le témoin montre la boîte avant de rougir. Si on le revoit en vrai, c'est là qu'on dira
  ces mots-là.
