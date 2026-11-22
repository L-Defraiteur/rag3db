# Le réseau ouvert du bac laisse envoyer

- **État** : ouvert (22 novembre 2026, 23 h 30), rag3weaver archi ; reporté au matin par c7 (le MVP de la nuit).
- **Famille** : moteur
- **Gravité** : un code lancé dans le bac (un script `postinstall`, un `npx`, le code de l'agent) peut envoyer des données de l'arbre hors de la machine.
- **Atteignable en service** : oui, en mode auto.
- **Touche rag3weaver** : oui (le bac, `commande.rs`, `backend_code.rs`).
- **Pour** : arbre, archi.
- **Signalé par** : c7 et archi, la nuit du terminal.

## Ce que c'est

Le bac borne l'écriture mais laisse le réseau ouvert (décision de Lucie, 21 nov.) : il ne distingue pas un téléchargement d'un envoi. Les motifs de « envoyer » (`git push`, `curl -d`) ne couvrent pas un script qui poste. Pistes : un bac qui distingue le sortant de l'entrant, ou un proxy du bac qui laisse passer les registres et dit le reste.
