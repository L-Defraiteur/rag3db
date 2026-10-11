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
