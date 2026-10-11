# Les éventails : plusieurs sorties en une instruction

*11 octobre 2026, session embarquements. Écarts seuls, écrits avant le code, pour
que Lucie puisse renverser. La décision de l'orchestration : l'IR ne change pas.
C'est une capacité de plus du dialecte, sur un sous-graphe qu'on lui confie
entier.*

## Pourquoi

Le relevé (page 03) le montre : sans les éventails, `run_plan` ne fond presque
rien des vrais chemins. Le coût d'impact vient de R sauts, ou de R degrés, qui
partent de la même entrée. Ce sont des sœurs, et le Rust les réunit ensuite.

## La forme

- **Un éventail**, c'est au moins deux nœuds sœurs. Leur entrée a la même source :
  - soit la même colonne d'un même nœud amont, qui n'est pas une sortie et n'a
    pas d'autres suivants ;
  - soit la même liste d'uuids donnée par l'appelant.
- Les sœurs ont **la même forme de sortie**. Ce sont des sauts aux mêmes
  colonnes, des degrés, ou des sélections aux mêmes colonnes. Les deux moteurs
  exigent des colonnes de même type dans une union.
- Elles partent en **une instruction**, avec une colonne de tête qui dit la
  branche (`__branch`) :
  - en Cypher, `UNION ALL` de branches qui rendent `__branch, c0, c1, …` ;
  - en SQL, `UNION ALL` de sous-requêtes sur la même CTE d'entrée.
- L'exécuteur range les lignes **par sortie**, d'après `__branch`.
- La règle du repli ne change pas : chaque branche reçoit les valeurs distinctes
  et non nulles de sa colonne d'entrée.

## Ce qui diffère d'un planificateur SQL

1. **Une instruction rend plusieurs sorties.** Une requête SQL rend un seul
   résultat. Ici, une instruction en rend plusieurs, séparées par une colonne de
   branche que le dialecte ajoute et que l'exécuteur retire. C'est l'écart n° 2
   de la page 01, levé.
2. **Le rapport le dit.** `fused` contient un groupe à plusieurs sorties, par
   exemple `[["sel", "deg_in", "deg_out"]]` pour une seule instruction.
3. **En Cypher, l'amont est rejoué dans chaque branche.** Une `UNION` de Cypher
   ne partage pas de préfixe. rag3db lit donc la sélection d'amont une fois par
   branche, dans la même instruction. Ça fait un aller-retour, mais pas un seul
   balayage. PostgreSQL la partage par sa CTE.
4. **Les sœurs de formes différentes ne se fondent pas.** Un saut et un degré
   rendent des colonnes de types différents, et l'union les refuserait. Ils
   restent découpés, et `fused` le dit.

## Le témoin

Chaque scénario d'éventail joue fondu puis découpé, avec les mêmes lignes par
sortie, sur rag3db et sur le conteneur PostgreSQL :
- deux degrés sur la même liste d'uuids de l'appelant ;
- deux degrés nourris par une même sélection.

Ensuite, impact se branche sur `run_plan`. Il sera mesuré avant et après, en
allers-retours **et** en temps, sur les deux moteurs, avec l'arbre principal en
relecture de `code_tools`.

## Mesuré, et choisi (11 octobre 2026)

**Impact branché sur `run_plan`** (`e2e_mesure_impact`, corpus `src/daemon`, `poste
mesure`, même base avant et après). Les allers-retours tombent de 35 à 42 % :
20→12, 23→13, 17→11, 17→11, 21→13, 24→14. Les lignes rendues sont les mêmes (même
empreinte des lignes triées, dans les six cas). Le temps sur rag3db ne baisse pas :
39→43, 46→49, 32→33, 33→33, 39→44, 44→48 ms.

**Les éventails seuls**, sur un graphe peuplé par le catalogue (300 nœuds, trois
relations, les mêmes 300 uuids), découpés puis fondus, en médiane de 21 :

| moteur | éventail | découpé | fondu |
|---|---|---|---|
| rag3db | six degrés | 6 instructions, 20,05 ms | 1 instruction, 20,18 ms |
| rag3db | trois sauts | 3 instructions, 8,34 ms | 1 instruction, 9,97 ms |
| PostgreSQL (conteneur local) | six degrés | 6 instructions, 2,70 ms | 1 instruction, 2,81 ms |
| PostgreSQL (conteneur local) | trois sauts | 3 instructions, 2,12 ms | 1 instruction, 2,35 ms |

Les mêmes lignes sortent partout (1 800 et 2 700). **En local, la fusion ne gagne de
temps sur aucun des deux moteurs, et coûte 1 à 20 %.** rag3db vit dans le processus ;
le conteneur PostgreSQL est sur la boucle locale, où un aller-retour coûte à peine
0,1 ms. Le gain n'existe qu'avec un aller-retour cher.

*Projection, pas une mesure* : à 1 ms d'aller-retour, six instructions en une
économisent environ 5 ms, pour 0,1 à 0,2 ms de travail en plus.

**Le choix** (orchestration, 11 octobre) : une capacité déclarée,
`DialectCapabilities::fanout`, **fausse pour rag3db et fausse pour PostgreSQL par
défaut**. Un montage la lève par `Catalog::set_fanout(Some(true))` (un PostgreSQL
distant), ou elle se lève après une mesure sur un vrai déploiement ; jamais sur la foi
d'un calcul. Les deux traductions restent et sont éprouvées directement : textes
exacts, et e2e fondu puis découpé avec la fusion levée (`PlanOptions::fanout`). Impact
reste sur `run_plan`, et sans éventail levé il fait les allers-retours d'avant, moins
les deux de callees.
