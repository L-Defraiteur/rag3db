# usages rend une classe comme usage de type d'une fonction, en double

- **État** : ouvert (21 novembre 2026, session recherche, tri du fichier des ratés du MCP).
- **Gravité** : réponse fausse
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (les usages de type de l'index de code).
- **Famille** : produit
- **Pour** : codeparsers
- **Signalé par** : codeparsers (12 h 34), dans `~/.local/state/rag3weaver/mcp-misses.tsv`.

## Le pas exact

`usages` de `copier_les_liens` : attendu un appel (`record_nodes.rs:1424`, méthode `execute`) et aucun usage de type ;
rendu l'appel, et deux fois « class LinkRecordNode — record_nodes.rs:1270 » comme usage de type de la fonction.

## Pour le fermer

Un témoin sur ce cas : une fonction libre n'a pas d'usage de type par la classe qui l'appelle, et une même ligne ne
sort pas deux fois.
