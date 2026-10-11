# Les fils d'un plan proportionnés à la cardinalité estimée — la mesure avant le code

*11 octobre 2026, 20 h 45, session cœur C++. Lot né de la mesure de F (page
`11-octobre-2026-embarquements/06-ou-va-le-temps-d-un-aller-retour.md`, 821d70a63). Ticket :
`docs/tickets/2026-10-11-les-fils-d-un-plan-sont-proportionnes-a-la-cardinalite-estimee-par-le-planificateur.md`.*

## 1. Le fait mesuré par F **[lu]**

Une instruction d'`impact` lit quelques lignes et reçoit 32 fils d'exécution (tous les cœurs de
luciepc). À 1 fil, 150 appels d'`impact` passent de 7 336 ms à 3 855 ms (−47 %), l'exécution
d'une instruction de 1,55 à 0,73 ms, la préparation de 0,59 à 0,40 ms (moins de contention). Sur
PostgreSQL, par le réseau, la même mesure prend 5 à 11 ms par appel d'`impact` : le coût est
dans le moteur, dans la mise en route des fils.

## 2. Ce que fait le moteur aujourd'hui **[lu]**

- `QueryProcessor::execute` découpe le plan en tâches, une par puits (`decomposePlanIntoTask`).
- `ProcessorTask` naît avec `maxNumThreads` = le réglage `threads` de la connexion
  (`ClientConfig::numThreads`, par défaut `SystemConfig::maxNumThreads` = les cœurs).
- `initTask` met la tâche à **un** fil si un de ses opérateurs n'est pas `isParallel()` (SET,
  DELETE, tri, `CALL` non parallélisable, `IndexScan`…). C'est le seul réglage : tout ou rien.
- L'ordonnanceur laisse les ouvriers s'enregistrer sur la tâche tant que `maxNumThreads >
  numThreadsRegistered` ; chacun clone le pipeline et prend des morceaux du balayage.
- La cardinalité estimée existe au plan logique (`LogicalOperator::cardinality`, posée par le
  `CardinalityEstimator` aux balayages, extensions, jointures, agrégats) et **ne descend pas** au
  plan physique ni aux tâches.

## 3. Ce que font PostgreSQL et Neo4j **[déduit de leurs docs]**

- **PostgreSQL** : le parallélisme se décide **au plan, sur le coût**. Un `Gather` n'est
  envisagé qu'au-delà de `min_parallel_table_scan_size` (8 Mo), le nombre d'ouvriers monte par
  paliers avec la taille (un de plus par ×3), plafonné par `max_parallel_workers_per_gather` (2),
  et le plan parallèle n'est retenu que si son coût, avec `parallel_setup_cost` (1 000) et
  `parallel_tuple_cost` (0,1), bat le plan en série. Trois lignes n'ont jamais d'ouvrier.
- **Neo4j** : pas de parallélisme automatique sur une petite requête ; le runtime `pipelined`
  (défaut) exécute une requête sur un fil, le runtime `parallel` est choisi par requête pour
  l'analytique lourde (morceaux sur des ouvriers).

L'écart chez nous : **tout ou rien, et « tout » par défaut** — là où les deux moteurs établis
donnent zéro parallélisme à une petite lecture.

## 4. La forme **[déduit, à mesurer]**

Porter la cardinalité estimée de la source d'une tâche (et de sa sortie) du plan logique au
`ProcessorTask`, et en déduire :

```
maxNumThreads = clamp(ceil(cardinalité estimée / lignes par fil), 1, threads)
```

- `threads` reste le plafond (le réglage de la connexion ne change pas de sens) ;
- une tâche à un opérateur non parallèle reste à un fil ;
- une cardinalité inconnue (fonction de table sans estimation, `CALL`) **garde le plafond** :
  jamais moins de fils qu'aujourd'hui sur ce qu'on ne sait pas estimer ;
- « lignes par fil » : un morceau — de l'ordre du vecteur (2 048) ou du groupe de nœuds
  (131 072) ; la mesure tranche.

Ce que la forme ne fait pas : elle ne répartit pas mieux les gros travaux (COPY, construction
d'index, balayages) — ils gardent leurs fils ; elle ne change rien à `threads` posé par
l'appelant ; elle ne touche pas à l'ordonnanceur.

## 5. La mesure, avant tout code

1. **F, avec son instrument** (`RAG3WEAVER_EXEC_THREADS`, `rag3db_connection::phases`,
   `e2e_mesure_impact`) : l'ingestion (`e2e_code`, le chemin de masse) à 1, 4 et 32 fils — ce
   qu'un fil seul coûte aux gros travaux ; la page 06 le propose, rien n'est mesuré.
2. **Côté moteur, sans réglage posé** : `impact` et l'ingestion de F, avant et après la forme.
   Attendu : le temps « 1 fil » pour les lectures d'outil (≈ −47 % sur `impact`), le temps
   « 32 fils » pour l'ingestion ; aucun appelant ne pose rien.
3. **Le témoin du moteur** : deux tables (3 lignes, 300 000 lignes), un `EXPLAIN` ou un compteur
   de tâche qui dit les fils donnés : 1 pour la petite, le plafond pour la grande ; et la liste
   complète + le banc (le banc pose `threads` dans ses cas ? à lire : s'il compte sur 32 fils
   pour des petites tables, ses attentes de simultanéité peuvent changer — le dire au banc avant).

Résultats : à venir (section 6), puis le lot.
