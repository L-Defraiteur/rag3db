# Un COMMIT dont le point de reprise échoue après la validation fait annuler une transaction déjà détruite

- **État** : ouvert, confirmé par le banc le 18 octobre 2026 (Release et ASAN, trois sur trois)
- **Gravité** : lecture après libération (`Transaction` détruite par le gestionnaire, encore
  tenue par le contexte de la connexion) : assertion en debug, comportement indéfini en release ;
  rien n'est perdu (la validation est au journal), le danger est le plantage ou un verrou rendu
  pour un mauvais identifiant.
- **Atteignable en service** : oui, mais seulement quand un point de reprise échoue (disque
  plein, erreur d'écriture) juste après une validation qui l'a provoqué ; ensuite tout est déjà
  refusé par `REOPEN_AFTER_FAILED_CHECKPOINT`.
- **Touche rag3weaver** : oui, par le même chemin (une validation dont le point de reprise
  automatique échoue).
- **Ouvert le** : 18 octobre 2026, session cœur C++.
- **Pour** : cœur C++.

## Ce que c'est

`TransactionContext::commit` appelle `TransactionManager::commit` puis `clearTransaction()`.
Le gestionnaire retire la transaction de `activeTransactions` (`clearTransactionNoLock`, qui
détruit l'objet `Transaction` : la liste en est propriétaire) **avant** d'écrire le point de
reprise qui suit la validation. Si ce point de reprise échoue, trois chemins lèvent une
`TransactionManagerException` après ce retrait :

- la validation dans la file d'un point de reprise en attente, quand ce point de reprise échoue
  (`commitInCheckpointQueueNoLock`, `throwIfCheckpointFailedNoLock` après l'attente, et le
  `catch (CheckpointException)` qui suit, `transaction_manager.cpp:359-378`) ;
- la demande automatique écrite par la dernière transaction à partir
  (`writeRequestedCheckpointIfLastNoLock`, `catch (CheckpointException)` converti,
  `transaction_manager.cpp:410-422`).

Le contexte n'a pas vidé `activeTransaction` (l'exception est sortie avant `clearTransaction`).
`runFuncInTransaction` (`client_context.cpp:603-640`) ne reconnaît que `CheckpointException` ;
une `TransactionManagerException` prend le chemin ordinaire : `context.rollback()` pour un
COMMIT, `rollbackAfterStatementFailure()` pour une instruction en validation automatique, et les
deux appellent `TransactionManager::rollback(clientContext, activeTransaction)`, qui lit
`transaction->getType()`, `hasFailed()`, appelle `transaction->rollback(&wal)` puis
`clearTransactionNoLock(id)` : tout cela sur un objet détruit. En debug, l'assertion de
`clearTransactionNoLock` (« la transaction est dans la liste ») tombe ; en release, l'annulation
rejoue l'undo d'une transaction libérée et `lockManager.releaseAll` reçoit un identifiant lu dans
la mémoire rendue.

Les deux autres sorties sont saines, et c'est pour elles que le `catch (CheckpointException)` de
`runFuncInTransaction` existe : le point de reprise forcé d'une validation (COPY FROM, index) et
le point de reprise au seuil écrit par une validation seule lèvent une `CheckpointException`
brute, après le retrait, et le contexte ne fait que `clearTransaction()` sans repasser par le
gestionnaire. Un `CHECKPOINT` explicite n'a pas de transaction ouverte : son `clearTransaction`
ne vide rien.

Ce que la transaction retient : rien. Elle a quitté `activeTransactions` avant l'échec ; elle ne
retient ni le point de reprise suivant (de toute façon refusé jusqu'à la réouverture) ni les
versions (`oldestActiveSnapshotNoLock` ne la voit plus). Le défaut est seulement l'annulation sur
l'objet détruit.

## Recette (lue, pas jouée)

Deux connexions, `auto_checkpoint` allumé, `checkpoint_threshold` bas, un `Checkpointer` qui lève
dans `checkpointStorage` (le `FlakyCheckpointer` des témoins) :

1. A : `BEGIN TRANSACTION ; CREATE (:Doc {id: 0})` (A reste ouverte).
2. B : des écritures validées au-delà du seuil : B pose la demande automatique (A8, A est ouverte)
   et rend la main.
3. Poser le `Checkpointer` en panne.
4. A : `COMMIT` : A part la dernière, écrit le point de reprise demandé, qui échoue ;
   `writeRequestedCheckpointIfLastNoLock` lève une `TransactionManagerException` ; le contexte de
   A appelle `rollback()` sur sa transaction détruite.

Attendu : A reçoit « This transaction is committed (it is in the journal); the automatic
checkpoint that followed it … failed » et sa connexion est vide de transaction. Lu : en debug,
assertion dans `clearTransactionNoLock` ; en release, lecture après libération (à prouver sous
ASAN ou en debug : le témoin ordinaire en Release ne le verra sans doute pas).

## Témoin

`CommitAfterFailedCheckpoint.DoesNotRollBackADestroyedTransaction`
(`test/transaction/concurrence/commit_after_failed_checkpoint_test.cpp`), en suite ordinaire et en
`known_red.txt`. La recette est jouée dans un processus fils, avec le `FlakyCheckpointer` qui lève
dans `checkpointStorage`. Contrôle
`commit-whose-checkpoint-fails-does-not-touch-a-destroyed-transaction` : le fils sort normalement,
et le COMMIT de A est refusé par le message qui dit la validation faite et le point de reprise
échoué.

**Une précision à la recette** : B doit franchir le seuil **en une seule validation**, et A valider
aussitôt. Avec plusieurs validations de B, les BEGIN suivants de B attendent la demande. Celle-ci
est abandonnée au bout de `AUTO_CHECKPOINT_WAIT` (250 ms), et A, seule, écrit alors le point de
reprise elle-même par le chemin sain : l'exception brute arrive, sans annulation.

Joué sur luciepc, trois fois sur trois de chaque côté :
- **Release** (`build-vingt`) : le COMMIT de A est refusé par « Invalid transaction type to
  rollback. », c'est-à-dire le type lu dans la mémoire libérée, puis le processus s'abandonne
  (SIGABRT) à la fermeture.
- **ASAN** (`RelWithDebInfo`, `ENABLE_ADDRESS_SANITIZER`, `ASAN_OPTIONS=detect_leaks=0`) :

```
ERROR: AddressSanitizer: heap-use-after-free … READ of size 1
#0 Transaction::getType() const              transaction.h:107
#1 TransactionManager::rollback(ClientContext&, Transaction*)
#2 TransactionContext::rollback()            transaction_context.cpp:85
#4 ClientContext::TransactionHelper::runFuncInTransaction
freed by: operator delete ← clearTransactionNoLock
```

## Forme proposée

`TransactionContext::commit` ne doit rendre la transaction au gestionnaire qu'une fois ; une
exception sortie de `TransactionManager::commit` après le retrait de la transaction doit le dire.
Deux formes : (a) les trois conversions en `TransactionManagerException` après
`clearTransactionNoLock` lèvent une exception dérivée que `runFuncInTransaction` traite comme
`CheckpointException` (vider le contexte sans annuler) ; (b) plus sûr, `TransactionContext::commit`
demande au gestionnaire, dans son `catch`, si la transaction est encore active
(`isActiveNoLock(id)`), et vide le contexte sans annuler si elle ne l'est plus. (b) couvre aussi
tout chemin futur. Fermé quand le témoin, en debug, passe par le chemin 4 sans assertion ni
annulation d'une transaction partie.
