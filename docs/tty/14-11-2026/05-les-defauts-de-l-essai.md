# tty-9 : les cinq défauts vus à l'essai de Lucie

Session tty, 14 novembre 2026. En rejouant l'essai de Lucie contre l'hôte du portable (`04-l-essai-en-une-minute.md`), cinq défauts du terminal ont paru, qu'aucun test ne voyait. Les écrans de l'essai, sans leurs couleurs, sont dans `mesures/essai-reel-tmux-tty8.txt` (avant) et `mesures/essai-reel-tmux-tty9.txt` (après).

| Défaut | Pourquoi | Ce qui change |
|---|---|---|
| Sous tmux, la demande s'affiche **deux fois**, alors que la boîte de l'hôte n'en a qu'une | Le terminal relit la boîte (`GET /ide/teammate`) tant que le flux ne la relaie pas, et la ligne de la demande arrivait par les deux chemins, selon une course | Une ligne de boîte s'écrit une fois par son `id` |
| La ligne d'état dit **« fini »** dès l'ouverture | La relecture rend toute l'histoire de la tâche, y compris le `ended` du run précédent | Une demande, une consigne ou une réponse de toi après la fin remet l'état à « attend » |
| `report` et `step_done` restent en anglais | Ces deux genres manquaient à la traduction de tty-8 | `compte rendu`, `étape faite`, ainsi que `plan revu` (`edit_plan`), `erreur` et `parti` (`started`) |
| Une ligne longue se **coupe au milieu d'un mot** (« son code sont c / eux de ce commit ») | La prose se repliait caractère par caractère, comme le code | La prose (texte du modèle, boîte, outils, notes) passe à la ligne entre deux mots, l'espace de la coupure tombe, et seul un mot plus long que la largeur se coupe ; le code se replie tel quel |
| La phrase du repli sous mosh est **coupée par le début** à 110 colonnes (« que ne se relit pas… ») | Une phrase et une commande trop longues ensemble, gardées par leur fin | La phrase et la commande si elles tiennent, sinon la commande seule, entière |

## Les témoins

Les cinq tests ont été vus rouges, chacun pour sa vraie raison, avant le correctif :
- `a_mailbox_line_received_twice_is_written_once` ;
- `an_earlier_end_followed_by_a_request_is_no_longer_finished` ;
- `the_teammate_kinds_are_all_in_french` ;
- `prose_folds_between_words_and_only_cuts_a_word_longer_than_the_width` ;
- `too_narrow_for_its_sentence_the_fallback_keeps_its_whole_command`.

Les tests du crate passent de 58 à 63 avec la vue, et restent 14 sans elle.

L'essai réel, rejoué sous tmux avec le binaire de tty-9, montre la demande une seule fois et la ligne d'état « attend », tous les genres en français et la prose repliée entre les mots.

## Un défaut du harnais, trouvé au passage

Le harnais entier, rejoué sous ssh (`mesures/tty9-ssh.json`), est d'abord rouge : « Ctrl-G : 2 stop au lieu d'un ; Échap : 0 ». Le premier stop y est daté 6 ms avant l'envoi de Ctrl-G. Les horloges des deux postes s'écartent de quelques millisecondes, et luciepc était chargé à 17. Le harnais rangeait les stops de part et d'autre de l'instant exact où il envoie Échap, de sorte qu'un stop d'Échap arrivé en 5 ms, daté 6 ms trop tôt, comptait pour Ctrl-G. La frontière est maintenant au milieu de la seconde qui sépare les deux touches. Après ce correctif, la passe est verte : un stop chacune, et la saisie au clavier reçue exacte.

## Ce qui ne relève pas du terminal

L'équipier rend un compte rendu au lieu du contenu demandé : c'est le ticket `docs/tty/tickets/2026-11-14-l-equipier-rend-un-compte-rendu-au-lieu-du-contenu-demande.md`, pour archi.
