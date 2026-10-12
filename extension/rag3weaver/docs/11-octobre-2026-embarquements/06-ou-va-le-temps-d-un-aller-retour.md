# Où va le temps d'un aller-retour sur rag3db

*11 octobre 2026, session embarquements. La mesure avant tout code, comme pour les
éventails (page 04).*

## La question

La mesure d'impact (page 04) l'a montré : sur rag3db, impact fait 17 à 24 allers-retours
pour 34 à 52 ms, soit environ 2,2 ms par aller-retour, alors que la base vit dans le
processus. Sur PostgreSQL, par le réseau local, impact prend 5 à 11 ms. Le coût est donc
dans le moteur, à chaque appel, et pas dans le transport. Mais dans quelle phase ?

## La mesure

`rag3db_connection::phases` relève, pour chaque instruction à paramètres, le temps de
chaque phase :
- `prepare` : l'analyse, la liaison et le plan ;
- la conversion des paramètres ;
- `execute` ;
- la conversion des lignes vers nos valeurs ;
- et ce que le moteur dit de lui-même (compilation, exécution).

Le relevé est coupé par défaut : une lecture atomique par appel. Le témoin est
`e2e_mesure_impact::ou_va_le_temps_d_un_aller_retour_sur_rag3db` : impact sur
`src/daemon`, trois noms, profondeur 3, cinquante tours après un tour de chauffe, sous
`poste mesure`, avec un seul fil de test (les compteurs sont globaux).

