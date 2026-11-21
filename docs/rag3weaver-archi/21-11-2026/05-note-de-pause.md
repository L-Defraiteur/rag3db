# Note de pause, 21 novembre 2026 à 15 h 30

*rag3weaver archi, avant le redémarrage de la tablette (consigne de c7).*

## Ce qui tourne

Rien, ni à la tablette ni à luciepc. Mes dernières passes à luciepc sont finies ou arrêtées par leurs pids :
l'attestation caduque de l'ancien `fermeture-garde-la-tete`, son confineur orphelin, et la pile de la fermeture après
le SIGTERM de 15 h 11.

## Ce qui a atterri aujourd'hui

`ide-agents-2` (GET /ide/agents), `run-du-pas`, `garde-claude-name`, `nom-d-enfant-slug`, `ide-run` (GET /ide/run),
`e2e-equipier-faux-modele`, `campagne-lanceur-7`, et les pages `agent-neutre-2`, `mode-auto-contextuel-4`,
`mode-survivant`, `ticket-e2e-equipier`.

## Ce qui attend

| Branche | Tête | État | Prochaine action |
|---|---|---|---|
| `fermer-sans-detruire-3` | fca037ece2 | en file au pousseur, vert (e2e_demon_hote 27/27, modules, check_features 8/8) | attendre son train |
| `fermeture-garde-la-tete-2` | 2bad9a85ea | parent `fermer-sans-detruire-3` ; rouge vu, vert à la tablette (280) ; **pas de verdict à luciepc** : son e2e a tourné dans la boucle du confineur, puis a été tué par le SIGTERM de 15 h 11 (code 143) | **rejouer après l'atterrissage de `confiner-poste-classes`** : e2e_demon_hote, modules, check_features, puis déposer avec ce parent |
| `card` | 33647d6f93 | rouge vu sur ses assertions (à part, le commit du témoin ne compilait pas seul) ; vert à la tablette (daemon_host:: 282) ; pas attestée | après l'annonce du confineur : attester à luciepc, déposer (parent -) |

Pas de validation ni d'e2e avant l'annonce de c7, même après la reprise.

## Ensuite, dans l'ordre

1. Le titre d'humain, côté hôte : `by: {kind, client, session}` posé par l'hôte d'après qui appelle ; `user_token`
   en 0600 sous `~/.local/share/rag3weaver/user/` ; l'en-tête `X-Rag3weaver-User` (tty fait le terminal).
2. La page « un client MCP, plusieurs hôtes » (routage par la cible d'abord).
3. La page 03 mise à jour en un seul dépôt : tout ce que Lucie a dit depuis 9 h 40 (suppression dans les worktrees,
   appel de l'agent et `in_reply_to`, la frise et la carte unique, `open_root` en lecture par défaut, `declares`, le
   bac qui ne monte que les racines, `by` et le titre d'humain), la porte en graphe accordée avec ED (machine à
   évènements, `postpone`, `cancel_prefix`).
4. Puis le code de la porte : la Card branchée à l'équipier (l'accord générique pour tty), l'étage 0, RuleClass,
   ResolveEvidence, la porte en graphe.
5. Les renommages promis : `appel_d_outil` → `tool_call`, le test en
   `the_teammate_plays_a_request_end_to_end_on_a_fake_model`, `nom_d_enfant` → `child_name`, `lire`/`ecrire` →
   `read`/`write` dans agents.rs ; les cas Gemini d'e2e_equipier en mesures.

## Worktrees tenus

Sous `~/git_workspaces/archi/` : `20-11-2026-passes-tablette` (mes passes, sans target depuis que c7 l'a effacé),
`21-11-2026-card`, `21-11-2026-fermer-sans-detruire`, et ceux des lots atterris, à retirer à la reprise
(agent-neutre, e2e-equipier-faux-modele, garde-claude-name, ide-run, mode-auto-contextuel, mode-survivant,
nom-d-enfant-slug, run-du-pas, ticket-e2e-equipier). À luciepc : `~/git_workspaces/archi/19-11-2026-passes`
(target delais-c).
