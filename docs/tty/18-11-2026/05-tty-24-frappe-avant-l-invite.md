# tty-24 : une frappe tapée avant l'invite d'une commande `!` ne se garde plus

Session tty, 18 novembre 2026. Ticket `docs/tickets/closed/2026-11-15-une-frappe-tapee-avant-l-invite-d-une-commande-se-voit.md`.

## Le défaut

Une commande `!` tourne dans un pseudo-terminal à elle, dont l'écho reste ouvert tant que la commande ne le coupe pas. Un mot de passe tapé avant que `sudo` coupe l'écho était renvoyé par le pseudo-terminal, et entrait dans la sortie gardée, donc dans la consigne envoyée à l'équipier et dans le journal. Le module promettait pourtant : « ce qui se tape n'est jamais gardé, seulement ce que la commande écrit ».

## Le correctif (`src/coquille.rs`)

- Avant de relayer une frappe, le terminal lit l'état d'écho du pseudo-terminal (`tcgetattr` sur le maître). La disposition de `struct termios` est celle de Linux, ou de macOS et des BSD. Si la lecture échoue, l'écho est tenu pour ouvert, du côté du secret.
- Écho ouvert, il note l'écho qu'il attend (`ExpectedEcho`) : les octets imprimables tels quels, une fin de ligne en `\r\n`. Quand la sortie revient, cet écho est retiré de la sortie gardée ; l'écran le montre toujours, comme tout terminal. Un caractère de contrôle, dont l'écho diffère (`^C`, l'effacement), arrête l'attente ; la sortie se garde alors telle qu'elle vient.
- Le maître du pseudo-terminal est fermé dans les processus fils (`FD_CLOEXEC`) : une commande n'hérite plus de son propre maître.

Pistes du ticket écartées :
- retenir les frappes tant que l'écho est ouvert bloquerait toute invite qui lit écho ouvert (`read a`) ;
- dire seulement, dans la consigne, qu'une frappe a précédé l'invite laisserait le secret dedans.

## Les témoins

- `coquille::tests::a_password_typed_before_the_prompt_never_reaches_the_kept_output` : un faux `sudo` qui coupe l'écho après 300 ms, un mot de passe tapé tout de suite. La commande le lit (« lu 16 caractères »), mais il n'est ni dans la sortie gardée ni dans la consigne.
- `coquille::tests::what_is_typed_is_shown_but_not_kept` : une réponse tapée écho ouvert se montre à l'écran (« Continuer ? oui ») et ne se garde pas ; la sortie de la commande se garde entière.
- Les deux ont été vus rouges (e4bad4a1cf), puis verts.

En les jouant en parallèle, les tests du pseudo-terminal se gênaient : `TIOCSCTTY` refusé (« Operation not permitted »), ou une ligne jamais lue. La cause : la session et le terminal de contrôle sont un état propre au processus entier, après des `fork` concurrents, et des descripteurs se transmettaient sans `CLOEXEC`. Les quatre tests qui lancent une commande se prennent donc un verrou (`PTY`) en tête ; le terminal lui-même ne lance qu'une commande `!` à la fois. Une fois le verrou posé, le module est passé 20 fois sur 20 en parallèle, et le crate entier 5 fois sur 5.
