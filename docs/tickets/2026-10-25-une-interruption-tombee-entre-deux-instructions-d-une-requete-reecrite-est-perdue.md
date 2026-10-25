# Une interruption tombée entre deux instructions d'une requête réécrite est perdue

- **État** : ouvert, le 25 octobre 2026 (seconde session cœur C++, trouvé en écrivant les témoins
  de (A), le CREATE concurrent). Aucun correctif écrit.
- **Gravité** : une interruption ignorée. La requête continue jusqu'au bout au lieu de s'arrêter,
  sans erreur ni avertissement.
- **Atteignable en service** : oui, pour toute fonction qui se réécrit en plusieurs instructions
  (`CREATE_VECTOR_INDEX`, `DROP_VECTOR_INDEX`, les index plein texte et spatiaux, la migration
  neo4j). La fenêtre est étroite, sauf quand une instruction de la suite attend : l'attente des
  écrivains plus anciens du CREATE concurrent en est une.
- **Touche rag3weaver** : oui, s'il interrompt une connexion pendant un CREATE (arrêt d'un run,
  arrêt de l'hôte).
- **Pour** : le cœur C++.

## Ce qu'on voit

Une connexion lance `CALL CREATE_VECTOR_INDEX(...)`. Une autre appelle `Connection::interrupt()`
dès que l'index est visible en construction. Le CREATE finit pourtant « ok », comme si rien
n'avait été demandé.

## La cause

`ClientContext::interrupt()` pose `activeQuery.interrupted`, que les opérateurs lisent dans
`PhysicalOperator::getNextTuple` et qui lève `InterruptException`. Mais `executeNoLock` appelle
`resetActiveQuery()` au début de **chaque** instruction (client_context.cpp, autour de la ligne
520). Une requête réécrite par `parseQuery` est une suite d'instructions, exécutées une à une
par `queryNoLock`. Une interruption qui tombe pendant une instruction qui ne passe par aucun
opérateur (une attente, un BEGIN, un COMMIT), ou entre deux instructions, est donc remise à zéro
par l'instruction suivante.

## La recette

`VectorIndexCreateStates.AnInterruptedCreateLeavesAnIndexToRebuild`
(`test/transaction/concurrence/vector_index_update_test.cpp`, branche create-concurrently-etat).
Sa première version appelait `creator.interrupt()` une seule fois, dès l'index vu NOTED, donc
pendant l'attente des écrivains (`_WAIT_HNSW_WRITERS`) ou juste avant le bâti, et le CREATE
finissait « ok ». Le témoin répète désormais l'interruption jusqu'au retour du CREATE pour
qu'elle tombe pendant le bâti. C'est ce contournement que ce ticket doit rendre inutile.

## Pour le fermer

L'interruption vaut pour la **requête**, pas pour l'instruction. `queryNoLock` remet le drapeau à
zéro une seule fois, avant la première instruction, et ne le fait plus pour les suivantes. Une
instruction qui commence avec le drapeau posé lève aussitôt `InterruptException`. Les attentes
longues d'une suite (`waitForWriteTransactionsStartedBefore`) le lisent aussi pendant qu'elles
attendent. Témoin : une seule interruption, pendant l'attente des écrivains d'un CREATE retenu par
un écrivain ouvert, arrête le CREATE.
