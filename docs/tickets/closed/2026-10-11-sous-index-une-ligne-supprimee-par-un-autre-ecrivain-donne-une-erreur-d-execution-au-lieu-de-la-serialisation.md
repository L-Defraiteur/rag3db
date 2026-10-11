# Sous index, une ligne supprimée par un autre écrivain donne une erreur d'exécution au lieu de l'erreur de sérialisation

- **État** : corrigé par le commit qui porte ce ticket (11 octobre 2026, 18 h 40 ; ouvert et fermé dans
  le même commit — relevé par le banc dans sa passe de preuve de la bascule, 10 fois sur 10 sur
  master `fd10b8c27` et `481afb550`, hors bascule).
- **Gravité** : réponse mal nommée (P3 du moteur sous le défaut) — l'écriture est bien refusée,
  rien n'est corrompu, mais l'erreur est une `RuntimeException` d'exécution là où le contrat des
  verrous (A4′) promet « could not serialize access due to concurrent update », que l'agent sait
  rejouer.
- **Atteignable en service** : oui, depuis que le mode multi-écrivains est le défaut (a0a79016e) :
  deux sessions, l'une lit une ligne d'une table à index vectoriel, l'autre la supprime et valide,
  la première la met à jour.
- **Touche rag3weaver** : oui (l'indexeur et l'agent écrivent la même table de documents).

## Ce que c'est

`H4_IndexedRandomMix/Thread_Hot` et `Thread_Crash` du banc : « Row 314 of table Doc has no
primary key to lock: it is not visible to this transaction » — un refus inattendu, classé
`Unexpected`, dix fois sur dix.

Cause : `NodeTable::update`, sur une ligne validée, prend d'abord l'index (`lockIndexForWrite`,
marche I1), qui **rafraîchit l'instantané** d'une transaction qui n'a rien écrit ; puis il lit la
clé de la ligne pour la verrouiller (`lockKeyOfRow`, `lookup` avec ce nouvel instantané). Si un
autre écrivain a supprimé et validé la ligne entre le `MATCH` et le `SET`, elle n'est plus visible,
et `lockKeyOfRow` jetait une `RuntimeException`. Sans index, le chemin est juste : l'instantané
n'est pas rafraîchi, la clé se lit, et `lockRowForWrite` lève l'erreur nommée.

## Forme

`lockKeyOfRow` : quand la clé ne se lit pas, si la ligne n'est pas visible au dernier état validé,
c'est l'erreur de sérialisation nommée (`TransactionManagerException`, fragment
`could not serialize access due to concurrent update`, « row N of table T was deleted by another
transaction after this one started ») ; la `RuntimeException` reste pour le cas impossible (visible
et sans clé).

## Témoin

`IndexLockBench.AnUpdateOfARowDeletedByAnotherCommitIsASerializationError`
(`test/transaction/concurrence/lock_bench_test.cpp`) : un bloc lit la ligne 5 de `Doc` (index HNSW),
une autre connexion la supprime et valide, le bloc la met à jour — refusé avec le fragment nommé,
jamais « no primary key to lock » ; après `ROLLBACK`, la ligne est absente, l'intégrité juste.
Rouge sur master, vert après.

`H4_IndexedRandomMix/Thread_Hot` et `Thread_Crash` restent dans `probabilistic.txt` : leur second
message, « HNSW index: node offset 310 is beyond the visited set of 310 nodes », est un autre
défaut (ticket du banc), pas celui-ci.
