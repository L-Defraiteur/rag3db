# Une relation porte deux valeurs d'une même propriété selon le sens où on la lit

- **État** : ouvert — **bloque la stèle** (orchestration, 4 octobre 2026) ; recette inconnue, défaut intermittent
- **Gravité** : réponse fausse, durable (sur disque, après fermeture et réouverture)
- **Atteignable en service** : oui — vu dans rag3weaver tel qu'il tourne, sans transaction par paquet
- **Touche rag3weaver** : oui (relations CONSUMES et CONSUMED_BY du graphe de code, propriété `resolution`)
- **Ouvert le** : 4 octobre 2026, session du banc, sur le constat de la session de l'arbre principal

## Ce que c'est

Chaque sens d'une table de relations stocke ses propres colonnes de propriétés. Dans une
base de reprise de rag3weaver, 140 relations CONSUMES, et les 140 réciproques
CONSUMED_BY, portent une valeur de `resolution` dans le sens direct et une autre dans le
sens inverse. Une lecture rend donc l'une ou l'autre selon le plan choisi.

## Ce qui est établi (base gardée)

Base : `~/.cache/rag3weaver-build/tx-ligne-a-ligne/reprise-sans-830913-1791144559552361438/base.rag3db`
(à ne pas modifier ; lue sur des copies reflink). Fermée proprement, sans `.wal`. L'écart
est donc **sur disque**, après le point de reprise de fermeture.

- Par le stockage (contrôle de niveau 2 du banc, `stored-rel-properties-agree`) : CONSUMES,
  140 relations, sens direct « fichier », sens inverse « nom » ; CONSUMED_BY, 140 relations,
  sens direct « nom », sens inverse « fichier ». Seule `resolution` diffère ; `line`,
  `usage`, `usages` et les extrémités sont d'accord.
- Les 140 relations ont des identifiants **contigus** (2399 à 2538), les mêmes dans les
  deux tables : un seul lot d'arêtes, validé ensemble. Leurs nœuds ont des décalages 2431
  à 2591, tous dans la même région de 1 024.
- La valeur restée « fichier » est, dans les deux tables, dans les listes rattachées **aux
  mêmes nœuds** (le côté f300_1, source de CONSUMES et cible de CONSUMED_BY) : ce n'est pas
  un sens de MERGE (direct pour CONSUMES, inverse pour CONSUMED_BY), c'est un ensemble de
  nœuds dont les listes n'ont pas reçu la seconde écriture.
- Aucun identifiant de relation en double ; pas de collision.
- La base a subi une reprise : 81 Scope supprimés et 81 relations supprimées (trous dans
  les décalages), puis 160 relations neuves (2399 à 2558).
- Le rejeu du journal est hors de cause pour une mise à jour : `replayRelUpdateRecord` passe
  par `RelTable::update`, symétrique.

## Ce qui n'est pas établi

- L'ordre exact des deux écritures (l'analyseur, puis le « rendez-vous », probablement par
  `batch_link_labeled` : `UNWIND … MATCH … MATCH … MERGE (a)-[r]->(b) SET …`, chaque
  instruction sa propre validation). La session de l'arbre principal trace la paire.
- La recette : sept scènes en Cypher brut (lots MERGE … SET, avec et sans point de
  reprise entre les deux, suppression d'un fichier avant, après, entre, tables réciproques,
  texte exact du produit) restent cohérentes.

## Témoin

- Générique, vert faute de recette : `RelationPropertiesBothWays.*`
  (`uncommitted_relations_test.cpp`), qui vérifie l'accord des deux sens dans le processus
  écrivain puis après réouverture, sous trois régimes de points de reprise.
- L'invariant `stored-rel-properties-agree` est désormais vérifié par tout cas du banc qui
  appelle le contrôle de niveau 2.

## Cause

Inconnue. Piste lue dans le code, non vérifiée : une mise à jour de relation validée passe
par `RelTableData::update` pour chaque sens ; elle retrouve la ligne par
`findMatchingRow` (le nœud de rattachement du sens et l'identifiant de la relation), et un
échec n'y est vérifié que par `KU_ASSERT(rowIdx != INVALID_ROW_IDX)`, éteint en Release.
Si, dans un sens, la ligne n'est pas retrouvée (mauvais nœud de rattachement, ligne dans une
partie de la CSR que le balayage ne voit pas), la mise à jour de ce sens est perdue en
silence. La session cœur C++ pose une sonde qui rend une erreur nommée dans ce cas.

## Pour le fermer

La recette (témoin rouge au banc), la cause, et le correctif ; jusque-là, une mise à jour
qui ne retrouve pas sa relation dans un sens doit échouer par son nom plutôt qu'en silence.
