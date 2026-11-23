# Relancer l'agent quand son compte rendu affirme ce que les faits démentent

- **État** : ouvert (23 novembre 2026, 06 h), rag3weaver archi ; pour le matin (c7).
- **Famille** : produit
- **Gravité** : l'utilisatrice lit « ajout d'un test dédié » alors qu'aucun fichier n'a changé.
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (`daemon_host/teammate.rs`, la fin d'un tour).
- **Pour** : archi.
- **Signalé par** : c7, banc et archi, la nuit du terminal.

## Ce que c'est

Depuis le 23 nov., la fin de chaque tour porte une ligne de faits de l'hôte (`turn_changes` : fichiers changés d'après git ou la copie, commandes en échec). Il reste la relance (c7, 23 nov. 01 h 50) : quand le compte rendu prétend un ajout, une modification ou un test vert, et que les faits disent « aucun fichier modifié » ou « commande introuvable », l'hôte relance une fois l'agent avec ce fait. Juger le texte du compte rendu sans juge de modèle, ou avec : à concevoir.
