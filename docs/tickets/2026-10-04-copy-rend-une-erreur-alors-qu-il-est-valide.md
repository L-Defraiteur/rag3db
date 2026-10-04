# Un `COPY` rend une erreur alors qu'il est validé, quand une autre transaction est ouverte

- **État** : corrigé le 4 octobre 2026, commit « fix(transactions): un point de reprise n'attend plus en tenant le verrou public, et une écriture validée ne rend plus d'erreur »
- **Gravité** : perte (un COPY vu par d'autres disparaît après une mort) et réponse fausse
- **Atteignable en service** : oui (dès qu'une autre connexion tient une transaction ouverte pendant un `COPY`)
- **Touche rag3weaver** : à vérifier (il valide chaque instruction seule ; un lecteur long sur une autre connexion suffirait)
- **Ouvert le** : 4 octobre 2026, session cœur C++ (lecture du code, non reproduit)
- **Pour** : cœur C++ (transactions)

## Ce que c'est

Un `COPY FROM` force un point de reprise à sa validation. Si une autre transaction est ouverte, ce point de reprise attend puis expire : l'instruction rend une erreur, alors que le `COPY` est déjà validé. L'appelant croit à un échec et rejoue.

## Recette minimale

```cypher
-- connexion 1
BEGIN TRANSACTION READ ONLY;
MATCH (c:C) RETURN count(*);
-- connexion 2
COPY C FROM 'mille-lignes.csv';   -- après 5 s : « Timeout waiting for active transactions… »
-- connexion 1
COMMIT;
-- n'importe quelle connexion
MATCH (c:C) RETURN count(*);      -- 1000 : le COPY « échoué » est validé et visible
-- mort brutale du processus, réouverture
MATCH (c:C) RETURN count(*);      -- 0 : les lignes vues sont perdues
```

La même erreur remonte d'une écriture ordinaire validée, quand le point de reprise
automatique (au seuil) expire sur un lecteur long ; là, l'écriture est au journal et survit.

## Témoin

`test/transaction/concurrence/single_writer_crash_test.cpp` (banc, 4 octobre), rouges :
- `AutoCheckpointTimeoutOnACommittedWrite` (`error-means-not-committed`) ;
- `CopyWhileATransactionIsOpen` (`error-means-not-committed`) ;
- `CopyThatReturnedAnErrorThenDeath` (`visible-copy-survives-death`) : 1 000 lignes vues
  avant la mort, 0 après.

Le test de l'amont `test/test_files/transaction/basic.test` (lignes 78-80) attend l'erreur et
ne regarde que la transaction déjà ouverte.

## Cause

`TransactionManager::commit` valide, écrit le journal, puis lance le point de reprise forcé (`src/transaction/transaction_manager.cpp`) ; le délai d'attente des autres transactions lève une erreur qui remonte comme le résultat de l'instruction. Un délai dépassé ne marque pas la base « point de reprise échoué ».

## Correctif de l'amont

Aucun connu. Ladybug `4ca1d6f5d` ne force plus le point de reprise quand `auto_checkpoint` est faux, ce qui évite le cas sans le corriger.

## Ce qu'il faut pour le fermer

Décider ce que rend un `COPY` validé dont le point de reprise n'a pas pu se faire — un succès avec avertissement nommé, ou une erreur nommée qui dit « validé, non durable » — puis le témoin.

## Côté rag3weaver (arbre principal, 4 octobre)

Aujourd'hui, une seule connexion fait tout pendant un index (catalogue et magasin de blobs : aucun hôte n'appelle `set_sync_connection`), chaque requête rend ses lignes déjà lues, et le catalogue est tenu sous verrou pendant `sync_source` : rag3weaver ne garde aucune transaction ouverte sur une autre connexion, et ne déclenche pas ce délai lui-même. Le cas devient réel le jour où un hôte fournit une connexion séparée au magasin de blobs, ou quand les écritures parallèles arrivent. Prévu après le correctif : l'erreur reconnue par son nom à un seul endroit ; hors transaction, le COPY rejoué trois fois au plus (`copy_retried` au rapport) ; dans la transaction par paquet, empoisonnement puis reprise, la cause nommée.
