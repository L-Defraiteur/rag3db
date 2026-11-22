# La classe d'une commande par son effet, à poser

- **État** : ouvert (22 novembre 2026, 23 h 30), rag3weaver archi ; reporté au matin par c7 (le MVP de la nuit).
- **Famille** : moteur
- **Gravité** : aucune tant que le mode auto fait confiance au bac ; c'est le retour du juge en auto qui l'attend.
- **Atteignable en service** : non : `judge_in_auto` est faux par défaut.
- **Touche rag3weaver** : oui (`daemon_host/gate.rs`, `templates/gate/rules.json`).
- **Pour** : archi.
- **Signalé par** : c7 et archi, la nuit du terminal.

## Ce que c'est

La branche `effect-classes` (0e936bbd03) classe une commande par son effet : locale par défaut dans le bac, quel que soit son programme ; seuls l'envoi, la destruction de ce qui n'est sur aucun distant, le système et une base vont au juge, avec des mots qui vivent dans `rules.json`. Reportée au matin du 23 par c7 (le MVP de la nuit : en auto, tout ce que le bac lance passe). À rejouer contre master, rouge vu à f41499f08a, puis à déposer avec `judge_in_auto`.
