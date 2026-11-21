# La fermeture d'un index étiqueté perd sa génération

- **État** : ouvert (21 novembre 2026).
- **Gravité** : un invariant de lucivy 4.4.0 cassé, sans perte. `ShardedHandle::close` commite chaque shard
  directement (`ShardMsg::Commit`), sans le payload `lucivy-generation:<g>` : sur un index étiqueté, le `meta.json` de
  chaque shard perd sa génération. `revert_to` n'en souffre pas, puisqu'il lit les copies `meta.<g>.json`, mais
  `tagged_generations()` compare la génération de la racine au payload de chaque shard, et dirait alors un retour coupé
  là où il n'y en a pas.
- **Atteignable en service** : oui, à chaque fermeture d'un plein texte étiqueté par rag3weaver (décision (A) du
  21 novembre 2026), dès que le lot des générations par paquet sera posé.
- **Touche rag3weaver** : non (lucivy) ; rag3weaver n'appelle pas `tagged_generations()` aujourd'hui.
- **Famille** : moteur
- **Pour** : lucivy
- **Signalé par** : lucivy, en écrivant le lot (A) dans rag3weaver (une passe, puis l'inspection du dossier)

## Le correctif

Tout commit interne de lucivy sur un index étiqueté (`close`, et tout autre `ShardMsg::Commit` hors de
`commit_inner`) prend le payload de la génération courante, comme `commit_keeping_generation`. Témoin rouge d'abord :
`commit_tagged(1)`, `close`, réouverture, puis `tagged_generations()` rend `([1], 1)` au lieu d'une erreur. Dans une
4.4.1, publiée au mot de Lucie.
