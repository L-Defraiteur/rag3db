# Un COPY de relations annulé laisse gonflée l'estimation du nombre de relations

- **État** : ouvert — confort, hors stèle (classé par l'orchestration, 5 octobre 2026)
- **Gravité** : estimation fausse (plans moins bons), aucune réponse fausse
- **Atteignable en service** : oui, par tout `COPY` de relations annulé ou refusé
- **Touche rag3weaver** : au plus des plans plus lents après des paquets défaits
- **Ouvert le** : 5 octobre 2026, session cœur C++ (par lecture, en rangeant les statistiques du `COPY` de nœuds dans sa transaction)
- **Pour** : cœur C++

## Ce que c'est

Les tables de relations n'ont pas de statistiques tenues (`STATS_INFO` les refuse). Le planificateur estime leur nombre par `RelTable::getNumTotalRows`, c'est-à-dire `nextRelOffset` plus les relations locales (`CardinalityEstimator::getNumRels`). `nextRelOffset` est avancé par le partitionneur d'un `COPY` (`reserveRelOffsets`) et par le commit des relations locales ; aucune annulation ne le recule, et il est écrit sur disque.

Par lecture et recherche des écritures de `nextRelOffset` ; non exécuté.

## Pourquoi ce n'est pas corrigé avec les nœuds

`nextRelOffset` n'est pas qu'une estimation : c'est l'allocateur des identités des relations. Le reculer à l'annulation rendrait des identités déjà réservées, ce qui n'est pas sûr sous plusieurs écrivains. La voie juste est une estimation à part, tenue comme celle des nœuds (en attente dans la transaction, fusionnée à la validation), ou recalée au point de reprise sur le nombre réel de relations.

## Pour le fermer

Un témoin (`COPY` de relations annulé, puis l'estimation lue par `EXPLAIN` ou par un accès de test), et l'une des deux voies.
