# Les petites cardinalités : les fils d'un plan, et le balayage sous masque — la mesure avant le code

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

## 6. L'instrument (11 octobre, 20 h 45, branche `fils-proportionnes`, a3e5e5dd7)

Avant la règle, de quoi lire où vont les fils : la cardinalité estimée descend du plan logique au
plan physique quand le planificateur l'a posée (`LogicalOperator::hasCardinalityEstimate`, le 1
par défaut ne dit rien ; `PhysicalOperator::getEstimatedCardinality`, vide sinon), et chaque tâche
se dit à sa fin sous `RAG3DB_TRACE_TASKS=1` : « [tache] source=<opérateur> card=<estimée|?>
puits=<opérateur> fils=<venus>/<donnés> duree=<ms> ». À lire sur la mesure de F
(`e2e_mesure_impact`, `RUST_TEST_THREADS=1`, `poste mesure`) : sur quelles tâches les 32 fils se
mettent en route pour rien, et si l'estimation y est juste. Résultats : section 8.

## 7. Le second cas, trouvé par F : le balayage entier sous un petit masque **[lu, plan de F]**

Sur la copie de l'index entier de rag3db (7 310 fichiers), le saut d'`usages` —
`UNWIND $uuids AS u MATCH (d:Symbol {_uuid: u})<-[r:MENTIONS]-(m:Scope) RETURN m…` — se planifie
ainsi (PROFILE) : `PRIMARY_KEY_SCAN` de d (1 ligne), l'extension MENTIONS (117 arêtes), puis un
`HASH_JOIN` dont le côté Scope est un **`SCAN_NODE_TABLE` de toute la table sous `SEMI_MASKER`** :
**71 657 lignes sorties pour 117 retenues**, 7 ms de balayage et 5 ms de sonde à 2 colonnes ; à 9
colonnes, 420 à 470 ms par appel. L'estimation est juste (117) : c'est l'opérateur qui lit tout.

