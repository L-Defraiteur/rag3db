# Un mot qui commence par deux tirets est lu comme une négation dans search_code

- **État** : ouvert (21 novembre 2026, session recherche, tri du fichier des ratés du MCP).
- **Gravité** : réponse incomplète : le signal par mots tombe, seul le vecteur répond, sans le dire à l'appelant.
- **Atteignable en service** : oui (toute recherche d'une option de ligne de commande : `--daemon`, `--features`…).
- **Touche rag3weaver** : oui (le nœud `BM25SearchNode`, la lecture de la requête).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : rag3weaver archi (15 h 56), dans `~/.local/state/rag3weaver/mcp-misses.tsv`.

## Le pas exact

`search_code` avec « client mince MCP désigne un hôte distant par daemon-json-command ou adresse --daemon » : le mot
`--daemon` est lu comme une négation ; la requête par mots est refusée (« a query cannot be only a negation » au
nœud BM25 pour la partie restante), et seul le vecteur répond.

## Pour le fermer

- Un mot de la requête qui commence par `--` (une option de ligne de commande) est cherché comme un littéral, pas
  comme une négation ; la négation reste `-mot` en tête de mot, ou une syntaxe explicite.
- Un signal qui tombe est dit dans la réponse (« recherche par mots refusée : … »), jamais en silence.
- Témoin : la requête « adresse --daemon » trouve, par mots, un fichier qui contient `--daemon`.
