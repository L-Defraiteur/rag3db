# Le terminal envoie son PATH à l'ouverture d'un dossier

- **État** : ouvert (23 novembre 2026, 01 h), rag3weaver archi ; reporté au matin par c7 (le MVP de la nuit).
- **Famille** : produit
- **Gravité** : une commande lancée depuis le terminal ne voit pas forcément ce que voit ce terminal (un `nvm use`, un venv activé, un PATH modifié dans la session).
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (`/dossiers/ouvrir`, `rag3weaver-tty`, l'environnement des commandes d'une tâche).
- **Pour** : archi, tty.
- **Signalé par** : archi, la nuit du terminal.

## Ce que c'est

`login-path` fait adopter à l'hôte, à son démarrage, le PATH du shell de connexion de l'utilisatrice : c'est le premier pas. La forme la plus juste (c7, 23 nov.) : le terminal envoie son PATH quand il ouvre un dossier, et les commandes de la tâche tournent avec lui, comme dans le terminal d'où on l'a lancée. À borner : seul un client au titre humain le donne, et il ne vaut que pour la tâche.