Ce que fait le moteur : l'optimiseur de jointure accumulée (`acc_hash_join_optimizer.cpp`,
`tryApplySemiMask`) pose un `SemiMasker` sur le côté construit et masque le balayage du côté
sondé par les identifiants déjà connus (`SemiMaskTargetType::SCAN_NODE`). Le masque est **par
décalage** (`SemiMask::mask(offset)`, `range(start, end)` rend les décalages masqués d'un groupe),
et le balayage le consomme **par groupe de nœuds** (`NodeGroup::scan`, `state.semiMask->range`) :
il ne saute un groupe que s'il n'y a aucun décalage masqué dedans, et lit sinon les colonnes du
groupe entier (131 072 lignes) pour en garder quelques-unes. Cent dix-sept scopes éparpillés sur
une table de 71 657 lignes touchent presque chaque groupe : le balayage lit tout, et son coût suit
la taille de la table et le nombre de colonnes, pas le travail.

- **PostgreSQL** : une boucle imbriquée avec accès par index ou par TID sous un petit ensemble,
  choisie au plan sur le coût (le hachage de la table entière ne gagne que si l'ensemble est
  grand).
- **Neo4j** : l'expansion rend des identifiants de nœuds, et les propriétés se lisent **par
  identifiant** (accès direct au magasin), jamais par balayage de l'étiquette.

**Forme [déduit, à mesurer]** : quand le masque est petit (quelques centaines de décalages, seuil
sur la cardinalité estimée du côté construit, comme pour les fils), le côté sondé **ne balaye pas** :
il lit les lignes masquées **par décalage** (`NodeTable::lookup` sur les décalages du masque,
groupe par groupe, les seules colonnes demandées), et au-delà du seuil il balaye comme aujourd'hui.
Mesure avant/après : la copie de l'index entier de F, `usages` et son saut en 2 et 9 colonnes, lignes
lues (PROFILE) et temps par appel ; attendu : de 71 657 à 117 lignes lues, et `usages` de 400 ms à
quelques dizaines de millisecondes. Ticket : par F, avec son plan collé ; pris après les fils.

## 8. Les mesures de la nuit (11-12 octobre) **[lu]**

**Le petit masque, par le banc** (`MaskedScan.MeasureAtOneAndThirtyTwoThreads`, poste principal,
`poste mesure`, master b808a3bee, cinq passes par nombre de fils ; le témoin
`MaskedScan.ReadsAboutTheMaskNotTheTable` est en known_red, étiquette `scan-reads-the-mask`) :

| fils | lignes sorties du `SCAN_NODE_TABLE` sous le masque | durée médiane | min / max |
|---|---|---|---|
| 1 | 67 584 | 5,72 ms | 5,50 / 6,52 |
| 32 | 67 584 | 5,25 ms | 5,22 / 5,38 |

Le nombre de fils ne change ni les lignes lues ni, presque, la durée (8 % à 32 fils pour 117
lignes utiles) : **le coût est le balayage lui-même, pas sa parallélisation** — la lecture par
décalages (§7) visera ≈ 117 lignes. Un écart entre postes (67 584 ici, 69 632 sur luciepc, même
code et mêmes données) : le découpage du balayage par blocs, sans effet sur la borne.

**La trace des fils** (`RAG3DB_TRACE_TASKS`, a3e5e5dd7) : la mesure de F n'a pas encore couru —
sur luciepc, une heure à attendre derrière les lourds (poste corrigé depuis) ; ici, bloquée par le
`poste lourd` intérieur de `run_e2e.sh` sous `poste mesure` (défaut à G) ; relancée à 0 h 02 sous le
seul lourd de `run_e2e.sh` : résultats ci-dessous dès qu'ils sortent.

**La trace des fils, lue (12 octobre, 0 h 10 ; ici, sous le lourd de `run_e2e.sh`, lib
rag3db-build-a3e5e5dd7, mesure d'impact de F : 150 appels, 3 620 tâches, 3 038 ms de moteur).**

| tâches (source → puits, fils donnés) | n | durée | part | fils venus |
|---|---|---|---|---|
| SCAN_NODE_TABLE → HASH_JOIN_BUILD, 32 | 1 250 | 852 ms | 28 % | ~11 |
| TABLE_FUNCTION_CALL → RESULT_COLLECTOR, 32 (card. inconnue) | 477 | 578 ms | 19 % | ~9 |
| TABLE_FUNCTION_CALL → BATCH_INSERT, 32 (l'ingestion) | 7 | 443 ms | 15 % | ~31 |
| AGGREGATE_FINALIZE, 32 | 223 | 278 ms | 9 % | ~21 |
| AGGREGATE_SCAN → RESULT_COLLECTOR, 32 | 211 | 273 ms | 9 % | ~7 |
| TABLE_FUNCTION_CALL → AGGREGATE, 32 (card. inconnue) | 207 | 231 ms | 8 % | ~10 |
| TABLE_FUNCTION_CALL → RESULT_COLLECTOR, 1 | 593 | 213 ms | 7 % | 1 |
| PRIMARY_KEY_SCAN → RESULT_COLLECTOR, 1 | 469 | 26 ms | 1 % | 1 |

- **91 % du temps dans les tâches dotées de 32 fils** (2 501 tâches ; ~11 fils viennent vraiment ;
  1,1 ms par tâche contre 0,24 ms pour une tâche à 1 fil) ; 9 % dans les tâches à 1 fil.
- Parmi les tâches à 32 fils : **63 % ont une cardinalité estimée ≤ 2 048** (1 588 : les balayages
  de construction des jointures, les agrégats), **37 % n'en ont pas** (913, source
  `TABLE_FUNCTION_CALL` : le planificateur ne pose rien sur un CALL).
- Le lecture : **« moins de tâches » n'est pas le levier** (24 tâches par appel sont la forme du
  plan) ; **les fils proportionnés** couvrent tout de suite ≈ 55 % du temps (les tâches à
  estimation connue) ; le second pas est que **les fonctions de table déclarent leur estimation**
  (`QUERY_VECTOR_INDEX` connaît son k, un CALL simple rend une ligne), pour que la règle les
  voie — sans quoi elles gardent le plafond, par construction.
- Référence `[mesure]` avant la règle, sur ce poste, fils par défaut : joindre p2 55,2 ms, p3
  67,5 ms ; est_local p2 54,7 ms, p3 80,4 ms ; servir_lie p2 57,5 ms, p3 68,7 ms (médianes de 5).

## 9. La règle, et le avant/après (12 octobre 2026, 0 h 20 à 2 h) **[mesuré]**

**La règle** (branche `fils-proportionnes`, 6ffec5a82 (master dda63783a) ; `QueryProcessor::initTask`) : la plus grande
cardinalité estimée le long du pipeline, un fil par tranche de 2 048 lignes
(`DEFAULT_VECTOR_CAPACITY`), sous le plafond `threads` de la connexion (`Task::limitNumThreads`
ne fait que réduire : un `CALL threads=1` reste 1) ; un opérateur sans estimation garde le plafond.
Ce que le témoin `ThreadsByCardinality` a exigé pour que la règle voie les pipelines :

1. **les puits nés dans le mapper** sans contrepartie logique — l'agrégat, la construction d'une
   jointure, le collecteur de la racine — héritent l'estimation de leur enfant
   (`PlanMapper::inheritEstimates`, `createResultCollector`) ;
2. **l'optimiseur ne remettait les cardinalités à jour que pour EXPLAIN LOGICAL** (le commentaire
   de l'amont : « on ne s'en sert pas après la planification ») ; la jointure que pose la descente
   des filtres (`UNWIND … MATCH (n {id: x})`) naissait sans estimation → le `CardinalityUpdater`
   tourne après toute optimisation, et un enfant sans estimation en prive son parent ;
