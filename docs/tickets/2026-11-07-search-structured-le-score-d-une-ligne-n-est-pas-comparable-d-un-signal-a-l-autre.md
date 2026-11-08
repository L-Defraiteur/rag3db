# search_structured : le score d'une ligne n'est pas comparable d'un signal à l'autre

- **État** : ouvert (7 novembre 2026), pour archi depuis le 8 novembre : le rendu dit maintenant l'échelle de chaque score et le signal qui porte la ligne (lot best-signal de recherche) ; restent `recall_min_score` du hook et le seuil mesuré. Relevé par rag3weaver archi au 3 ter des hooks.
- **Gravité** : réponse trompeuse. Un appelant qui seuille sur le score d'une ligne seuille au hasard.
- **Atteignable en service** : oui. Tout appelant de `recall_fiches` ou `recall_apports` qui lit
  `chunk.score`, comme le hook UserPromptSubmit jusqu'au 3 ter (b).
- **Touche rag3weaver** : oui, `templates/tools/search_structured.mmd` et le rendu de ses lignes.
- **Famille** : produit

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

## Fait le 8 novembre 2026 (recherche, lot best-signal)

- `chunk.signal` (et `chunks[].signal`) : l'étiquette du signal dont vient `chunk.score`, posée par
  `FuseResultsNode`. `chunk.score` ne se compare qu'à un score du même signal.
- Le morceau d'une ligne fusionnée se choisit par la contribution de chaque signal au rang,
  poids / (k + rang), et non plus par le plus grand score brut (`fuse_signals`, search.rs) : un
  bm25 à 1,6 au troisième rang ne l'emporte plus sur un dense à 0,4 au premier. Le choix du
  morceau ne touche ni le score ni l'ordre : rien ne monte que les rangs ne portaient pas. Les
  surlignages d'un même morceau vu par deux signaux restent fusionnés.
- `bestSignal: {label, score, rank}` par ligne : le signal qui pèse le plus dans le rang fusionné
  (les signaux en rôle `boost` n'y comptent que si aucun ne fusionne) ; `RerankNode`, quand il
  réordonne la tête, y met sa ligne. `ResolveParentNode` et l'enrichissement du reranker le
  gardent (`Provenance`).
- Aucun score « absolu » n'est inventé : un min-max ou un rang par requête ne se compare pas d'une
  requête à l'autre.
- Témoin : `le_morceau_d_une_ligne_suit_le_rang_pas_l_echelle_du_score`
  (generic_search_nodes.rs).

## Ce qui reste, pour archi

- `recall_min_score` du hook (`claude_code_hook.rs`, réglage documenté comme un seuil sur
  `chunk.score`) n'est plus lu par la logique : le relier à `bestSignal`, ou le retirer du schéma
  avec son commentaire.
- Le seuil dense, ou un écart relatif, se mesure sur une mémoire peuplée, à partir de prompts réels
  étiquetés « en rapport » ou « sans rapport ».
