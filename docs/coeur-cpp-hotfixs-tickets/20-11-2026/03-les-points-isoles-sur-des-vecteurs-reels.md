# Les points isolés d'un index HNSW, mesurés sur des vecteurs réels

Le 20 novembre, à la demande de c7, pour le ticket du banc
`docs/banc/tickets/2026-11-19-les-points-les-plus-isoles-d-un-index-hnsw-ne-sont-pas-atteints.md`. Une mesure, sans
aucun changement du moteur.

## Ce qui a été mesuré

- **Les vecteurs** : les 127 198 vecteurs de `Scope_Chunk.embedding__granite_278m` (granite-278m, FLOAT[768], tous de
  norme 1), lus sur une copie par reflink de l'instantané `avant-4ad394f4d-20261120/code-lourd` de G, ouverte en
  lecture seule et exportée par `COPY … TO`. Comme les vecteurs sont normés, le classement par l2 du test est celui du
  cosinus de rag3weaver.
- **Le test** : `VectorIndexSeedMeasure` du banc, avec une variable de plus, `BANC_VECTORS_CSV` (branche
  `seed-measure-real-vectors`, 8caebb63e9, posée sur `isolated-point-found-first-measure`). Les 200 dernières lignes
  servent de requêtes, hors de la table ; les 126 998 autres sont indexées. Le rappel global à k = 10 est jugé contre
  une recherche exhaustive. Les 200 points les plus isolés d'un échantillon de 2 000 sont cherchés par leur propre
  vecteur.
- **Le moteur** : la branche de mesure, qui porte l'amorce séquentielle (`seed_points`). 8 fils, Release, à luciepc,
  sous `poste lourd` (donc des temps pris sous charge, à titre indicatif seulement).

## Résultats

| seed_points | CREATE (ms, sous charge) | rappel@10 | points isolés dans le top 10 |
|---|---|---|---|
| 0 | 57 857 | 0,999 | 200/200 |
| 1000 | 67 254 | 0,9985 | 200/200 |
| 0 | 61 631 | 0,998 | 200/200 |
| 1000 | 55 555 | 0,998 | 200/200 |

**Sur des vecteurs réels, le défaut du ticket ne se montre pas, même sans amorce** : aucun des 200 points les plus
isolés n'est manqué, et le rappel reste à 0,998 ou 0,999. L'amorce n'y change rien de mesurable, ni en rappel ni en
temps (les écarts de temps sont dans le bruit de la charge).

Le défaut ne se voit donc que sur la base tirée du banc : des vecteurs gaussiens de dimension 768, de dimension
intrinsèque maximale, où les points les plus éloignés le sont vraiment. Des embeddings de code ont une structure bien plus
ramassée.

**Une réserve sur la colonne « premier »** du test (le point cherché revient en tête) : elle n'a valu que 12 à 22 sur
200, alors que chaque point est dans le top 10 et à distance nulle de lui-même. `QUERY_VECTOR_INDEX … RETURN node.id`
sans `ORDER BY` ne garantit pas l'ordre des résultats : cette colonne ne mesure pas la qualité de l'index. Seule la colonne
du top 10 compte.

## Ce que j'en tire

- L'amorce (6a5595c950) reste : elle corrige le cas tiré, et ne coûte rien de mesurable sur les vrais vecteurs.
- Le ticket du banc peut se fermer pour les données réelles, ou passer en faible gravité, au choix du banc et de c7 :
  il ne touche pas les bases de code d'aujourd'hui.
- Rien à changer au moteur.

La copie de mesure (`~/.cache/rag3weaver-build/mesures-hotfixs/code-lourd-points-isoles-20261120` à luciepc) est marquée
jetable (famille `copie-de-mesure-hotfixs`).
