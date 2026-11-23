# Le témoin des commandes de fond rougit sous la charge de la lib entière

- **État** : ouvert (23 novembre 2026, session recherche, passe de la pile de l'IDE à luciepc).
- **Gravité** : témoin instable
- **Atteignable en service** : non (un témoin).
- **Touche rag3weaver** : oui (`daemon_host::teammate_tests::background`).
- **Famille** : produit
- **Pour** : l'auteur des témoins de l'équipier
- **Signalé par** : recherche.

## Le pas exact

`background_commands_are_bounded_in_number_and_time` a échoué une fois dans `cargo test --lib` entier (3 032 passent, 1
échoue ; luciepc chargée par plusieurs passes, 23 nov. vers 06 h) : la seconde commande de fond, lancée pendant que la
première tourne, revient « **Commande en attente.** `sh` écrit : je ne peux pas l'accorder tout seul … Mode
`Approbation` » au lieu d'être refusée en nommant la borne (`background.rs:155`). Joué seul dans le même target, il est
vert sur master (87e7237669), sur le premier lot de la pile et deux fois de suite sur la tête de la pile (3b501acba4).

## Ce qu'on en lit

Sous la charge, le premier appel n'a pas encore enregistré sa commande de fond quand le second arrive, ou le mode
d'approbation du témoin n'est pas encore posé : le second est jugé par la porte (`sh` écrit) avant la borne.

## Pour le fermer

Le témoin attend que la première commande soit enregistrée comme en cours (par l'état de l'hôte, pas par un délai)
avant de lancer la seconde, et déclare son mode d'approbation avant le premier appel ; rejoué cent fois sous
`stress-ng` sans un rouge.
