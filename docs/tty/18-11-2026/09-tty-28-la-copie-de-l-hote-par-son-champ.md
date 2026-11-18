# tty-28 : la copie de l'hôte se reconnaît par son champ

Session tty, 18 novembre 2026. Ticket `docs/tickets/closed/2026-11-16-la-copie-de-l-hote-se-reconnait-par-un-champ.md`.

## Ce qui a changé côté hôte

À la fin d'un pas sans `step_done`, l'hôte pose la réponse du modèle dans la boîte avec `origin: "host_copy"`, sans le préfixe français d'avant (archi, `pas-va-au-bout`, `src/daemon_host/teammate.rs`).

## Le défaut trouvé en lisant le champ

La règle de tty-22 ne réaffichait pas une copie **égale** au texte streamé depuis le départ du run. Or ce texte couvre tout le run, alors que la copie ne porte que la dernière réponse du modèle. Un run qui dit « Je lis le fichier. », appelle un outil, puis répond « Voici la réponse. » faisait réafficher « Voici la réponse. ».

## Le correctif (`src/vue.rs`, `ligne_de_boite`)

- Une ligne `origin: "host_copy"` ne se réaffiche pas quand le texte streamé du run la **contient**, aux blancs près (`contains_text`).
- Sans texte streamé (un terminal ouvert après le run), elle s'affiche comme la réponse de l'agent.
- Les autres comptes rendus gardent la règle de l'égalité, et le préfixe français reste reconnu pour les boîtes anciennes.

## Le témoin

`vue::tests::the_host_copy_is_known_by_its_field_and_not_shown_again_when_streamed`, rouge commité avant le code (7db7ac3be1 : la copie de la dernière réponse se réécrivait), puis vert. Le crate passe 119 + 2 tests, et 22 sans vue ; clippy `-D warnings` ne signale rien.
