# Sous IGNORE_ERRORS, laquelle de deux lignes de même clé un COPY garde dépend des fils

- **État** : ouvert — confort (session cœur C++ et orchestration, 10 octobre 2026)
- **Gravité** : réponse non déterministe (aucune perte : une seule des deux lignes reste, juste)
- **Atteignable en service** : oui, par un `COPY … (IGNORE_ERRORS=true)` à plusieurs fils qui
  porte deux fois une même clé
- **Touche rag3weaver** : non (il n'écrit jamais `IGNORE_ERRORS`)
- **Ouvert le** : 10 octobre 2026, seconde session cœur C++
- **Pour** : cœur C++

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
