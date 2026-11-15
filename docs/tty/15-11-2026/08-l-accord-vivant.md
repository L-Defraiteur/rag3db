# tty-16 (second morceau et suite) : o / n sur une proposition, et l'accord vivant

Session tty, 15 novembre 2026.

## o / n sur une proposition de déclarations (488d17d11)

Le terminal s'abonne aussi au sujet `declarations` du flux. Il ne retient que les `PropositionChanged` du dépôt principal, par le `backend` de l'enveloppe.

- Une proposition `proposed` attend sur la ligne d'état : « attend ton accord : proposition ⟨id⟩ (chemins) — o / n ». La ligne ajoute « — refusée par la validation » quand `accepted` est faux.
- `applied` ou `rejected` la retire, d'où qu'il vienne.
- Saisie vide et rien dans la boîte : `o` poste `/ide/propositions/appliquer`, `n` poste `/ide/propositions/rejeter`, avec la raison « rejetée depuis le terminal ».
- Une demande d'accord de la boîte passe avant la proposition.

Témoin : `a_declarations_proposal_waits_on_the_status_line_and_o_or_n_answers_it`. Il a été vu rouge avant le code, puis par mutation sur le bras `PropositionChanged` de `trame`.

## L'accord vivant (2504c33e4)

Lucie, sur le binaire d'essai, a vu « [équipier · accord demandé] commande en attente d'accord : git -c core.excludesfile="" status ». La ligne d'état montrait l'animation de `run_command` (« SIGNAL REÇU… »), pas « attend ton accord ». Sa règle : « les accords demandés, je dois pas les voir défiler comme ça, ça doit être vivant et pas rester dans les lignes de mon terminal ».

- **La demande ne s'écrit plus** dans l'historique.
- **La ligne d'état la dit, sans animation.** La cause de l'écran de Lucie : `ligne_vivante` jouait l'animation dès que l'état en avait une, et ce qui attendait de toi ne passait qu'avant le texte de l'état. Désormais, une attente de la boîte ou une proposition coupe l'animation.
- **La réponse laisse une ligne**, « › accordé : <commande> » ou « › refusé : <commande> ». Elle s'écrit au retour de la réponse dans la boîte, et non à la frappe. Une réponse donnée dans l'IDE laisse donc la même ligne.

Témoin : `an_approval_request_lives_in_the_live_zone_and_leaves_one_result_line`. Il a été vu rouge sur « la demande ne s'écrit pas ». Le crate passe 101 tests, et clippy `-D warnings` ne signale rien.

## `git -c … status` n'est pas libre, et c'est juste

`git status` est sur la liste sûre de l'hôte (`LECTURE_SEULE`, `src/commande.rs`). L'équipier a lancé `git -c core.excludesfile="" status` : `famille()` y lit « core.excludesfile= » pour verbe, et la commande demande un accord. Ce refus protège : `git -c` fait exécuter du code (`core.fsmonitor`, `core.pager`, un alias). Il y a deux pistes, à trancher par Lucie et la session archi, puisque c'est du code de l'hôte :

- reconnaître une courte liste de clés `-c` inoffensives ;
- dire à l'équipier de lancer `git status` nu.

Je recommande la seconde.
