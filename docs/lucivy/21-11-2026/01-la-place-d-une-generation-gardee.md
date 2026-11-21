# La place d'une génération gardée du plein texte

Session lucivy, 21 novembre 2026, à la question de Lucie : « si lucivy savait partager un index entre plusieurs
générations, ce serait fou, pour économiser la place disque ».

## Ce que fait lucivy 4.4

Il partage déjà, par construction. Une génération gardée n'est qu'un `meta.<g>.json` de quelques Ko, qui nomme ses
segments. Un segment ne change jamais une fois écrit : tant que les générations suivantes le gardent, c'est **le même
fichier** pour toutes. Une génération gardée ne retient en plus que ce que les suivantes ont remplacé :
- les segments que les fusions et la compaction ont réécrits ;
- les anciens fichiers de retraits (`.del`) d'un segment qui a reçu des retraits ;
- l'ancienne génération du dictionnaire de shard (`dict-<g>`), que la compaction du dictionnaire réécrit.

Entre worktrees, c'est l'autre direction : un index par dépôt, des vues par worktree.

## La mesure

Sur une copie de l'index ScopeContent du dépôt (instantané de G, 08:37, hôte arrêté), avec lucivy 4.4.0 seule, par une
sonde jamais commitée :

| Étape | Taille | Écart |
|---|---|---|
| Départ | 1 015 Mo (104 917 documents) | |
| Un `commit_tagged`, rien changé | 1 021 Mo | +6,3 Mo |
| Une maintenance avec compaction, la génération 1 gardée | 2 015 Mo | +993 Mo, en 31 s |
| La génération 1 relâchée (`release_below`, puis un commit) | 1 006 Mo | |

## Ce que cela dit

- **Garder une génération quand peu change coûte peu** : quelques Mo, le méta, les statistiques et ce que le commit
  réécrit.
- **Une compaction réécrit tout l'index** : tant que la génération d'avant est gardée, elle retient l'index entier
  (~1 Go ici). La place revient au relâchement.
- D'où la règle de la voie B : `release_below` dès que la base a validé de façon durable. Avec la génération par
  paquet (décision (A) du 21 novembre), la fenêtre se compte en secondes, et un paquet ordinaire ajoute quelques Mo.
- La compaction est le pire cas. Elle est rare : maintenance nocturne, ou explicite. À la faire tourner, il faut garder
  de la place pour un index de plus, le temps d'une génération.

## Les deux questions de Lucie, après la mesure

### « Les lucids pourraient pas aider à ça ? »

Les « lucids », ce sont LUCID et LUCIDS, les formats de delta incrémental de lucivy : l'export des seuls segments
changés d'un shard (LUCID), ou des seuls shards changés (LUCIDS), pour synchroniser une copie distante (client et
serveur, WASM hors ligne). Ici, ils n'aident pas : ils réduisent ce qu'on **transfère**, pas ce qu'on **garde** sur le
disque local. Une compaction réécrit tous les segments : son delta est l'index entier. Et une génération gardée est déjà
un delta, au sens où elle ne retient que les segments que les suivantes ont remplacés.

### « Rien à faire, généralement, pour qu'une compaction évite les doublons ? »

**Ce que fait lucivy aujourd'hui.** `compact(max_docs)` passe shard par shard, fusionne les segments de chacun par
tours jusqu'à n'en garder qu'un (au plus `compact_max_docs` documents, 10 millions par défaut, donc tout), puis
compacte le dictionnaire de chaque shard. Il commite une seule fois, à la fin. Sans génération gardée, le surcoût est
celui d'une fusion à la fois : chaque fusion achevée retire les segments qu'elle remplace. Avec une génération gardée,
tout reste jusqu'au relâchement, d'où les +993 Mo.

**Un point à corriger dans (A), trouvé en écrivant ceci.** Une compaction ne crée pas de version de la base : elle
commite sous la génération courante (`commit_keeping_generation`). Or `release_below(g)` ne relâche jamais `g`
elle-même. Après la maintenance de la nuit, l'ancien index entier resterait donc retenu par `meta.<g>.json` jusqu'au
paquet suivant d'une synchronisation, qui étiquette `g+1` puis relâche `g`. Le remède : clore une compaction par une
génération à elle, `commit_tagged(g+1)` et la marque `fts_generation = g+1` validée en base, puis `release_below(g+1)`.
Le lot de (A) doit le faire, avec un témoin : après une maintenance avec compaction, la place revient sans attendre une
synchronisation.

**Une compaction par étapes (la piste de c7), confirmée.** Clore chaque étape par sa génération et son relâchement
borne la place en plus à l'étape, au lieu de l'index entier :
- **par shard** : compacter un shard, étiqueter, valider la marque, relâcher, puis le suivant. La borne est d'un shard
  et de son dictionnaire, environ un quart de l'index (~250 Mo ici, à mesurer). C'est le plus simple, car lucivy
  compacte déjà shard par shard : il faut un `compact_shard(i)`, ou un filtre de shards ;
- **plus fin** : quelques segments à la fois dans un shard, chaque tour clos par sa génération. La borne est d'une
  fusion, mais le dictionnaire de shard se réécrit d'un bloc : sa compaction n'est pas découpable aujourd'hui, et il
  devient la borne ;
- **le coût** : les mêmes fusions, donc à peu près la même durée (31 s ici pour l'index entier), plus un commit
  étiqueté et une validation de la marque en base par étape (quelques dizaines de ms chacune). L'étiquette reste à
  l'appelant, parce que `g` vient de la base : lucivy donne `compact_shard(i)`, et rag3weaver enchaîne les étapes.

Rien n'est codé : à relire d'abord.
