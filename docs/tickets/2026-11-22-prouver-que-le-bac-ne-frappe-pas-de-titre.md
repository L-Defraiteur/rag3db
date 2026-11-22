# Prouver que le bac ne frappe pas de titre humain

- **État** : ouvert (22 novembre 2026, 23 h 30), rag3weaver archi ; reporté au matin par c7 (le MVP de la nuit).
- **Famille** : produit
- **Gravité** : aucun témoin ne prouve encore qu'un agent du bac ne peut ni lire le titre ni frapper un code.
- **Atteignable en service** : la borne existe (dossiers protégés), sans preuve.
- **Touche rag3weaver** : oui (le bac, `/ide/entry-code`).
- **Pour** : archi, arbre.
- **Signalé par** : c7 et archi, la nuit du terminal.

## Ce que c'est

Le plancher protégé couvre le dossier de l'hôte (`daemon.json`, `user/user_token`), son état, `~/.config/rag3weaver` et les profils des navigateurs. Témoin à écrire : depuis le bac d'un équipier, lire `user_token` et `daemon.json` échoue, `POST /ide/entry-code` répond 403, le profil du navigateur est illisible. Hors du bac (une autre session dans le shell de Lucie), rien n'est promis : le titre ne distingue l'humaine que des agents du produit. Une carte `protected` sur le dossier de l'hôte doit dire « ce dossier contient ton titre et le jeton de l'hôte ».
