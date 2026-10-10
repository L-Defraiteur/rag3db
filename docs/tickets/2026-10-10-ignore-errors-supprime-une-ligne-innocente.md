# Sous IGNORE_ERRORS, un doublon de clé fait supprimer une ligne innocente

- **État** : corrigé `de8fc8c0f` (10 octobre 2026) — témoin
  `IgnoreErrorsDuplicateKeyTest` (`test/transaction/ignore_errors_duplicate_key_test.cpp`), rouge
  avant (`4b2896ae1`), vert après ; relu par la session cœur C++
- **Gravité** : perte (une ligne validée supprimée) et réponse fausse (le doublon reste, sans
  entrée d'index), en silence
- **Atteignable en service** : oui — un `COPY … (IGNORE_ERRORS=true)` qui porte deux fois une
  même clé, dans une table dont l'index de clé a déjà des entrées sur disque
- **Touche rag3weaver** : non — il n'écrit jamais `IGNORE_ERRORS` (ses seuls `COPY` sont
  `dialect.rs:1322` et `:1341` ; un doublon y est refusé, filet `f0f7f0951` ; vérifié par la
  session de l'arbre principal). Défaut du moteur quand même : condition 4 de la stèle
- **Ouvert le** : 10 octobre 2026, seconde session cœur C++, sur le rouge probabiliste relevé par
  la session cœur C++
- **Pour** : cœur C++ — condition 4 de la stèle

## Ce que c'est

Quand un `COPY` sous `IGNORE_ERRORS` porte deux lignes de même clé et que l'index de clé primaire a
déjà des entrées sur disque, la ligne supprimée n'est pas forcément le doublon : c'est la ligne
qui se trouve, dans le paquet de l'index, à la position du nombre d'insertions réussies.

## Recette minimale (déterministe, un seul fil)

```cypher
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);
UNWIND range(0, 199) AS i CREATE (:Doc {id: i, name: 'old ' + CAST(i AS STRING)});
CHECKPOINT;   -- l'index de clé a des entrées sur disque
-- f.csv : 1000 … 1010, puis « 1005,dup », puis 1011 … 2999
COPY Doc FROM 'f.csv' (IGNORE_ERRORS=true, PARALLEL=false);
MATCH (d:Doc) WHERE d.id >= 1000 RETURN count(*), count(DISTINCT d.id);  -- 2000, 1999
MATCH (d:Doc) WHERE d.name = 'dup' RETURN count(*);                        -- 1 (attendu 0)
-- et une clé de 1000 … 2999 introuvable par MATCH (d:Doc {id: …})
```

Sans le `CHECKPOINT` (l'index tout en mémoire) : 2 000 lignes, 2 000 id distincts, pas de « dup ».
Mesuré le 10 octobre sur luciepc (sonde jetable, `~/.cache/rag3db-tickets-notes/ignore_errors_probe_scratch_test.cpp`).

## Témoin

`IgnoreErrorsDuplicateKeyTest.KeysOnDiskSkipOnlyTheDuplicate` : rouge avant le correctif (une clé
introuvable, le doublon resté, l'avertissement nommait 2782 au lieu de 1005), vert après. Sa
variante à clé chaîne était verte avant (le doublon tombait en fin de son paquet) : elle couvre,
elle ne témoigne pas. Après le correctif, la sonde à plusieurs fils (30 passes) ne perd plus
aucune ligne. Le rouge probabiliste
`ForcedTransactionJournalTest.OrdinaryWritesBeforeACopyThatSkipsRowsCommittedThenDead`
(`journaled_copy_test.cpp`, 1 sur 3 passes, la ligne « 17,again » restée et une ligne juste en
moins) en est très probablement la forme à plusieurs fils : l'ordre des fils décide seulement si
le doublon tombe en fin de paquet, où le compte fautif vise par hasard la bonne ligne.

## Cause (lue, puis exécutée)

- `HashIndex::appendNoLock` (`src/include/storage/index/hash_index.h:147-167`), branche
  `indexHeaderForWriteTrx.numEntries > 0` : quand `localStorage->appendNoLock(key, …)` refuse un
  doublon venu du même `COPY`, la boucle continue et rend le nombre d'insertions **réussies**.
- `IndexBuilder::maybeConsumeIndex` (`src/processor/operator/persistent/index_builder.cpp:73-93`)
  le prend pour la position du premier échec : `buffer[insertBufferOffset + numValuesInserted]`
  part à `handleError`, donc à `NodeTable::delete_`. Puis `insertBufferOffset += 1 + n` repasse
  des entrées déjà insérées — une cascade est possible, non mesurée.
- La branche sans entrée sur disque (`InMemHashIndex::append`) s'arrête au premier échec : juste.

Sans `IGNORE_ERRORS`, le `COPY` est refusé et annulé ; seul le message nomme peut-être une autre
clé que le doublon (lu, non exécuté).

## Correctif (`de8fc8c0f`)

Dans la branche disque, s'arrêter au premier refus local comme la branche mémoire :
`if (!localStorage->appendNoLock(...)) { return i - bufferOffset; }`. La clé n'est déplacée qu'à
l'insertion : le refus nomme la bonne. Le test probabiliste reste rouge 13 fois sur 40 après le
correctif, mais pour une autre raison — laquelle des deux lignes 17 survit suit l'ordre des fils :
il tolère maintenant l'une ou l'autre (`9bd9087cc`), 40 vertes sur 40. Ticket de confort à part :
`2026-10-10-copy-garde-un-doublon-selon-les-fils.md`.

## À part (confort)

Laquelle de deux lignes de même clé un `COPY` garde dépend de l'ordre d'arrivée dans l'index, donc
de l'ordre des fils (les tests d'origine le disent : `test/test_files/exceptions/copy/duplicated.test`,
« the reported lines non-deterministic », `PARALLEL=false`). PostgreSQL n'offre pas de référence :
l'`ON_ERROR ignore` de son `COPY` ne couvre que les erreurs de conversion
([COPY](https://www.postgresql.org/docs/current/sql-copy.html)).
