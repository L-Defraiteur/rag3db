# Les modèles de décision, mesurés sur nos sujets

**3 octobre 2026, session « optimiseur ».** Temps 2, après
[la vérification à la source](01-les-modeles-de-decision-verifies-a-la-source.md).
Trois modèles et deux références, mesurés sur un petit jeu à nous, contre les
cinq conditions que la proposition de la mémoire longue pose au nœud de
décision ([§7.5](../../3-octobre-2026-20h41/01-une-memoire-longue-proposition.md)).

**Le résultat : JevK5 est le seul des trois à rendre un verdict utilisable, y
compris en français. Les deux petits encodeurs ne le font pas. Et nos
références ordonnent bien mais ne jugent pas : le cosinus note une
contradiction aussi haut qu'une redite.**

## Ce qui a été mesuré, et comment

| | |
|---|---|
| les modèles | `laya-multilingual` (322 M), `GLiNER2.5-multi-Decide` (287 M), JevK5-4B v0.3 quantifié en Q4_K_M (2,7 Go) |
| les références | cosinus de granite-278m ; score de `bge-reranker-v2-m3` (moyenne des deux sens) |
| le jeu | 42 paires de sujets (14 en français, 14 en anglais, 14 mixtes ; 15 « même », 15 « variante », 12 « sans rapport ») ; 20 paires de mémoires (même, complète, contredit, neuve) ; 6 décisions à dix candidats |
| le critère | écrit en anglais, donné à l'appel, avec une phrase par option |
| la machine | CPU seulement, huit fils ; la carte n'a pas servi. JevK5 par `llama-server`, les autres par leur bibliothèque Python |
| pour rejouer | `banc/` à côté de ce document : le jeu, les scripts, les sorties brutes |

Les réserves, à lire avant les chiffres :

- **Le jeu est petit et d'une seule main.** Les étiquettes sont les miennes,
  non relues. À 42 paires, une paire vaut 2,4 points ; à 14 par langue, 7.
- **« Variante » est une étiquette discutable** : la frontière avec « sans
  rapport » est la mienne. C'est là que tous les modèles se trompent.
- **Les seuils sont réglés sur les paires mêmes qu'ils jugent** : les
  exactitudes « au meilleur seuil » sont optimistes.
- **GLiNER est peut-être mal servi par notre usage** : il attend des
  étiquettes courtes, pas un critère ; je lui ai donné les options et leurs
  descriptions, comme aux autres.
- **Latences sur CPU** : elles disent ce que coûte une décision sans carte,
  pas ce qu'elle coûterait sur la carte.

## 1. Le verdict

Exactitude du choix le plus probable.

| modèle | sujets (hasard 33 %) | français | anglais | mixtes | mémoires (hasard 25 %) | français | anglais | mixtes |
|---|---|---|---|---|---|---|---|---|
| JevK5-4B | **64 %** | 57 % | 71 % | 64 % | **85 %** | 88 % | 86 % | 80 % |
| Laya multilingue | 45 % | 36 % | 43 % | 57 % | 60 % | 62 % | 43 % | 80 % |
| GLiNER2.5-multi-Decide | 31 % | 29 % | 29 % | 36 % | 15 % | 0 % | 14 % | 40 % |

- **JevK5** se trompe presque toujours au même endroit : dix « variantes »
  sur quinze jugées « sans rapport ». Sur les mémoires, 17 sur 20, et aucune
  contradiction prise pour une redite. Il se dit anglais seulement ; **le
  français passe** (57 % et 88 %, contre 71 % et 86 % en anglais).
- **Laya** répond « sans rapport » à tort (cinq vraies « même chose » et douze
  variantes).
- **GLiNER** répond « même chose » à presque tout (40 fois sur 42) : sur ce
  jeu, il ne décide pas.

## 2. Le score à critère fixe

La condition dure de la proposition : un seuil unique, pour un critère donné,
sépare-t-il « même chose » du reste ? Le score est la probabilité de « same »
pour les modèles, le score brut pour les références. AUC : 1,00 si toutes les
vraies passent devant toutes les autres, 0,50 au hasard.

**Sujets**

