# put_apport refuse une section vide que sa description accepte

- **État** : ouvert (21 novembre 2026, session recherche, tri du fichier des ratés du MCP).
- **Gravité** : gêne : un appel suivant la description échoue
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (le schéma de put_apport de la mémoire longue).
- **Famille** : produit
- **Pour** : mémoire
- **Signalé par** : banc (12 h 34), dans `~/.local/state/rag3weaver/mcp-misses.tsv`.

## Le pas exact

`put_apport` avec une `section` vide, pour un apport qui porte sur une source entière, comme la description du champ le
dit (« vide pour le source entier ») : refus « identity field section must be a non-empty string ».

## Pour le fermer

Accorder le schéma et sa description : soit la section vide est acceptée (et veut dire la source entière), soit la
description dit la valeur à donner pour une source entière.
