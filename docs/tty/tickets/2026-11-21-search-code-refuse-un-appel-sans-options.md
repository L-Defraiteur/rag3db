# `search_code` refuse un appel sans `options`, sur la phrase du schéma

- **État** : ouvert le 21 novembre 2026 par la session tty, dans les essais de la nuit (scénario 1). Pour ED (c7) : `options` facultatif, avec ses défauts.
- **Famille** : produit
- **Gravité** : un premier appel d'un agent ou d'un humain échoue sans dire quoi mettre.
- **Atteignable en service** : oui, par le client MCP et par l'IDE.
- **Touche rag3weaver** : oui (le schéma de `search_code`).

## Ce que c'est

Par `rag3weaver-backend mcp`, `search_code {query: "resoudre_depot"}` répond seulement « "options" is a required property ». Avec `options: {limit: 3}`, la recherche répond, et bien.

## Contournement

Passer `options: {}` (ou `{limit: …}`).

## Ce qu'il faut

`options` facultatif, avec ses défauts (`limit`, les signaux de l'entité cible) ; à défaut, un refus qui dit la forme attendue.
