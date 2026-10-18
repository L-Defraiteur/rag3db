# Un BEGIN refusé laisse la connexion en transaction explicite, sans transaction : la requête suivante plante

- **État** : ouvert
- **Gravité** : plantage du processus (SIGSEGV), sur la requête qui suit un BEGIN refusé
- **Atteignable en service** : oui. Un hôte dont le tampon est plein refuse un BEGIN par
  « Unable to allocate memory », et la requête suivante de la même connexion tue l'hôte. Le même
  enchaînement se produit après un point de reprise échoué, ou en mode un seul écrivain.
- **Touche rag3weaver** : oui, par l'hôte, qui garde ses connexions d'une requête à l'autre
- **Ouvert le** : 18 octobre 2026, banc de concurrence, au premier jeu du scénario des vingt
  écrivains sur luciepc (master 87a362e30)
- **Pour** : cœur C++

## Ce qu'on voit

Le scénario des vingt écrivains (`test/transaction/concurrence/twenty_writers_scenario_test.cpp`)
plante environ une seconde après le départ des acteurs, avant tout DDL, avec le tampon des tests
(64 Mo). La pile, sous gdb :

```
#0 CatalogSet::traverseVersionChainsForTransactionNoLock(Transaction const*, CatalogEntry*)
#1 CatalogSet::containsEntryNoLock
#3 Catalog::containsTable
#4 Binder::bindNodeTableEntry
#10 Binder::bindMergeClause
#16 ClientContext::prepareNoLock … lambda #2
#17 ClientContext::TransactionHelper::runFuncInTransaction
rdi = 0   (mov 0x8(%rdi),%rdx : la transaction est nulle)
```

Chaque refus a été journalisé avec le numéro de son fil. Le fil qui plante a toujours la même
histoire juste avant :

```
BEGIN    -> Buffer manager exception: Unable to allocate memory! The buffer pool is full …
ROLLBACK -> No active transaction for ROLLBACK.
(MERGE en automatique : SIGSEGV)
```

Ce qui a été écarté, sur luciepc :
- **le harnais** : une connexion par fil, et le plantage revient sans fork ;
- **un bloc qui échoue** : sept séquences sur un fil (lien refusé, exécution refusée, COMMIT ou
  ROLLBACK en trop) sont refusées sous leur nom et la connexion reprend saine ;
- **le tampon plein seul** : 20 fils en automatique, 1 252 refus « Unable to allocate memory »
  en 8 s, aucun plantage. Le plantage ne vient que des blocs.

Les blocs avec vecteur plantent 3 fois sur 3 à 64 Mo et passent 3 fois sur 3 à 2 Gio.

## La cause

`TransactionContext::beginWriteTransaction` (et `beginReadTransaction`) pose `mode = MANUAL`
**avant** d'appeler `TransactionManager::beginTransaction` (`transaction_context.cpp:25-29`).
Si celle-ci lève, le mode reste MANUAL et `activeTransaction` reste nulle :
- `ROLLBACK` et `COMMIT` voient `!hasActiveTransaction()`, répondent « No active transaction » et
  ne rendent pas le mode (`clearTransaction` n'est pas appelé) ;
- la requête suivante, se croyant dans un bloc, ne commence pas de transaction automatique et se
  lie avec `Transaction::Get(context) == nullptr`. `RETURN 1` passe, mais tout ce qui lit le
  catalogue plante.

`beginTransaction` lève de trois façons (`transaction_manager.cpp:51-88`) :
1. un second écrivain quand le mode multi-écrivains est éteint (« Only one write transaction at a
   time ») ;
2. `throwIfCheckpointFailedNoLock`, après un point de reprise échoué ;
3. le tampon plein, à la construction de la `Transaction` ou à `logBeginTransaction`.

## La recette réduite, déterministe

Deux connexions et un fil, le mode multi-écrivains éteint :

```
A: CALL enable_multi_writes=false;
A: BEGIN TRANSACTION;    A: CREATE (:Doc {id: 1, titre: 'a'});
B: BEGIN TRANSACTION;    -> refus « Cannot start a new write transaction … »
B: ROLLBACK;             -> refus « No active transaction for ROLLBACK. »
B: RETURN 1;             -> ok
B: MERGE (d:Doc {id: 2}) SET d.titre = 'b';   -> SIGSEGV
```

## Témoin

`FailedBegin.LeavesTheConnectionUsable` (`test/transaction/concurrence/failed_begin_test.cpp`),
dans `known_red.txt`. La recette se joue dans un processus fils. Contrôle :
`failed-begin-leaves-the-connection-usable`. Attendu après le correctif : le fils ne meurt pas,
et le MERGE passe en automatique, ou est refusé sous un nom.

## Forme proposée

Ne poser le mode MANUAL qu'une fois la transaction obtenue, ou bien rendre le mode AUTO si
`beginTransactionInternal` lève (`clearTransaction` dans un `catch` qui relance). Le choix
appartient au cœur.

## Effet sur le scénario

Pour juger E10 malgré ce ticket, le scénario des vingt écrivains joue avec un tampon de 2 Gio.
Ce tampon est aussi celui du produit, et il est dit dans le rapport du banc. Le tampon plein reste
un déclencheur ; seul le témoin ci-dessus garde ce défaut en vue.