| modèle | AUC | français | anglais | mixtes | vraies « même » : min / médiane / max | les autres : min / médiane / max | vraies retrouvées sans une seule fausse fusion |
|---|---|---|---|---|---|---|---|
| JevK5-4B | **0,98** | 0,87 | 1,00 | 1,00 | 0,37 / 0,71 / 0,89 | 0,06 / 0,23 / 0,50 | **80 %** (seuil 0,50) |
| reranker bge | 0,91 | 0,78 | 0,87 | 1,00 | 0,00 / 0,30 / 1,00 | 0,00 / 0,00 / 0,15 | 60 % (seuil 0,15) |
| cosinus granite | 0,89 | 0,82 | 0,73 | 1,00 | 0,60 / 0,78 / 0,93 | 0,48 / 0,56 / 0,77 | 53 % (seuil 0,77) |
| Laya multilingue | 0,77 | 0,62 | 0,80 | 0,96 | 0,03 / 0,51 / 0,72 | 0,00 / 0,19 / 0,60 | 33 % (seuil 0,60) |
| GLiNER | 0,60 | 0,69 | 0,80 | 0,27 | 0,28 / 0,59 / 0,78 | 0,45 / 0,57 / 0,65 | 27 % (seuil 0,65) |

**Mémoires**

| modèle | AUC | vraies « même » : min / médiane / max | les autres : min / médiane / max | vraies retrouvées sans une seule fausse fusion |
|---|---|---|---|---|
| JevK5-4B | **0,97** | 0,04 / 0,44 / 0,78 | 0,00 / 0,01 / 0,07 | **80 %** (seuil 0,07) |
| reranker bge | 0,83 | 0,01 / 0,87 / 1,00 | 0,00 / 0,01 / 0,96 | 40 % |
| Laya multilingue | 0,77 | 0,00 / 0,19 / 0,79 | 0,01 / 0,06 / 0,18 | 60 % |
| cosinus granite | 0,71 | 0,66 / 0,81 / 0,95 | 0,50 / 0,75 / 0,87 | 40 % |
| GLiNER | 0,61 | 0,24 / 0,27 / 0,29 | 0,07 / 0,25 / 0,47 | 0 % |

Ce que ces tableaux disent :

- **JevK5 sépare** : avec un seuil qui ne laisse passer aucune fausse fusion,
  il retrouve quatre vraies « même chose » sur cinq, sur les deux jeux. Mais
  **le seuil n'est pas le même d'un critère à l'autre** (0,50 pour les
  sujets, 0,07 pour les mémoires) : les seuils par critère de la proposition
  sont nécessaires, pas seulement prudents.
- **Les paires mixtes sont le cas facile** pour tout ce qui est multilingue :
  un sujet en français et son double en anglais sont séparés sans faute par
  JevK5, le cosinus et le reranker.
- **Le reranker a une vraie faille** : cinq vraies « même chose » notées sous
  0,03, toutes des reformulations longues (« poids des modèles publiés sur
  Hugging Face » et « mise en ligne des fichiers de poids sur le hub »).
- **Le cosinus n'a pas de marge** : les vraies commencent à 0,60, les autres
  montent à 0,77.

**Une référence ne sait pas dire « contredit ».** Score moyen par étiquette,
sur les mémoires :

| référence | même | complète | contredit | neuve |
|---|---|---|---|---|
| cosinus granite | 0,83 | 0,74 | **0,85** | 0,55 |
| reranker bge | 0,61 | 0,45 | 0,12 | 0,00 |

Pour le cosinus, « le défaut est granite-278m » et « le défaut est désormais
granite-107m » sont la même phrase. C'est exactement ce que dit la
proposition : un ordre n'est pas un verdict. Le reranker, lui, note bas une
contradiction, mais ne la distingue pas d'une mémoire sans rapport.

## 3. La calibration

Une assurance de 0,9 a-t-elle raison neuf fois sur dix ?

| modèle | jeu | assurance moyenne | exactitude | écart de calibration | assurance au-dessus de 0,9 : cas, dont justes |
|---|---|---|---|---|---|
| JevK5-4B | sujets | 0,67 | 64 % | 0,11 | 2, 2 |
| JevK5-4B | mémoires | 0,74 | 85 % | 0,11 | 6, 6 |
| Laya multilingue | sujets | 0,69 | 45 % | 0,24 | **9, 4** |
| Laya multilingue | mémoires | 0,71 | 60 % | 0,22 | 5, 4 |
| GLiNER | sujets | 0,58 | 31 % | 0,27 | 0, 0 |
| GLiNER | mémoires | 0,41 | 15 % | 0,26 | 0, 0 |

JevK5 : huit décisions annoncées au-dessus de 0,9, huit justes ; son
assurance moyenne suit son exactitude. Laya annonce 0,97 sur des réponses
fausses. Huit cas ne font pas une preuve de calibration : c'est un signe.

## 4. L'abstention

Avec une quatrième option « unsure » ajoutée au critère des sujets :

