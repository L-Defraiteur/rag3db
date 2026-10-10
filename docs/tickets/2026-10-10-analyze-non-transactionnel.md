# CALL analyze n'est pas transactionnel

- **État** : ouvert — accepté tel quel (orchestration, 10 octobre 2026 ; Lucie peut renverser)
- **Gravité** : estimation périmée (aucune réponse fausse : seuls le planificateur et
  `STATS_INFO` lisent ces comptes)
- **Atteignable en service** : oui, par un `CALL analyze` dans une transaction annulée
- **Touche rag3weaver** : non (il n'appelle ni `analyze` ni `STATS_INFO`)
- **Ouvert le** : 10 octobre 2026, seconde session cœur C++
- **Pour** : cœur C++

## Ce que c'est

`CALL analyze('Table')` remplace les statistiques de la table pendant son exécution
(`NodeTable::replaceStats`), pas à la validation de sa transaction : si la transaction est
annulée, les statistiques recalculées restent.

## Recette minimale

```cypher
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);
-- 2 000 lignes
BEGIN TRANSACTION;
MATCH (d:Doc) WHERE d.id >= 1000 DELETE d;
UNWIND range(5000, 5499) AS i CREATE (:Doc {id: i, name: 'new ' + CAST(i AS STRING)});
CALL analyze('Doc');
ROLLBACK;
MATCH (d:Doc) RETURN count(*);                    -- 2000
CALL STATS_INFO('Doc') RETURN cardinality;        -- 1000 : ce que l'analyze annulé a vu
```

L'analyze balaie les lignes validées en voyant les suppressions de sa transaction, et pas ses
insertions (locales).

## Témoin

`TableAnalyzeTest.AnAnalyzeInARolledBackTransactionLeavesAStaleEstimate`
(`test/transaction/table_analyze_test.cpp`) : la cardinalité gardée est celle que l'analyze a
vue, les distincts n'excèdent pas les lignes, et un analyze hors transaction remet tout juste.

## Pourquoi c'est accepté

PostgreSQL fait de même pour `reltuples`/`relpages` : ANALYZE et VACUUM les mettent à jour sur
place, hors transaction, et ils survivent au `ROLLBACK` ; seul `pg_statistic` est
transactionnel (Tom Lane, liste pgsql-bugs, 22 octobre 2014, BUG #11638 :
<https://www.postgresql.org/message-id/10043.1413988524%40sss.pgh.pa.us>). Notre analyze se
comporte comme `reltuples`. Ce sont des estimations du planificateur, jamais une donnée.

## Pour le fermer, si un jour on le veut

Un « remplacement en attente » dans `LocalStorage`, à côté des statistiques en attente d'un
`COPY` (`addPendingNodeStats`) : remplacé à la validation, jeté à l'annulation, et vu par la
transaction elle-même (`NodeTable::getStats`). Le témoin change alors d'attendu (la cardinalité
d'avant l'analyze).