Avec le défaut du moteur (32 fils d'exécution sur ce poste) : 3 200 instructions,
95 % du temps d'impact dans les instructions.

| phase | par instruction | part |
|---|---|---|
| `prepare` (analyse, liaison, plan) | 0,59 à 0,62 ms | 27 à 29 % |
| conversion des paramètres | 0,01 ms | 0,6 % |
| `execute` | 1,48 à 1,55 ms | 70 à 72 % |
| conversion des lignes | 0,02 ms | 0,8 % |

Le moteur dit à peu près la même chose : 0,48 à 0,50 ms de compilation et 1,17 à 1,28 ms
d'exécution. **C'est l'exécution qui domine, pas l'analyse ni le plan.** Nos
conversions ne comptent pas.

## Ce qui domine l'exécution : les fils

Une instruction d'impact lit quelques lignes. Le moteur lui donne pourtant autant de
fils d'exécution que de cœurs. Mesuré en fixant les fils de la connexion
(`RAG3WEAVER_EXEC_THREADS`, un réglage de mesure lu seulement s'il est posé), même
scénario :

| fils d'exécution | impact, 150 appels | `prepare` | `execute` | le moteur : compilation / exécution |
|---|---|---|---|---|
| 1 | **3 855 ms** | 0,40 ms | 0,73 ms | 0,33 / 0,50 ms |
| 4 | 4 758 ms | 0,45 ms | 0,93 ms | 0,37 / 0,69 ms |
| 32 (défaut) | 7 336 ms | 0,59 ms | 1,55 ms | 0,48 / 1,28 ms |

**Un fil par instruction divise presque par deux le temps d'impact sur rag3db** (−47 %).
La préparation baisse aussi, sans doute moins de contention. À un fil, la préparation
devient environ 35 % de ce qui reste.

## Ce que ça dit, et ce qui reste à décider

1. **Le premier gain est le nombre de fils, pas l'instruction préparée.** Une lecture
   d'outil (impact, usages, liens, recherche) touche peu de lignes, et la mise en
   route de 32 fils coûte plus que le travail lui-même.
2. **Mais un seul fil peut ralentir les gros travaux** : l'ingestion, les COPY, la
   construction des index vectoriels, un balayage. Rien n'est mesuré de ce côté. Les
   formes possibles, à mesurer avant de choisir :
   - **une connexion de lecture pour les outils**, à peu de fils, à côté de celle de
     l'ingestion (le moteur sert plusieurs connexions sur une même base ; la connexion
     de synchronisation existe déjà) ;
   - **un nombre de fils par instruction**, posé par l'appelant : les lectures d'outil à
     1, le reste au défaut. Le moteur le règle par connexion, pas par instruction : il
     faudrait le poser et le rendre autour de chaque appel.
3. **L'instruction préparée** viendrait ensuite : elle gagnerait au mieux la
   préparation, environ 0,4 ms sur 1,1 ms par instruction à un fil. Le moteur sait
   préparer ; une instruction du dialecte dont seuls les paramètres changent se
   prépare une fois par connexion et se rejoue. Ce serait une capacité déclarée du
   dialecte, avec impact comme témoin (mêmes empreintes de lignes, temps avant/après).

**Rien n'est changé dans le produit.** Le relevé des phases et `RAG3WEAVER_EXEC_THREADS`
sont des outils de mesure, inertes s'ils ne sont pas posés. La suite proposée :
mesurer l'ingestion (`e2e_code`, le chemin de masse) à 1, 4 et 32 fils, puis choisir
entre la connexion de lecture et les fils par appel. Ensuite seulement, l'instruction
préparée.

## L'ingestion selon les fils

Mesuré ensuite (`e2e_mesure_impact::l_ingestion_selon_les_fils_d_execution`, `src`
entier, 176 fichiers, en mémoire, lib `ae3d781aa49f`, deux tours par valeur) :
l'ingestion prend 83,3 à 84,4 s à 1 fil, 82,8 à 82,9 s à 4, 82,8 s à 32 — **un fil seul
ne lui coûte pas plus de 2 %** ; impact, sur ce corpus plus gros, gagne 20 % à 1 fil
(3,44 à 3,49 s contre 4,30 à 4,35 s). Réserve : un COPY de masse sur disque, un très gros
corpus et la construction d'un index HNSW ne sont pas mesurés. Le cœur C++ prend la
forme moteur (les fils proportionnés à la cardinalité estimée), ticket
`2026-10-11-un-plan-de-quelques-lignes-prend-un-fil-par-coeur`.

## `usages` sur l'index entier : un balayage, pas des allers-retours

Lucie : « les usages c'est juste des relations, ça ne devrait pas prendre 6 secondes ».
Mesuré sur une copie de l'index de l'agent de code (7 310 fichiers,
`e2e_mesure_index_entier`, `poste mesure`) : un appel d'`usages` fait **six
instructions** et prend 360 à 370 ms en médiane (de 400 ms à 1,5 s le matin, sur une
lib plus ancienne). Chaque instruction est un saut simple sur 1 à 46 uuids, qui rend au
plus 142 lignes, et prend pourtant 24 à 470 ms, même celle qui ne rend rien.

Le plan du saut le plus coûteux (le rendez-vous `MENTIONS` de `update`, 117 lignes) le
dit : le symbole par sa clé (1 ligne), l'extension (117 arêtes), puis **un balayage de
toute la table `Scope` sous semi-masque, 71 657 lignes lues pour 117 retenues**, joint
par hachage. Le masque est petit et éparpillé : presque chaque bloc en contient une
ligne. Ticket moteur au cœur C++ :
`2026-10-11-un-saut-par-semi-masque-lit-toute-la-table`.

Les formes du même saut, mesurées sur la copie (médiane de 7) :

| forme | temps | lignes |
|---|---|---|
| le saut tel quel (6 colonnes du voisin) | 118 à 122 ms | 117 |
| le saut sans propriété du voisin (identifiant interne) | 12,6 à 12,9 ms | 117 |
| le saut qui ne rend que `m._uuid` | 34,5 ms | 117 |
| les champs par `_uuid` (clé primaire), seuls | 22,1 ms | 117 (balaie aussi : 81 407 lignes) |
| **en deux temps** : `m._uuid` seul, puis les champs par clé | **55,9 ms** | **les mêmes que le saut tel quel** |

Une instruction préparée ne servirait à rien ici : la préparation pèse 5 ms sur
360 ms par appel.

**Ce que ça dit.** La forme en deux temps divise le saut par deux, avec les mêmes
lignes ; sur `usages` entier, on irait vers 170 ms, pas vers quelques dizaines. Le reste
est le balayage, que seul le moteur supprime (une recherche par offset quand le masque
est petit). Deux voies :
- **attendre le correctif moteur**, qui rend le saut en un temps à toutes les lectures
  sans rien changer au produit ;
- **ou, d'ici là, le saut en deux temps pour les seuls sauts qui rendent beaucoup de
  champs** (le rendez-vous d'`usages`), comme une chaîne `Hop` → `Select` de `run_plan`,
  avec le témoin des mêmes lignes et du temps.

La décision est à l'orchestration ; rien n'est changé dans le produit.

## Après le lot des fils, et le saut en deux temps (12 octobre)

**Le lot des fils du cœur** (dda63783a) divise impact par deux sur `src/daemon`, mais
n'atteint pas `usages` sur l'index entier : remesuré sur la lib 116acb252ace
(7b54c5f84), `usages` reste à 382 à 389 ms en médiane — ses sauts balaient toute la
table `Scope`, estimée grande, et gardent leurs fils. Au-dessus de 100 ms : la règle de
l'orchestration pose la forme en deux temps.

**La forme** : `dataflow::graph_walk::run_hop`, un saut dit par le dialecte en une
instruction ou en deux. En deux temps, le saut ne rend que l'uuid du voisin (ses champs
vides à leur place), puis ces champs se lisent par clé (`Select::by_uuids`) et se
remettent à leur place, ligne par ligne. Seulement pour un saut vers une table connue
qui lit des champs du voisin, sans filtre, ordre ni limite. Capacité déclarée,
`DialectCapabilities::two_step_hops` : **vraie pour rag3db, fausse pour PostgreSQL**
(sa jointure passe par l'index) ; `Catalog::set_two_step_hops` la lève ou la baisse
pour un montage. Les cinq sauts d'`usages_of` et de `declarations_of` passent par elle.

**Le témoin** (`e2e_mesure_index_entier::usages_en_un_temps_et_en_deux`, même copie,
même catalogue, dix appels chacun, `poste mesure`) : le rendu est **le même** (empreinte
des lignes triées), et la médiane baisse de 35 à 40 % :

| appel | un temps | deux temps |
|---|---|---|
| `update` sous `src/storage/table/node_table` | 359,9 ms | 234,9 ms |
| `update` sans chemin | 363,3 ms | 240,5 ms |
| `getTableID` | 362,7 ms | 217,6 ms |

Le reste (plus de 200 ms) est le balayage sous masque lui-même, que la lecture par clé
fait encore : le lot suivant du cœur (page `coeur-cpp/12` §7). Quand il est sur master,
remesurer : si le saut en un temps rejoint les deux temps, la capacité repasse à faux
pour rag3db et la forme se retire.

**Fusionné** en `198aeeaec` (le saut), puis `6f5554868` : relu par l'arbre principal,
qui a demandé deux choses. Une ligne dont le voisin a disparu entre le saut et la
lecture par clé est écartée et comptée (aucun saut n'est optionnel, donc ce compte ne
mêle pas un « pas de voisin » à un « voisin disparu ») ; le compte part en
`CatalogEvent::Warning`. Les noms de colonnes sont ceux du saut d'origine.

**Remesuré sur la lib du correctif de lecture** (`StringColumn::scanFiltered`, master
`e6f626df4`, sha256 `1b600a4b0b29`, chargée par `LD_LIBRARY_PATH`, `ldd` vérifié), sur
un reflink neuf de la copie scellée :

| appel | un temps | deux temps |
|---|---|---|
| `update` sous `src/storage/table/node_table` | 355,9 ms | 234,7 ms |
| `update` sans chemin | 354,3 ms | 238,9 ms |
| `getTableID` | 353,8 ms | 211,9 ms |

Mêmes empreintes qu'avant. Le correctif de lecture ne change pas le temps : c'est le
balayage sous masque qui coûte. Le lot du cœur qui le fait passer par décalages (117
lignes lues au lieu de 67 584 sur le témoin du banc) est en passe : la remesure suivante
se fait sur lui.

