# Pas de gabarit `code` en lecture seule : il faut retirer les outils d'écriture à la main

- **État** : ouvert (11 octobre 2026) — trouvé en montant le MCP du code sur l'arbre principal partagé (session codeparsers)
- **Gravité** : friction (chaque montage en lecture seule coûte deux refus et une copie modifiée du gabarit)
- **Atteignable en service** : oui, pour tout dépôt qu'un agent doit lire sans le modifier
- **Touche rag3weaver** : oui (gabarit `code`, validation du manifeste)

## Ce que c'est

Un `workspace` en `read_only: true` avec `commands: false` est refusé tant que le manifeste
déclare `edit_file`, puis `run_command`. Les refus sont nommés et justes, mais le gabarit livré
déclare ces outils. Il faut donc copier le gabarit et retirer les outils un par un, refus après
refus.

## Ce qu'il faut pour le fermer

Un gabarit le déclare. Au choix :
- un gabarit `code-lecture` livré à côté de `code` ;
- ou les outils d'écriture marqués `requires: write` (et `requires: commands`) dans le
  gabarit, que le chargement retire d'eux-mêmes quand le `workspace` est en lecture seule, en
  le disant une fois au démarrage.
