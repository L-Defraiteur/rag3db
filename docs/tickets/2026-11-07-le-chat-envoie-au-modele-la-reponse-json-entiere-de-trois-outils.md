# Le chat envoie au modèle la réponse JSON entière de read_file, grep_files et list_files

- **État** : ouvert (7 novembre 2026), relevé par hooks-essai en lisant le chemin des retours
  d'outils du chat (ticket de la mise en page humaine).
- **Famille** : produit
- **Gravité** : coût et bruit. Le modèle lit `delivery`, `validation`, `run` et `metadata` vide
  autour du résultat. Le JSON coupé par la borne de 32 Kio arrive invalide.
- **Atteignable en service** : oui, tout appel de ces trois outils par le chat de l'IDE.
- **Touche rag3weaver** : oui (`src/backend_tools.rs`).

## Ce que c'est

`BackendTools` (`src/backend_tools.rs:17-30`) prend `presentation` quand la réponse en a une
(c'est le cas de `search_code`), sinon `result.to_string()` : **la réponse entière**, pas son champ
`result`. Pour read_file, grep_files et list_files, au format `markdown` par défaut, le modèle reçoit
donc par exemple :
`{"delivery":{"ok":true,"results":[]},"freshness":{…},"metadata":{},"result":"<le Markdown>","run":"…","validation":{…}}`.
Une coupure à `max_tool_output_bytes` tombe au milieu de ce JSON.

## Ce qu'il faut pour le fermer

Quand `result` est une chaîne, le modèle reçoit cette chaîne, précédée de la ligne de fraîcheur
quand il y en a une, et suivie des métadonnées non vides, sous la même forme que `search_code`.
Le reste de l'enveloppe reste au journal du run. Témoin : le retour de read_file reçu par un faux
modèle commence par le Markdown du fichier, et ne contient ni `delivery` ni `validation`.
