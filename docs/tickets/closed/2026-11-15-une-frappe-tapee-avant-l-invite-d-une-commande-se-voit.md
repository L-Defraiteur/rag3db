# Une frappe tapée avant l'invite d'une commande `!` se voit

- **État** : corrigé. Lot tty-24 (18 nov. 2026) : l'écho d'une frappe faite écho ouvert se montre et ne se garde plus ; page `docs/tty/18-11-2026/05-tty-24-frappe-avant-l-invite.md`.
- **Famille** : produit
- **Gravité** : gêne, et un risque léger : un mot de passe tapé trop tôt s'affiche à l'écran, et entre dans la sortie gardée, donc dans la consigne envoyée à l'équipier.
- **Atteignable en service** : oui, si l'on tape le mot de passe d'un `!sudo …` avant que `sudo` ait affiché son invite (et coupé l'écho).
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty`, `src/coquille.rs`).

## Ce que c'est

Une commande `!` tourne dans un pseudo-terminal à elle, dont l'écho est ouvert jusqu'à ce que la commande le coupe : `sudo` le coupe au moment où il demande le mot de passe. Une frappe arrivée avant ce moment est renvoyée par le pseudo-terminal, comme par tout terminal : elle s'affiche, et elle entre dans la sortie gardée, puisque le terminal garde tout ce que la commande écrit, l'écho compris.

## La recette

Taper `!sudo true`, Entrée, puis tout de suite le mot de passe, avant l'invite « [sudo] password ». Le test `coquille::tests::a_password_read_with_echo_off_never_reaches_the_kept_output` le montre si on lui retire son attente de 500 ms : le secret paraît dans la sortie.

## Pistes

- ne garder de la sortie que ce qui suit la dernière invite de mot de passe reconnue, ou retirer d'une sortie l'écho d'une frappe qui précède une coupure d'écho (le terminal sait ce qu'il a relayé) ;
- lire l'état d'écho du pseudo-terminal (`tcgetattr` sur le maître) avant de relayer une frappe, et retenir les frappes tant que la commande n'a pas lu ;
- au moins, dire dans la consigne qu'une frappe a précédé l'invite.