| modèle | « unsure » choisi | dont sur une paire qu'il avait fausse |
|---|---|---|
| JevK5-4B | 0 fois sur 42 | — |
| GLiNER | 0 fois sur 42 | — |
| Laya multilingue | 5 fois sur 42 | 2 |

**Aucun ne sait s'abstenir.** La proposition l'avait prévu : la zone
incertaine du score *est* l'abstention. Pour JevK5 sur les sujets, elle va de
0,37 (la plus basse des vraies) à 0,50 (la plus haute des autres).

## 5. Le déterminisme et la latence

| modèle | écart entre deux passes | une décision, 3 options | 4 options | dix candidats en une décision | bon candidat sur dix | premier appel |
|---|---|---|---|---|---|---|
| cosinus granite | 0 | 13 ms | 16 ms | 133 ms | 5 sur 5 | — |
| Laya multilingue | 0 | 54 ms | 56 ms | 63 ms | 6 sur 6 | 5,1 s |
| GLiNER | 0 | 90 ms | 102 ms | 117 ms | 5 sur 6 | 0,2 s |
| reranker bge | 0 | 104 ms | 120 ms | 1 060 ms | 5 sur 5 | — |
| JevK5-4B | 0 | **1 955 ms** | 2 453 ms | **4 405 ms** | 6 sur 6 | 2,0 s |

- **Tous sont déterministes** : deux passes, mêmes probabilités au dernier
  chiffre.
- **JevK5 coûte deux secondes par décision sur CPU**, quatre et demie pour dix
  candidats. C'est trop pour « le temps d'une écriture » sans carte. Non
  mesuré : la même chose sur la carte, et la taille 2B du même modèle.
- **À dix candidats, tous trouvent le bon** (et JevK5 comme Laya reconnaissent
  le cas « aucun »). Les références n'ont pas de « aucun » sans seuil.

## Contre les cinq conditions

| condition de la proposition | JevK5-4B | Laya multilingue | GLiNER | cosinus, reranker |
|---|---|---|---|---|
| 1. liste fermée, hors de sa bonne volonté | oui : lecture des logits des lettres | oui : une tête par option | oui | sans objet : un score |
| 2. score comparable à critère fixe | **oui**, avec un seuil par critère | non | non | pour « proche ou non » seulement ; le cosinus est aveugle à la contradiction |
| 3. abstention | non ; la zone incertaine en tient lieu | non | non | non |
| 4. déterminisme | oui | oui | oui | oui |
| 5. une passe, plusieurs langues, latence d'une écriture | une passe jusqu'à 16 options ; le français passe ; **2 s sur CPU** | oui, 54 ms | oui, 90 ms | oui |

## Ce que j'en conclus

1. **Les deux étages de la proposition sont les bons, et ils existent.** Le
   cosinus ou le reranker ordonnent dix candidats et trouvent le bon à chaque
   fois ; ils ne peuvent pas dire « même », « complète » ou « contredit ».
2. **Pour l'étage du verdict, JevK5-4B est le seul candidat mesuré qui
   tienne** : 85 % sur les quatre choix de `remember`, un score qui sépare,
   une assurance qui suit l'exactitude, le français qui passe. Rien à
   entraîner.
3. **Son prix est la latence** : il ne passe pas par notre moteur burn mais
   par `llama-server`, et deux secondes sur CPU. La suite utile est de le
   mesurer sur la carte, en régime doux, et d'essayer la taille 2B.
4. **Les petits encodeurs ne sont pas prêts tels quels** sur nos critères.
   Laya le dit lui-même (« a fast base to specialise ») ; l'affiner serait
   l'entraîner nous-mêmes, ce qui est exclu.
5. **« Variante » n'est pas une sortie fiable**, pour aucun modèle et
   peut-être pas pour nous non plus. Le verdict à quatre choix de `remember`
   est bien mieux tenu que le verdict à trois choix des sujets : c'est un
   argument pour poser au nœud des questions dont les options se définissent
   nettement.
6. **Le défaut sûr de la proposition reste juste** : sous le seuil haut, on
   crée et la question remonte. Avec JevK5, ce seuil laisse passer quatre
   vraies fusions sur cinq sans aucune fausse, sur ce petit jeu.

## Ce qui n'est pas mesuré

- Un jeu plus grand, relu par quelqu'un d'autre, et le critère écrit en
  français.
- JevK5 sur la carte, sa taille 2B, sa quantification Q8.
- Kev, écarté faute de voie CPU simple, et les autres modèles de la même
  famille (OpenDecider small, `Mapika/decider`).
- GLiNER dans son usage natif (étiquettes courtes, sans critère).
- L'accord avec un jugement humain, que la proposition demande : il faut le
  banc scripté de la mémoire longue pour cela.
