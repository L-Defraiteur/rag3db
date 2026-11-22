# La recherche de code sur le jeu de questions

22 novembre 2026, après-midi. Le chiffre de référence de la recherche de code de rag3weaver, et le verdict sur les
corrections et les budgets de lucivy. C'est la suite des pages 01 à 03, à la demande de Lucie (oui de 11 h 50 : un jeu
durable de questions avec leur or).

## La base, et deux fautes de la première mesure

1. **La base de mesure de la matinée n'indexait pas le code du jour**, mais l'arbre de Lucie à luciepc, resté au
   18 octobre (`284cc00c0b`) : `workspace.root` du manifeste, `source: working_tree`. Recherche l'a trouvé : 10 des
   26 ratés n'avaient pas leur or dans la base. **C'est mon erreur** : j'avais vérifié les ors sur master sans prouver
   quel arbre la base avait lu.
2. **La fuite** : mes pages et les e2e du banc citent les questions mot pour mot. Dans une base au master du jour,
   une question se trouve elle-même. Sur la base neuve, **41 questions sur 46** avaient des lignes qui fuient dans leurs
   réponses (recherche l'a vu aussi).

**La base neuve** :
- `banc-parite/base-mesure-master-0763672a95` à luciepc ;
- indexée sur un worktree du banc détaché sur master `0763672a95`, jamais l'arbre de Lucie ;
- les 46 ors vérifiés présents avant l'index ;
- son propre service d'embarquement (granite-278m sur 7882, card2, pas le 7878 partagé) ;
- 16 minutes, 2,0 Go.

**Le juge** ôte les lignes qui fuient avant de lire un rang. Les comparaisons de la matinée restaient valables entre
côtés (même base partout). Les niveaux absolus ci-dessous les remplacent.

## Les résultats de référence

Jeu v2, 46 questions ([questions-recherche-v2.md](../jeux/questions-recherche-v2.md)) ; le juge rend le rang de l'or sous
la fusion (top 10), le BM25 seul et le dense seul.

| | or premier | rappel@10 | BM25 seul : premier / rappel@10 | pages entières en tête du BM25 seul | temps médian |
|---|---|---|---|---|---|
| master `0763672a95` | 14 | 22 | 8 / 16 | 1 | 735 ms |
| **(a)+(b)** (`89dd0caec8`) | **15** | **24** | **9 / 18** | 1 | **635 ms** |
| les trois corrections, sans budget (`aa51b13297`) | 12 | 24 | 4 / 8 | 4 | 1 111 ms |
| les trois + `flat` | 9 | 23 | 5 / 5 | 25 | 679 ms |
| les trois + `idf` | 8 | 23 | 5 / 5 | 25 | 731 ms |
| les trois + `window` | 9 | 23 | 5 / 5 | 22 | 827 ms |
| les trois + `soft` | 7 | 23 | 5 / 6 | 16 | 671 ms |

Les corrections de lucivy :
- **(a)** la requête entière exacte (distance 0) au-delà d'un mot, sans `strict_separators` ;
- **(b)** les champs en un seul `contains` à `fields` ;
- **(c)** le chemin des mots en `contains_split`.

**(a)+(b) gagne sans rien faire reculer.** Trois questions montent, aucune ne recule :
- « vectr search nod » : absent → 2 ;
- « SearchOptons » : absent → 1 ;
- « the backend manifest » : 8 → 3.

Sur les 39 questions solides, le rappel@10 passe de 20 à 22.

**(c), le chemin des mots en `contains_split`, coûte.** Il divise par deux le BM25 seul (rappel@10 16 → 8), et donne
6 questions qui montent pour 8 qui reculent. Parmi celles qui reculent :
- « embed the query text… » : 6 → absent ;
- « le mode de la requête plein texte… » : 3 → absent ;
- « remove a deleted file » : 1 → 9 ;
- « fermer les sessions mortes » : 1 → 5.

**Tout budget d'erreurs, par-dessus, recule encore** : or premier de 12 à 7-9, et jusqu'à 25 pages entières en tête du
BM25 seul sur 46.

## Ce que je recommande

1. **Prendre (a)+(b)** (branche `whole-phrase-exact` de recherche, plus les champs en un seul `contains`). Gain net, aucun
   recul, plus rapide.
2. **Ne pas prendre (c)** : le chemin des mots en `contains_split` dégrade le BM25 sur la recherche de code.
3. **Aucun budget d'erreurs par défaut dans rag3weaver** (confirmé au correctif de lucivy, section suivante). Pour lucivy elle-même, le défaut reste à lucivy et à Lucie ; sur
   la recherche de code, le budget coûte quelle que soit la variante.

## Correction du soir : les budgets rejoués au correctif de lucivy

**Le classement des budgets ci-dessus était faussé.** lucivy l'a trouvé : dans 4.5.0, `ErrorBudgetQuery` et `contains`
à `fields` (`BestFieldQuery`) ne voyaient que le premier fragment d'un index à plusieurs fragments, alors que le booléen
sans budget les voyait tous. La comparaison avantageait donc « sans budget ».

Le tout est rejoué au correctif `61c32a6` (lucivy, PR #67), sur une base neuve au master `408c38bae5`. Elle contient les
docs remises à leur item (codeparsers, `3dc00c176c`) et la phrase entière exacte, (a), déjà atterrie. Elle est indexée
par un binaire de ce même master, sans repasse d'analyse. Jeu v2, même juge, aucune erreur HTTP :

| | or premier | rappel@10 | BM25 seul (rappel@10) | temps médian |
|---|---|---|---|---|
| master `408c38bae5` | 15 | 24 | 18 | 734 ms |
| + noms à une faute (`name-within-a-typo-2`) | 20 | 29 | 23 | 607 ms |
| **+ noms + `fields`** (`typo-fields-measure-3`) | **19** | **29** | **23** | **638 ms** |
| + noms + `fields` + (c), sans budget | 15 | 31 | 12 | 1 204 ms |
| + (c) + `flat` | 8 | 29 | 9 | 1 270 ms |
| + (c) + `idf` | 9 | 30 | 10 | 1 342 ms |
| + (c) + `window` | 8 | 29 | 9 | 1 377 ms |
| + (c) + `soft` | 9 | 31 | 11 | 1 319 ms |

- **Aucun budget par défaut : le verdict tient.** Chaque variante perd 6 à 7 premiers sur la même branche sans budget,
  et le BM25 seul met 21 à 24 pages entières en tête. **Nuance** : avec tous les fragments vus, le rappel@10 ne recule
  plus (`soft` garde 31). Le défaut de 4.5.0 exagérait la perte de rappel, pas celle des premiers.
- **(c) est un échange.** Contre noms + `fields`, il donne +2 dans le top 10, mais −4 premiers, un BM25 seul presque
  divisé par deux et un temps doublé.
- **La décision, de c7 (22 nov., au mot de Lucie « tranchez ») :**
  - la pile est **noms + `fields`** ;
  - **aucun budget par défaut**, le budget restant une option qu'une requête peut poser ;
  - **(c) n'est pas prise** : un agent lit d'abord le premier résultat. Elle reste une piste, avec ce tableau.
- `recherche-contains-fields` part sur cette pile quand la 4.5.1 de lucivy sera publiée, au mot de Lucie.

Les chiffres de la section précédente restent valables pour (a)+(b) contre master sur leur base. Ceux des budgets sont
remplacés par ceux-ci.

## Ce qui reste raté, et pourquoi

Avec (a)+(b), **22 questions sur 46** n'ont pas leur or dans le top 10 :

| famille | nombre |
|---|---|
| aucun signal ne met l'or dans ses 20 | 10 |
| les deux signaux le trouvent entre 11 et 20 | 5 |
| **le dense le met dans ses 10, la fusion l'enterre** | **6** |
| le BM25 le met dans ses 10, la fusion l'enterre | 1 |

Les sept dernières désignent **la fusion**. Recherche classe les causes côté index et lexical. Le premier cas qu'elle
a vu (q01) : `FuseResultsNode` est indexé, mais les tests de la fusion du même fichier passent devant, et sa doc est
rattachée à l'enum qui la précède. La table, ligne par ligne, est à luciepc :
`banc-parite/sorties/questions-ref-ab-20261122-1455/rates-ab.md`.

## Les fichiers

Les scripts et les sorties sont à luciepc, sous `~/.cache/rag3weaver-build/` :
- `base-mesure.sh`, la base ;
- `reference-v2.sh` et `lucivy-questions2.sh`, les passes ;
- `juge_questions_v2.py`, le juge ;
- `compare_questions.py` et `table_ratees.py`, la comparaison et la table des ratés.

Les bruts sont sous `banc-parite/sorties/questions-ref-*-20261122-*`.
