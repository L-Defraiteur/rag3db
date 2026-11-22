# `ps` échoue dans le bac, faute de /proc

- **État** : ouvert (23 novembre 2026, 01 h), rag3weaver archi ; reporté au matin par c7 (le MVP de la nuit).
- **Famille** : moteur
- **Gravité** : `ps` sort en erreur (« mount -t proc proc /proc ») : un agent qui veut voir ses processus de fond ne le peut pas.
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (le bac, `commande.rs`).
- **Pour** : arbre, archi.
- **Signalé par** : archi, la nuit du terminal.

## Ce que c'est

Relevé à blanc la nuit du 22 au 23 par archi. Landlock refuse la lecture de `/proc`. Pas cette nuit, sauf si un essai réel cale dessus (c7). Pistes : `/proc/self` et les processus de la tâche en lecture, ou un outil qui liste les commandes de fond de la tâche (il existe : `tail` sur la poignée).
