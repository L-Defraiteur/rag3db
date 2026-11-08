# Un apport accepte une source qui n'est pas le chemin d'une Source

- **État** : ouvert, le 7 novembre 2026. Relevé par la session d'essai des hooks (hooks-essai),
  remonté par l'orchestration ; pour la session mémoire (96), dans M4, où `put_apport` bouge.
- **Gravité** : une provenance fausse. Un apport dont `source` ne nomme aucune Source garde un
  texte sans L0 : la Ref `doc:<source>#<section>` pointe dans le vide, `cited_by` ne le retrouve
  pas depuis un chemin, et une reconstruction depuis les fichiers ne peut pas relier l'apport à un
  document.
- **Atteignable en service** : oui, sur la mémoire de Lucie, par les outils MCP.
- **Touche rag3weaver** : oui, le gabarit memory (`schemas/apport.json`, `put_apport`).
- **Famille** : produit

## Ce qu'on voit

`put_apport {record: {fiche, source: "sessions", section, texte, …}}` est accepté : `source` est
une chaîne quelconque, alors que la description du champ dit « le chemin de la Source
(`sources/<date>-<id>.md`) ». Rien ne vérifie non plus qu'une Source à ce chemin existe, ni à
l'écriture ni dans `relier_apport` (la clé `{fiche, source, section}` sert d'identité sans être
résolue).

## Ce qu'il faut

- Le schéma d'Apport exige la forme d'un chemin de Source (`sources/….md`, ou le chemin que la
  déclaration `files` de Source produit) : un `source` d'une autre forme est refusé en le disant.
- `put_apport` refuse une `source` qui ne nomme aucune Source existante, en disant quoi faire
  (`put_source` d'abord) — ou la crée vide avec `origine: inconnue` si Lucie préfère ne jamais
  perdre un texte ; à trancher avec elle.
- Témoin : un apport à `source: "sessions"` est refusé ; le même avec le chemin d'une Source posée
  passe, et `cited_by <chemin>` le rend.
