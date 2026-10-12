# Les fils d'un plan sont proportionnés à la cardinalité estimée par le planificateur

- **État** : fait (12 octobre 2026, le commit qui déplace ce fichier dans `closed/` : la règle dans `QueryProcessor::initTask`, le témoin `ThreadsByCardinality`, le avant/après en page coeur-cpp/12 §9 — appels d'impact 2,1 à 2,5 × plus courts). Ouvert le 11 octobre 2026, 20 h 45 ; né de la mesure de F, page
  `extension/rag3weaver/docs/11-octobre-2026-embarquements/06-ou-va-le-temps-d-un-aller-retour.md`,
  `821d70a63`). Propriétaire : le cœur C++. Page du lot :
  `extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/12-les-fils-d-un-plan-proportionnes-a-la-cardinalite.md`.
- **Gravité** : performance — une lecture d'outil qui touche quelques lignes (impact, usages, liens)
  reçoit autant de fils d'exécution que de cœurs (32 sur luciepc) ; à 1 fil, `impact` va 47 % plus
  vite (3 855 ms contre 7 336 ms pour 150 appels), l'exécution d'une instruction passe de 1,55 à
  0,73 ms. La mise en route des fils coûte plus que le travail.
- **Atteignable en service** : oui, à chaque instruction du produit.
- **Touche rag3weaver** : oui (tout appel d'outil ; l'ingestion et les COPY, à ne pas ralentir).

## Ce que fait le moteur aujourd'hui (lu le 11 octobre)

`QueryProcessor::execute` découpe le plan en tâches (une par puits) ; chaque `ProcessorTask`
naît avec `maxNumThreads` = le réglage `threads` de la connexion (`ClientConfig::numThreads`,
par défaut `SystemConfig::maxNumThreads`, tous les cœurs). Le seul réglage du parallélisme est
binaire : `initTask` met la tâche à un fil si un de ses opérateurs n'est pas `isParallel()`
(un SET, un DELETE, un tri, un `CALL` non parallélisable). Sinon, l'ordonnanceur laisse jusqu'à
`maxNumThreads` ouvriers s'enregistrer sur la tâche, quelle que soit la taille de ce qu'elle lit :
un balayage de trois lignes est découpé en morceaux pour 32 fils, et chacun se met en route pour
rien. La cardinalité estimée par le planificateur existe (`LogicalOperator::cardinality`, posée par
`CardinalityEstimator` aux balayages, extensions, jointures, agrégats) mais **ne descend pas** au
plan physique ni aux tâches.

## Ce que font les moteurs établis

- **PostgreSQL** décide le parallélisme **au plan, sur le coût estimé** : un nœud `Gather` n'est
  envisagé que si la relation dépasse `min_parallel_table_scan_size` (8 Mo), le nombre d'ouvriers
  croît par paliers avec la taille (un de plus par ×3), plafonné par
  `max_parallel_workers_per_gather`, et le plan parallèle n'est retenu que si son coût, qui compte
  `parallel_setup_cost` et `parallel_tuple_cost`, bat le plan en série. Une lecture de trois lignes
  n'a jamais d'ouvrier.
- **Neo4j** n'a pas de parallélisme automatique sur une petite requête : le runtime `pipelined`
  (le défaut) exécute une requête sur un fil ; le runtime `parallel` est choisi par requête, pour
  l'analytique lourde, et découpe par morceaux sur les ouvriers.

Chez nous : tout ou rien, et « tout » par défaut.

## Forme proposée (à mesurer avant)

Porter la cardinalité estimée de la source d'une tâche (et de sa sortie) sur le `ProcessorTask`,
et en déduire `maxNumThreads = clamp(ceil(cardinalité / lignes par fil), 1, threads)`, où « lignes
par fil » est un morceau (de l'ordre du vecteur, 2 048 lignes, ou du groupe de nœuds) — à fixer
par la mesure. Le réglage `threads` reste le plafond ; une tâche à un opérateur non parallèle
reste à un fil ; une cardinalité inconnue garde le plafond (jamais moins de fils qu'aujourd'hui sur
ce qu'on ne sait pas estimer).

## La mesure, avant tout code

1. F (`RAG3WEAVER_EXEC_THREADS`, `rag3db_connection::phases`, `e2e_mesure_impact`) : l'ingestion
   (`e2e_code`, le chemin de masse) à 1, 4 et 32 fils, pour savoir ce qu'un fil seul coûte aux gros
   travaux — la page 06 l'a proposé, rien n'est mesuré de ce côté.
2. Puis, côté moteur, le même `impact` et la même ingestion **sans réglage**, avant et après la
   forme : l'attendu est le temps à 1 fil pour les lectures d'outil, et le temps à 32 fils pour
   l'ingestion, sans qu'aucun appelant ne pose quoi que ce soit.

## Témoin

À écrire : une table de trois lignes et une de 300 000 ; `EXPLAIN` (ou un compteur de tâche)
dit le nombre de fils donné à chaque tâche ; 1 pour la petite, le plafond pour la grande ; et les
e2e de mesure de F avant/après.
