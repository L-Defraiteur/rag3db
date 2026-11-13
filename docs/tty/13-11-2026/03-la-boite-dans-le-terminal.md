# La boîte de l'équipier dans le terminal (tty-4)

13 novembre 2026, session tty. Sur master en `212893a17`.

## Ce qui change à l'écran

L'équipier attend parfois quelque chose de son superviseur. La ligne d'état le dit en priorité, avant toute animation :
- **une demande d'accord** (`approval`, une commande que la porte des commandes retient) : « ⛧ attend ton accord : echo accorde-moi — o / n ». Sans rien taper d'autre, `o` (ou `y`) l'accorde et `n` la refuse ;
- **une question** ou **une proposition** : « ⛧ attend ta réponse : <sa première ligne> ». La saisie y répond, puis Entrée l'envoie ;
- **plusieurs en même temps** : on répond à la plus ancienne, et « (+N) » dit combien il en reste ;
- **rien qui attende** : l'état vient de l'hôte, `GET /ide/teammate/list?supervisor=supervisor`. « attend la boîte » pour `waiting`, « en erreur » pour `error`, et la fin s'il a fini.

Une saisie ne devient une `instruction` que quand rien n'attend de réponse.

## Ce qui part à l'hôte

`POST /ide/teammate/mailbox` avec `{backend, task, kind: "answer", text, reply_to: <id de la ligne>, approved?}`. C'est la forme de la page de l'équipier (`extension/rag3weaver/docs/07-novembre-2026-everything-declarative/01-l-equipier-agent-de-code.md`, § 2). Pour une approbation, l'hôte pose l'accord dans la porte des commandes de la tâche, puis l'écrit dans la réponse (`granted`).

## Où vit la logique

`extension/rag3weaver/tty/src/boite.rs`, sans ratatui :
- `Boite::ligne` tient ce qui attend, jusqu'à ce qu'une ligne porte `reply_to: <son id>` ;
- `reponse` construit le corps de la réponse ;
- `etat_dans_la_liste` lit l'état d'une tâche.

La vue (`vue.rs`) et la boucle (`tui.rs`) s'en servent, et le témoin e2e aussi.

## Les témoins (le rouge d'abord)

Écrits avant le code, contre des stubs, ils étaient rouges :
- les trois témoins de `boite.rs` ;
- l'e2e : « en 120 s, pas d'accord en attente avec l'équipier en waiting : [], état None ».

Ils sont verts depuis :

| Témoin | Ce qu'il vérifie |
|---|---|
| `boite::tests` (3) | ce qui attend jusqu'à sa réponse ; le corps de la réponse ; l'état trouvé par dépôt et nom |
| `vue::tests::what_the_teammate_waits_for_is_shown_and_answered_from_the_keyboard` | la ligne d'état, `o` et `n`, la saisie qui répond, puis « attend la boîte » |
| `e2e_tty::le_terminal_accorde_une_commande_que_l_equipier_attend` | le vrai chemin de l'hôte, détaillé ci-dessous |

Le témoin e2e joue un vrai hôte sur un port, avec son jeton, et un faux équipier scripté :
1. `ls` passe par la liste sûre ;
2. `echo accorde-moi` attend l'accord ;
3. le terminal voit l'approval en attente et l'état `waiting` ;
4. il accorde, puis l'hôte écrit `granted`, la commande se joue, le pas réussit et la liste dit `ended`.

Un premier vert était faux. La garde des résidus de `run_e2e.sh` a trouvé dans le TMPDIR le journal du dernier pas, écrit 250 ms après la fin du run. Le témoin vide maintenant `journal_des_runs()` avant de rendre son dossier.

Les chiffres :
- le crate passe 39/39 avec la vue et 10/10 sans ;
- `e2e_tty` passe 4/4, avec la validation VERT ;
- `e2e_catalogue_gabarits` passe 11/12, avec son seul rouge nommé (`docs/tty/tickets/2026-11-13-un-gabarit-masque-puis-retire-ne-revient-pas-a-la-bibliotheque.md`).

Tout cela a été joué à luciepc, sur la lib `e93710f36`.
