# put_fiche rend un chemin de miroir nul alors que le miroir est écrit

- **État** : ouvert (21 novembre 2026, session recherche, tri du fichier des ratés du MCP).
- **Gravité** : gêne : l'appelant ne sait pas où lire la fiche
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (le serveur MCP de la mémoire longue).
- **Famille** : produit
- **Pour** : mémoire
- **Signalé par** : codeparsers (12 h 34), dans `~/.local/state/rag3weaver/mcp-misses.tsv`.

## Le pas exact

`put_fiche` (création de la fiche « Les occurrences des références au graphe ») répond `"file": null`, alors que le
miroir `~/memoire-longue/fiches/<uuid>.md` est écrit (4595 octets, 12 h 33).

## Pour le fermer

La réponse porte le chemin du miroir écrit ; un témoin : une fiche créée rend son `file`, qui existe sur le disque.
