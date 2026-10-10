# Un COPY de relations annulé laisse gonflée l'estimation du nombre de relations

- **État** : ouvert — confort, hors stèle (orchestration, 5 octobre 2026) ; rien à coder tant qu'un mauvais plan n'est pas observé (orchestration, 10 octobre 2026)
- **Gravité** : estimation fausse (plans moins bons), aucune réponse fausse
- **Atteignable en service** : oui, par tout `COPY` de relations annulé ou refusé, et par tout `DELETE` de relations
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

## Relu le 10 octobre 2026 (seconde session cœur C++), par les lignes

- `nextRelOffset` n'est écrit qu'à trois endroits : mis à 0 à la création
  (`rel_table.cpp:142`), avancé par `reserveRelOffsets` (`rel_table.h:214-219`, `+= numRels`, sous
  `relOffsetMtx`), relu du disque (`rel_table.cpp:561`).
- Le planificateur lit `RelTable::getNumTotalRows` = relations locales + `nextRelOffset`
  (`rel_table.cpp:542-547`, `CardinalityEstimator::getNumRels`, `cardinality_estimator.cpp:204-211`).
- Le ticket est plus large que son titre : **ni l'annulation ni un `DELETE`** ne reculent ce
  compte. L'estimation compte toutes les relations jamais créées.
- `getNumTotalRows` est intouchable : `nextRelOffset` est aussi l'allocateur des identités des
  relations, et la borne des décalages pour ses autres appelants.

## La forme retenue le jour où un mauvais plan est observé

Une estimation à part pour le planificateur : la cardinalité du `TableStats` des tables de
relations (porté par la `NodeGroupCollection` de `RelTableData`, sans lecteur aujourd'hui), recalée
au point de reprise sur les relations vivantes comme celle des nœuds l'est depuis le 10 octobre —
sans relire les colonnes, par la longueur des listes CSR persistées après le point de reprise (à
vérifier dans `CSRNodeGroup`) —, plus les relations locales ; `getNumRels` la lit à la place de
`getNumTotalRows`. Témoin rouge d'abord : l'estimation après un `COPY` de relations annulé et après
un `DELETE`, par un accès de test. Fichiers de stockage de la session cœur C++.
