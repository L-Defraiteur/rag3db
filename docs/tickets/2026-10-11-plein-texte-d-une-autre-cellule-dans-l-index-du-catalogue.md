# Le plein texte d'une fiche écrite dans une autre cellule tombe dans l'index de la cellule du catalogue

- **État** : ouvert (11 octobre 2026), à fermer par le pas b de la marche 1 bis (les index par cellule,
  `rechercher` sans bascule ; page `extension/rag3weaver/docs/11-octobre-2026-arbre-principal/04-la-marche-1-bis.md`).
- **Gravité** : réponse fausse (P1 de la stèle du produit) : une recherche d'une session peut rendre une
  fiche d'une autre cellule.
- **Atteignable en service** : sous l'hôte du démon seulement, depuis le pas a (`9dd4baedd`) : deux
  sessions sur deux cellules d'un même backend. Un serveur MCP par session, qui n'a qu'une cellule,
  n'y arrive pas.
- **Touche rag3weaver** : oui (le catalogue, la recherche).

## Ce que c'est

Depuis le pas a, une session écrit ses lignes dans sa cellule : `_org` / `_project` justes en base, et sa
lecture par identité ne voit qu'elles. Mais le plein texte de ces lignes s'écrit dans l'index ouvert pour la
**cellule du catalogue** (`fts_handles`), pas dans celui de la cellule de la session. Et la recherche ne
filtre pas encore par la cellule de la session. Une session alpha peut donc trouver par ses mots une fiche
écrite par une session beta.

## Pour le fermer

Le pas b : l'ingestion d'un lot d'une autre cellule écrit dans les index de cette cellule, et la recherche
prend les index et le filtre de la cellule de sa requête, sans basculer la cellule du catalogue. Le témoin :
deux sessions sur deux cellules, chacune ne trouve par ses mots que ses fiches.
