# Sur une table indexée par HNSW, chaque écrivain tient l'index en exclusif jusqu'à sa validation : les écrivains passent un par un

- **État** : ouvert
- **Gravité** : blocage, au sens de la règle « aucun bloqué ». Sur une table qui a un index
  vectoriel, deux écrivains ne valident jamais en même temps : chacun attend la fin de la
  transaction de l'autre.
- **Atteignable en service** : oui, c'est le régime de toute table à vecteurs qui reçoit des
  écritures de plusieurs connexions, par exemple pendant une indexation.
- **Touche rag3weaver** : oui. Les tables de morceaux portent un index vectoriel.
- **Ouvert le** : 18 octobre 2026, banc de concurrence, sur arbitrage de l'orchestration (c7).
  C'est l'écart E11 du rapport des migrations concurrentes.
- **Pour** : cœur C++, en fin de file. Ce n'est pas un doublon des deux tickets cités plus bas :
  c'est un écart à « aucun bloqué » en soi, distinct de la lenteur de l'insertion et du point de
  reprise.

## Ce qu'on voit

Le scénario des vingt écrivains (`test/transaction/concurrence/twenty_writers_scenario_test.cpp`)
a 20 écrivains de `Doc`, une table indexée par HNSW à partir de la minute 4, et 2 écrivains de
`Note`, une table sans index. Mesure hors de tout DDL, dans la même passe et au même moment
(`DdlEveryMinute`, luciepc, d147bc1b8, 2 Gio, point de reprise automatique en marche) :

| acteur | médiane | p99 | max |
|---|---|---|---|
| écrivains de `Doc` (indexée) | 21 ms | **390 ms** | 1,2 s |
| écrivains de `Note` (sans index) | 1,9 ms | **24 ms** | 1,2 s |
| lecteurs | 2,9 ms | 27 ms | 1,2 s |

**Après un DDL qui les retient tous, ils repassent en convoi.** À huit fils, le `CHECKPOINT` de la
minute 5 dure 786 ms (9936a4e6d, sous `poste mesure`). Les écrivains de `Note` finissent à 787 ms,
exactement sa durée. Ceux de `Doc` finissent de 1 080 à 1 218 ms : relâchés ensemble, ils
repassent un par un, et le convoi dure 300 à 430 ms. La borne du scénario pour `Doc` compte
désormais ce convoi : écrivains retenus × médiane hors DDL.

## Ce que fait le code

Sur une table qui a un index secondaire chargé, chaque écriture prend `LockResource::index(tableID)`
en `EXCLUSIVE` :
- `NodeTable::insert` (`node_table.cpp:630-638`), sous la marche I1 : l'index en exclusif sur une
  table indexée, en partagé sinon ;
- `NodeTable::lockIndexForWrite` (`node_table.cpp:1199-1209`), pour les SET et les suppressions.

Le verrou n'est rendu qu'à la fin de la transaction, par `LockManager::releaseAll`
(`transaction_manager.cpp:472`). Les transactions d'écriture d'une table indexée sont donc
sérialisées de leur première écriture à leur validation, fsync compris.

## Piste à étudier

- **Tenir le verrou le temps de l'insertion dans le graphe**, et non jusqu'à la validation. Le
  cœur estime cette insertion à 15 à 20 ms par écrivain dans ce scénario.
- **Ou une insertion qui n'exclut pas les autres écrivains** : lecture partagée du graphe, écriture
  par nœud.

Ce qui compte, c'est que la cohérence de l'index sous rétention (forme α du lot de la rétention)
reste prouvée par les témoins existants.

## Liens

- `2026-10-12-les-retrecissements-de-l-insertion-hnsw-alourdissent-son-cout` : le coût d'une
  insertion. Plus elle dure, plus le verrou se tient.
- `2026-10-18-un-point-de-reprise-fige-toutes-les-transactions-le-temps-de-s-ecrire` : le convoi
  qui suit un point de reprise en est un effet de bord.

## Témoin

`HnswWriters.DoNotWaitForEachOther` (`test/transaction/concurrence/hnsw_writers_test.cpp`), dans
`known_red.txt`, en suite ordinaire. La table compte 1 000 lignes indexées par HNSW. A ouvre un
bloc, insère une ligne et garde sa transaction ouverte 1,5 s. B insère une autre ligne en
automatique. Contrôle `writers-of-an-hnsw-table-do-not-wait-for-each-other` : l'insertion de B
finit avant la validation de A.

Le scénario des vingt écrivains le montre aussi, par le terme « convoi de l'index » de sa borne
`checkpoint-holds-writers-at-most-its-duration`, qui tombera avec ce ticket. Le p99 hors DDL de
`Doc` contre `Note` s'y lit à chaque passe.
