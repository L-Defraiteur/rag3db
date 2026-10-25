# Une instruction qui échoue condamne une transaction en lecture seule

- **État** : ouvert, le 25 octobre 2026 (relevé par la session « recherche », pour le cœur C++).
- **Gravité** : blocage d'une lecture entière pour la faute d'une seule de ses instructions.
- **Atteignable en service** : oui. Chaque appel d'outil de lecture lit dans une transaction
  `READ ONLY` qu'il partage entre toutes ses requêtes (L6, `extension/rag3weaver/src/read_pool.rs`).
- **Touche rag3weaver** : oui (toute lecture d'outil sous instantané).

## Ce que c'est

Depuis la marche T0 (`246c334a1`), une instruction qui échoue dans une transaction manuelle la
condamne, quelle que soit la cause : `TransactionContext::rollbackAfterStatementFailure`
(`src/transaction/transaction_context.cpp`) pose `manualTransactionFailed`, et toute instruction
suivante est refusée par `throwIfManualTransactionFailed` (« The transaction was rolled back
because a statement in it failed: nothing it wrote remains, and nothing can run in it. Send
ROLLBACK to close it. »). L'échec d'analyse ou de liaison y passe aussi (`prepareWithParams`,
`queryNoLock`, la liaison des paramètres, dans `src/main/client_context.cpp`).

C'est juste pour une transaction qui écrit : ses effets sont annulés, la continuer serait perdre
son atomicité sans le savoir. PostgreSQL fait de même (« current transaction is aborted »).

Mais une transaction en **lecture seule** n'a rien écrit. La condamner ne protège rien, et elle
fait tomber des lectures saines. Le cas vu : `search_code` lance ses signaux (plein texte, dense,
creux) sur une même transaction `READ ONLY`. Le `QUERY_VECTOR_INDEX` d'un index absent échoue
au binder, et le plein texte, qui n'avait rien à se reprocher, échoue à son tour sur le refus.
Le ticket produit `2026-10-25-un-signal-en-panne-fait-tomber-la-recherche-hybride-entiere`
contourne le cas connu (la branche dense demande d'abord à `SHOW_INDEXES` si son index existe).
Une autre faute d'une branche, inconnue aujourd'hui, ferait tomber les autres de la même façon.

## La recette

```
BEGIN TRANSACTION READ ONLY;
CALL QUERY_VECTOR_INDEX('T', 'un_index_absent', [0.1, 0.2], 3) RETURN *;  -- Binder exception
MATCH (n:T) RETURN count(n);                                               -- refusée : MANUAL_TRANSACTION_FAILED
ROLLBACK;
```

## Le témoin

À écrire côté moteur (gtest du dossier `test/transaction`) : la recette ci-dessus, la seconde
instruction doit répondre. Un second témoin garde la règle pour une transaction qui écrit :
`BEGIN TRANSACTION`, une écriture, une instruction fautive, puis une écriture refusée.

## Les formes, à trancher par le cœur dans le code

1. **La courte, à lire en premier** : une instruction qui échoue **avant toute exécution**
   (analyse, liaison, plan) dans une transaction **en lecture seule** ne la condamne pas. Rien n'a
   été écrit, rien n'est à défaire. Le bloc reste utilisable, l'erreur est rendue à l'instruction
   seule. À vérifier : qu'aucun état de la transaction (verrous, instantané, compteurs) ne reste
   faux après un échec au binder.
2. **La générale, à la PostgreSQL** : un point de sauvegarde implicite par instruction (comme
   `SAVEPOINT` / `ROLLBACK TO SAVEPOINT`, que les pilotes PostgreSQL posent autour de chaque
   instruction en mode « autosave »). Une instruction en échec ne défait qu'elle-même, la
   transaction continue, en lecture comme en écriture. Plus large, et plus cher : il faut savoir
   défaire une seule instruction d'une transaction qui écrit.

La forme 1 suffit au besoin connu (les lectures d'outil sous L6). La forme 2 rendrait aussi les
transactions d'écriture tolérantes à une instruction fautive, ce que personne ne demande
aujourd'hui.

## Pour le fermer

Le témoin de la recette vert, et celui de l'écriture toujours condamnée vert aussi.
