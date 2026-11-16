# L'affichage dans tty des messages entre sessions

- **État** : ouvert, le 16 novembre 2026 ; à faire par Codex (passation d'archi).
- **Famille** : produit
- **Pour** : tty (rédigé par archi pour la passation)
- **Gravité** : fonction manquante, liée au mode pair.
- **Atteignable en service** : non, à construire.
- **Touche rag3weaver** : oui
- **Signalé par** : tty et c7, le 16 novembre 2026

## Ce qu'il faut faire

Côté tty, sur les lignes de boîte du mode pair : le reçu replié en « › Message de @<peer> : <début>…
(Ctrl-O pour tout lire) », l'émis en « “<début>…” → envoyé à <peer> » avec son accusé, l'arrivée et
l'envoi sur la ligne discrète, et Ctrl-O qui ouvre le message entier. Une ligne de copie de l'hôte
(`origin: "host_copy"`, posée par `pas-va-au-bout`) s'affiche comme une réponse de l'agent, pas comme
un défaut.

## Comment le prouver

Les témoins du terminal (dossier `extension/rag3weaver/tty/`). Le mode pair doit exister.

## Taille et blocage

1 jour. Après le mode pair.

