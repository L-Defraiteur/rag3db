# Le relevé des vrais appelants : ce que `run_plan` fondrait aujourd'hui

*11 octobre 2026, session embarquements. Docs seuls.*

Pour chaque outil ou chemin du produit qui enchaîne plusieurs formes, la page
donne deux choses :
- les allers-retours d'**un** appel, relevés dans le code (fichier et ligne) ;
- ce que `Catalog::run_plan` (pages 01 et 02) fondrait en une instruction.

Notations :
- **R** : les relations suivies ;
- **D** : la profondeur ;
- **A** : les pas de `also_path` ;
- **T** : les tours de `links` ;
- **C** : les lignes concernées par une réaction.

Le cas mesuré est rag3db, une cellule, lucivy, régime `AuTick`.

## Le tableau

| appelant | allers-retours par appel | fondu par `run_plan` aujourd'hui | ce qui coupe le reste |
|---|---|---|---|
| **impact** (`NeighborhoodNode`, R = 3) | 12 + 6D en cas courant (**24** à D = 2, **30** à D = 3) ; au plus 13 + 9D (40) | la chaîne `also_path`, pas 1 → 2 → 3 (`neighborhood_nodes.rs:270-293`) : **3 → 1** | éventails (R sauts, R degrés, la même entrée) ; nœuds déjà vus, budget, `plus_etroits`, filtre sur le degré, unions par niveau, union finale pour `relever` |
| **links** (crochet de `search_code`) | 10 + 12T (**22** par défaut, 34 à 4 sauts) | rien | éventails de 2R ; frontière filtrée par les carrefours et les étiquettes |
| **impact_fichier** (après chaque `read_file`) | 3 + 1 + R + R·D + 2R(D−1) (**19** à D = 2, 16 en cas courant) | rien : Select fichier → saut est un éventail vers R sauts | comme impact |
| **callees** (R = 1, sortant) | 7 + 2D (**11** à D = 2) | `DEFINES` → saut si `path` est vide : **2 → 1** | nœuds déjà vus, budget, filtre sur le degré |
| **usages** | 3 + 1 + R + 1 + 1 (**9**) | rien : les définitions nourrissent R + 1 suivants | filtre de chemin en Rust ; éventail |
| **recall / recall_subjects**, `rechercher` | **≈ 7**, constant (conditions de vide seulement) ; × le nombre de cellules | rien | le plein texte et le HNSW ne sont pas des formes ; le rescorage et la fusion RRF sont en Rust ; `fetch_entities` et `resolve_*` **interpolent** leurs valeurs |
| **anchor / supersede_link** (`RelationBatchNode`) | **5 à 7**, par tranche de 512 et non par lien | rien | barrière Rust (un bout absent ou hors état fait avorter le lot) ; l'écriture prend `$items` |
| **review_anchored** (réaction) | 1 + C × (lecture, `SET`, points de reprise, 1 à 2 écritures de méta), C ≤ 500 | rien | filtre Rust par état et cellule ; **une mise à jour par ligne**, chacune avec son propre drain |

Le préambule `read_catalog` coûte 3 lectures de méta indépendantes avant chaque
outil. Il est compté plus haut.

## Ce que le relevé dit

1. **Les chaînes pures sont rares.** Une seule vraie chaîne à un seul suivant :
   `also_path` dans impact. La règle actuelle de `run_plan` (l'amont n'a que lui
   pour suivant) ne gagne donc que **2 allers-retours sur 30** à l'appelant le plus
   coûteux.
2. **Le coût vient des éventails.** La même entrée part en R sauts, en R degrés,
   ou en sauts et carrefours, et le Rust réunit les résultats. Dans impact,
   chaque niveau fait R degrés puis 2R sauts : 9 allers-retours, qui en feraient
   **2** si un éventail partait en une instruction.
3. **Le Rust entre deux niveaux n'est pas un défaut.** Nœuds déjà vus, budget,
   filtre sur le degré, barrière de vérification d'un lien : ce sont des choix du
   produit. Ils restent entre les instructions. Fondre à travers eux demanderait
   de les dire en SQL ou en Cypher, et ce serait un autre chantier.
4. **Des allers-retours ne servent à rien aujourd'hui**, sans aucun chaînage :
   - dans impact, les R sauts directs de `usages_of` doublent `saut(departs)` ;
   - dans callees, deux sauts entrants (direct et `MENTIONS`) ne sont jamais lus
     en sortant ;
   - dans review_anchored, C drains en série, alors qu'`UpdateRecordNode` sait
     mettre à jour C lignes en une lecture et un `SET`.

## La recommandation

Avant de brancher un premier appelant, **étendre la fusion aux éventails** : un
nœud qui nourrit plusieurs suivants de même forme, et dont les sorties sont
toutes désignées. C'est l'écart n° 2 de la page 01 (« plusieurs sorties »), levé
par une colonne qui dit de quelle branche vient chaque ligne :
- en Cypher, `UNION ALL` de branches qui rendent les mêmes colonnes, ou
  `UNWIND` d'une liste de relations ;
- en SQL, `UNION ALL` sur la même CTE d'entrée.

Le gain se mesure sur impact :
- un niveau passerait de 3R à 2 instructions ;
- à D = 3, environ **30 → 13** allers-retours, sans toucher au Rust entre les
  niveaux.

C'est un changement de la conception (une instruction rend plusieurs sorties) :
**à Lucie**, avec une page courte. Ce qu'il faut lui rendre, ce sont seulement
les écarts : un planificateur SQL ne rend jamais deux résultats.

Sans attendre, et sans `run_plan`, les redondances du point 4 se retirent chez
leurs propriétaires :
- `neighborhood_nodes.rs` et `usage_nodes.rs` : arbre principal ;
- la réaction : la mémoire.

On gagnerait 3 allers-retours sur impact, 2 sur callees, et C − 1 drains par
réaction.

Pour mesurer, avant et après, en allers-retours et en temps, sur le même
scénario :
- **rag3db** : la base est dans le processus, un aller-retour y coûte peu ; le
  temps doit donc être mesuré, pas supposé ;
- **PostgreSQL** : chaque aller-retour traverse le réseau ; c'est là que le
  gain se verra le plus.
