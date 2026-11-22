# Sous IGNORE_ERRORS, laquelle de deux lignes de même clé un COPY garde dépend des fils

- **État** : fermé (22 novembre 2026, hotfixs), corrigé sur la branche `copy-first-line-wins` (3bc15502f2 avant
  la repose) : la première ligne du fichier gagne. Au conflit de clé, sous le verrou de la partition de l'index,
  `IndexBuilderGlobalQueues::maybeConsumeIndex` compare la position des deux lignes (fichier, bloc, ligne du bloc,
  déjà portée sous `IGNORE_ERRORS`). Si le titulaire de la clé est plus tardif, il est écarté : sa ligne est
  supprimée, sa propre ligne signalée, son entrée locale retirée, et l'entrée réessayée. Une clé tenue par une ligne
  d'avant le COPY garde sa ligne, comme avant. La preuve : `CopyFirstLineWinsTest.TheFirstLineOfTheFileKeepsItsKey`
  (30 COPY à 8 fils de 20 000 lignes, trois clés reprises en fin de fichier), **rouge sur master 3895334012 : 28
  COPY sur 30 gardaient une ligne tardive**, **vert avec le correctif, 30 sur 30**. Le témoin du journal
  (`ForcedTransactionJournalTest.OrdinaryWritesBeforeACopyThatSkipsRowsCommittedThenDead`) attend désormais la
  première ligne. Le coût : la position de chaque ligne du COPY est gardée jusqu'à sa fin, sous `IGNORE_ERRORS`
  seulement.
- **Gravité** : réponse non déterministe (aucune perte : une seule des deux lignes reste, juste)
- **Atteignable en service** : oui, par un `COPY … (IGNORE_ERRORS=true)` à plusieurs fils qui
  porte deux fois une même clé
- **Touche rag3weaver** : non (il n'écrit jamais `IGNORE_ERRORS`)
- **Ouvert le** : 10 octobre 2026, seconde session cœur C++
- **Pour** : cœur C++
- **Famille** : moteur

## Ce que c'est

Le `COPY` garde la ligne qui arrive la première dans l'index de clé primaire, et écarte l'autre.
L'ordre d'arrivée suit celui des fils de lecture du CSV (blocs de 8 Kio, chaque fil remplit son
groupe, l'ordre de fin des fils fixe les décalages ; l'`IndexBuilder` vide ses files dans l'ordre
où elles sont poussées), pas celui du fichier : aucun tri par décalage ni par ligne.

## Mesuré

Sonde jetable du 10 octobre (luciepc, après le correctif `de8fc8c0f`) : 3 000 lignes et une
seconde ligne de clé 17 en fin de fichier, 30 passes à plusieurs fils — la seconde ligne a gardé
la clé dans 3 passes, et aucune ligne innocente n'est partie.

## Ce que disent les tests d'origine

`test/test_files/exceptions/copy/duplicated.test:81-82` : « If the vector size is 2 the CSV can be
read with multiple threads which makes the reported lines non-deterministic » — le cas y est
joué en `PARALLEL=false`.

## Ce que font les moteurs établis

Pas de référence directe : l'`ON_ERROR ignore` du `COPY` de PostgreSQL ne couvre que les erreurs
de conversion, une violation d'unicité fait échouer le `COPY`
([COPY](https://www.postgresql.org/docs/current/sql-copy.html)).

## Pour le fermer (si on le veut)

Garantir « la première du fichier gagne » : porter le numéro de ligne du fichier jusqu'à l'index
et, entre deux clés égales, garder la plus petite — une réécriture de l'`IndexBuilder`, les
décalages ne suivant pas l'ordre du fichier. Ou le dire dans la documentation du `COPY`. Témoin :
`ForcedTransactionJournalTest.OrdinaryWritesBeforeACopyThatSkipsRowsCommittedThenDead`, qui tolère
aujourd'hui l'une ou l'autre des deux lignes (commit du 10 octobre).