3. `UNWIND [1, 2, 3]` n'est pas un littéral mais un appel de `LIST_CREATION` :
   `LogicalUnwind::knownListSize()` (littéral, liste écrite, paramètre — `UNWIND $uuids`) sert au
   planificateur et au remetteur à jour ; le balayage factice vaut 1 ; une fonction de table ne
   pose `numRows` que s'il est connu.

La trace (`RAG3DB_TRACE_TASKS`) est lue à l'ouverture de la base, et nomme le premier opérateur
sans estimation (`sans=…`).

**Le avant/après**, même mesure (e2e_mesure_impact de F, ici, `poste mesure`, trace allumée des
deux côtés, RUST_TEST_THREADS=1, embarquement par les tunnels ; avant = a3e5e5dd7 l'instrument
seul, après = 6ffec5a82 (master dda63783a) ; les empreintes `[mesure]` sont identiques des deux côtés — mêmes
résultats) :

| `[mesure]` (médiane de 5) | avant | après |
|---|---|---|
| joindre, profondeur 2 / 3 | 55,2 / 67,5 ms | **26,6 / 29,8 ms** |
| est_local, profondeur 2 / 3 | 54,7 / 80,4 ms | **24,1 / 32,2 ms** |
| servir_lie, profondeur 2 / 3 | 57,5 / 68,7 ms | **25,3 / 30,3 ms** |

| `[phases]` (3 450 instructions à paramètres) | avant | après |
|---|---|---|
| total des appels d'impact | 10 343 ms | 4 748 ms |
| prepare, par instruction | 0,779 ms | 0,405 ms |
| execute, par instruction | 2,044 ms | 0,896 ms |
| le moteur dit : compilation / exécution | 0,642 / 1,648 ms | 0,338 / 0,656 ms |

| tâches tracées (toute la suite) | avant | après |
|---|---|---|
| tâches, temps cumulé | 69 132, 54 379 ms | 69 134, 27 119 ms |
| à 32 fils donnés | 46 581 (80 % du temps, ~11 fils venus) | 182 (7 %, ~18 venus : l'ingestion) |
| à 1 fil donné | 22 551 (20 %) | 68 064 (85 %) |
| à 2 fils donnés | — | 885 (8 %) |
| sans estimation (plafond gardé) | — | BATCH_INSERT 81, RESULT_COLLECTOR 82, CREATE_TABLE 127, PARTITIONER 19… |

- **Les appels d'impact sont 2,1 à 2,5 × plus courts** ; le temps cumulé des tâches est divisé par
  deux sur toute la suite. Les tâches à 32 fils ne sont plus que celles qui le méritent
  (l'ingestion, ~18 fils venus), et presque tout passe à un fil.
- **Que `prepare` et la compilation baissent aussi** (0,78 → 0,41 ms ; 0,64 → 0,34 ms) alors que
  la règle ne touche pas la planification dit que le coût d'avant n'était pas seulement dans les
  tâches : trente-deux fils réveillés pour trois lignes se disputaient le poste avec le fil qui
  prépare l'instruction suivante. Réserve : les deux passes ne sont pas à charge contrôlée (la
  nuit des autres sessions) ; le banc rejoue sa mesure du petit masque avant/après sous une seule
  tenue `poste mesure`, à lire à côté.
- **Le banc** (×10 probabilistes vectoriels, cherry-pick sur b808a3bee) : 73 verts / 7 rouges
  avant, 76 / 4 après — rien dans le mauvais sens ; les quatre ProductReloadRecovery
  (`threads=1` explicite) inchangés. **L'arbre principal** (e2e du produit sur 6ffec5a82 (master dda63783a), bâti à
  part, artefact prouvé par ldd + md5) : 131 tests, 0 rouge ; e2e_code 160 s contre 198.
