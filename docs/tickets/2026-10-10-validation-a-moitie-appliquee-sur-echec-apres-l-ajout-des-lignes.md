# Une erreur pendant la validation, après l'ajout des lignes aux groupes, laisse la transaction à moitié validée

- **État** : ouvert (relevé par le banc le 10 octobre 2026, à la relecture d'A3′).
- **Gravité** : réponse fausse, puis plantage (un groupe de nœuds introuvable à la suppression
  suivante, SIGSEGV dans `NodeGroup::delete_`).
- **Atteignable en service** : en principe oui — toute erreur levée entre l'étape 1 et la fin de
  `NodeTable::commit` (mémoire, interruption, un index qui lève) ; vu une fois, sous le mode
  multi-écrivains, par un contrôle d'unicité qui différait entre l'insertion et la validation
  (corrigé dans A3′, voir « Cause »).
- **Touche rag3weaver** : pas de cas connu ; le seul chemin vu est fermé.

## Ce que c'est

`NodeTable::commit` fait, dans l'ordre : (1) ajouter toutes les lignes locales de la transaction
aux groupes de la table (`nodeGroups->append`) et les journaliser ; (2) poser le drapeau de
suppression des lignes locales supprimées ; (3) écrire les clés des lignes neuves dans l'index
de clé primaire et les lignes dans les autres index (`commitInsert`) ; (4) vider la table locale.
Si l'étape 3 lève — « Found duplicated primary key value », ou toute autre erreur —, l'exception
remonte par `Transaction::commit` et `TransactionManager::commit` jusqu'à l'appelant, qui annule
la transaction. Mais les lignes de l'étape 1 sont déjà dans les groupes, avec l'identifiant de la
transaction pour version ; l'annulation défait ce que le tampon d'annulation connaît, et ce que
la table compte (`getNumTotalRows`) et ce que ses blocs portent ne se correspondent plus. La
validation suivante calcule le décalage de ses propres lignes à partir de ce compte, et sa
suppression différée (étape 2) ne trouve plus de bloc : SIGSEGV.

PostgreSQL ne connaît pas ce cas parce que l'insertion dans la table et dans l'index unique se
font à l'instruction, pas au commit, et qu'un échec du commit après l'écriture du journal ne
laisse rien à défaire. Ici, le commit fait du travail qui peut échouer **après** avoir touché aux
groupes.

## Recette

Vue le 10 octobre (binaire d'A3′ avant son second correctif), mode multi-écrivains :

```
CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);
A: CREATE (:Item {id: 7, v: 0});
B: BEGIN TRANSACTION;  B: MATCH (n:Item) RETURN count(n);     -- instantané pris
A: MATCH (n:Item {id: 7}) DETACH DELETE n;                    -- validé
B: CREATE (:Item {id: 7, v: 1});                               -- passe (dernier état validé)
B: COMMIT;   -- levait « duplicated primary key » APRÈS l'étape 1, par un contrôle à l'instantané
```

Puis, dans C7 du banc (quatre écrivains, douze clés, créations et DETACH DELETE), la
suppression différée d'une validation suivante plantait. Le chemin est fermé depuis A3′ (une
seule visibilité d'unicité, `NodeTable::getUniquenessVisibleFunc`), mais le défaut général —
une erreur à l'étape 3 — reste.

## Témoin

`test/transaction/insert_lock_test.cpp`,
`InsertLockTest.AKeyDeletedAndCommittedAfterTheSnapshotIsFreeAtCommitToo`, pour le chemin
fermé. Pour le défaut général : aucun. Il faudrait forcer une erreur à l'étape 3 — un index
d'essai qui lève à `commitInsert`, ou une doublure de mémoire qui refuse à cet instant — puis
vérifier que la transaction suivante valide et que le vérificateur d'intégrité ne voit rien.

## Cause

Deux visibilités d'unicité (le chemin vu) ; plus généralement, l'ordre des étapes de
`NodeTable::commit` : ce qui peut échouer vient après ce qui touche aux groupes. **[lu]**

## Pour le fermer

Soit faire tous les contrôles de l'étape 3 avant l'étape 1 (le contrôle d'unicité sur les
lignes locales ne demande pas qu'elles soient dans les groupes), soit défaire l'étape 1 sur
échec (les lignes ajoutées, leurs clés déjà écrites, le journal local). Comme PostgreSQL :
rien de ce qui peut échouer après avoir touché à la table.
