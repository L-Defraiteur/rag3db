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