- Ce qui reste hors de la règle, par construction : ce qui n'a pas d'estimation (les
  fonctions de table : `QUERY_VECTOR_INDEX` connaît son k, un CALL simple rend une ligne — le
  second pas de §8), et le balayage sous masque (§7, autre lot).

## 10. Le balayage sous un petit masque : ce que fait le code, et la forme **[lu, à coder]**

**Ce que fait le code aujourd'hui** (lecture du 12 octobre, master dda63783a) :

- Le masque (`SemiMask`, `common/mask.h`, bitmap Roaring) est créé par `mapScanNodeTable` à la
  taille de la table (`createMask(numTotalRows)`), un par table dans `ScanNodeTableSharedState`,
  rempli par le `SemiMasker` (un masque local par fil, `fastunion` à la fin), branché par
  `mapSemiMasker`. Son API : `isMasked`, `range(start, end)` (les décalages posés, triés),
  `getNumMaskedNodes()`, `collectMaskedNodes(n)` ; pas de min/max exposés (le `roaring` l'a).
- **Le morsel d'un balayage est un groupe entier** (2^17 lignes) ; le masque n'intervient pas dans
  le choix des groupes. Dans `NodeGroup::scan`, par fenêtre de 2 048, `applySemiMaskFilter` fusionne
  les décalages masqués dans la sélection ; **si la fenêtre n'en a aucun, elle est sautée ; sinon
  `ChunkedNodeGroup::scan` écrase la sélection** (`setToUnfiltered(numRowsToScan)`, ou celle des
  versions) et lit les 2 048 lignes de toutes les colonnes. Le masque ne sert donc qu'à sauter les
  fenêtres vides : 117 décalages tous les 615 touchent presque toutes les fenêtres → ≈ N lignes
  lues et sorties (le fait du §7). Le filtrage réel se fait au `HASH_JOIN_PROBE`. Forme de
  l'amont (2024).
- **Un chemin par décalages existe** : `NodeTable::lookupMultiple(tx, state)` (node_table.cpp:422)
  parcourt la sélection de `nodeIDVector`, ne réinitialise l'état qu'au changement de groupe ou de
  source, écrit chaque ligne à sa position ; COMMITTED et UNCOMMITTED. Personne ne l'appelle pour
  les nœuds (seules les relations). `PRIMARY_KEY_SCAN` et `INDEX_SCAN` font un `lookup` par ligne.
- **La cardinalité du masque est exacte à l'exécution** : le pipeline du masqueur est finalisé avant
  celui du balayage ; dans `ScanNodeTable::initGlobalStateInternal` (ou
  `ScanNodeTableSharedState::initialize`), `semiMask->isEnabled()` et `getNumMaskedNodes()` sont
  connus, contre `table->getNumTotalRows(tx)`. Pas besoin d'estimation : le vrai nombre.

**La forme** : dans `ScanNodeTableSharedState`, au moment de `initialize`, si le masque est actif et
petit — `getNumMaskedNodes() × SEUIL < numTotalRows` (seuil 64 : un décalage par 64 lignes, soit
moins d'une ligne utile par fenêtre de 2 048 sur 32) — le balayage passe en **mode décalages** :
les morsels sont des tranches de ≤ 2 048 décalages du masque (`range` par pas, ou l'itérateur
roaring), chaque fil remplit `nodeIDVector` (sélection = la tranche) et appelle `lookupMultiple` ;
`numOutputTuple` compte les lignes rendues. Sinon, le chemin d'aujourd'hui. Les lignes non visibles
(versions) sont rendues nulles par `lookupMultiple` — à retirer de la sélection avant de sortir (la
valeur de retour compte les nuls comme des échecs : à lire, pas à croire). Le témoin du banc
`MaskedScan.ReadsAboutTheMaskNotTheTable` (known_red `scan-reads-the-mask`) demande
`NumOutputTuples ≤ 10·k` sous le `SCAN_NODE_TABLE` : la lecture par décalages le rend ; réparer
seulement le saut par fenêtre ne suffirait pas (117 décalages touchent toutes les fenêtres).
Mesure avant/après : `MaskedScan.MeasureAtOneAndThirtyTwoThreads` (5,07 / 5,24 ms à 1 / 32 fils
sur le lot des fils, 67 584 lignes lues) et la mesure d'usages de F (367 à 399 ms en médiane sur la
copie de l'index entier). Garde : le témoin lit le premier `NumOutputTuples` sous
« SCAN_NODE_TABLE », que `PRIMARY_KEY_SCAN_NODE_TABLE` contient aussi — à resserrer au banc.
