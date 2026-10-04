# Un `COPY` rend une erreur alors qu'il est validé, quand une autre transaction est ouverte

- **État** : ouvert
- **Gravité** : réponse fausse
- **Atteignable en service** : oui (dès qu'une autre connexion tient une transaction ouverte pendant un `COPY`)
- **Touche rag3weaver** : à vérifier (il valide chaque instruction seule ; un lecteur long sur une autre connexion suffirait)
- **Ouvert le** : 4 octobre 2026, session cœur C++ (lecture du code, non reproduit)
- **Pour** : cœur C++ (transactions)

## Ce que c'est

Un `COPY FROM` force un point de reprise à sa validation. Si une autre transaction est ouverte, ce point de reprise attend puis expire : l'instruction rend une erreur, alors que le `COPY` est déjà validé. L'appelant croit à un échec et rejoue.

## Recette minimale

Non écrite. D'après le code : une connexion ouvre `BEGIN TRANSACTION` et lit ; une seconde lance un `COPY` ; attendre le délai du point de reprise.

## Témoin

Aucun chez nous. Un test de l'amont décrit le comportement : `test/test_files/transaction/basic.test` (lignes 78-80).

## Cause

`TransactionManager::commit` valide, écrit le journal, puis lance le point de reprise forcé (`src/transaction/transaction_manager.cpp`) ; le délai d'attente des autres transactions lève une erreur qui remonte comme le résultat de l'instruction. Un délai dépassé ne marque pas la base « point de reprise échoué ».

## Correctif de l'amont

Aucun connu. Ladybug `4ca1d6f5d` ne force plus le point de reprise quand `auto_checkpoint` est faux, ce qui évite le cas sans le corriger.

## Ce qu'il faut pour le fermer

Décider ce que rend un `COPY` validé dont le point de reprise n'a pas pu se faire — un succès avec avertissement nommé, ou une erreur nommée qui dit « validé, non durable » — puis le témoin.
