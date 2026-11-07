# search_structured : le score d'une ligne n'est pas comparable d'un signal à l'autre

- **État** : ouvert (7 novembre 2026), pour recherche. Relevé par rag3weaver archi au 3 ter des hooks.
- **Gravité** : réponse trompeuse. Un appelant qui seuille sur le score d'une ligne seuille au hasard.
- **Atteignable en service** : oui. Tout appelant de `recall_fiches` ou `recall_apports` qui lit
  `chunk.score`, comme le hook UserPromptSubmit jusqu'au 3 ter (b).
- **Touche rag3weaver** : oui, `templates/tools/search_structured.mmd` et le rendu de ses lignes.

## Ce que c'est

Une ligne du rendu porte trois scores qui ne se comparent pas :
- `score` est le rang fusionné, de l'ordre de 0,01 ;
- `chunk.score` est le score du morceau affiché, venu du signal qui a choisi ce morceau ;
- `signals[]` donne `{label, rank, score}` par signal : `vector`, une similarité de 0 à 1, et
  `bm25`, un score brut sans borne.

Relevé au journal des runs de l'hôte en service, backend `c6dd17bae70f`, mémoire de deux fiches :

| heure | fiche | `chunk.score` | `signals` |
|---|---|---|---|
| 18:16:13 | hooks-essai | 1,66 | vector 0,446 ; bm25 1,660 |
| 18:16:13 | régime GPU | 0,35 | vector 0,351 |
| 18:16:39 | régime GPU | 0,37 | vector 0,374 ; bm25 1,386 (rang 1) |
| 18:16:39 | hooks-essai | 0,46 | vector 0,459 |

À 18:16:39, le plein texte place « régime GPU » au premier rang, mais `chunk.score` rend le
dense, 0,37, parce que le morceau affiché est le résumé trouvé par le dense. Un seuil sur
`chunk.score` l'a tue. À 18:16:13, le même champ valait 1,66, un score brut, pour une fiche que
la requête ne nommait pas. Le dense donnait 0,32 à 0,37 à « régime GPU » sur trois tours, hors
sujet comme en plein sujet : seul, il ne sépare pas « en rapport » de « sans rapport ».

## Ce qui est fait ailleurs

Le hook (3 ter b) lit `signals` par ligne : le plein texte fait le candidat, et le dense seul
n'est pas jugé tant qu'il n'est pas mesuré. Il contourne le défaut, il ne le corrige pas.

## Ce qui reste

Un score par ligne comparable entre signaux, ou un champ qui dit sur quelle échelle est
`chunk.score`. Par exemple, chaque signal normalisé sur sa propre distribution, ou un
`best_signal: {label, score}` explicite. Un seuil dense ou un écart relatif se mesurera ensuite
sur une mémoire peuplée, à partir de prompts réels étiquetés « en rapport » ou « sans rapport ».
