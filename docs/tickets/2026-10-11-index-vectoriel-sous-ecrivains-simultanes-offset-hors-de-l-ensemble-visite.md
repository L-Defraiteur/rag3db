# Sous écrivains simultanés, l'index vectoriel refuse une instruction : « node offset N is beyond the visited set of N nodes »

- **État** : ouvert
- **Gravité** : refus d'une instruction valide (aucune perte vue : l'intégrité reste juste après le refus et après une réouverture)
- **Atteignable en service** : oui, depuis la bascule du défaut (`a0a79016e`), dès que deux écrivains insèrent ou mettent à jour une même table indexée par HNSW
- **Touche rag3weaver** : oui en principe, puisque ses tables de morceaux portent un index vectoriel et que le mode est maintenant le défaut. Non vu en usage.
- **Ouvert le** : 11 octobre 2026, session du banc (preuve de la bascule)
- **Pour** : à attribuer (extension vector sous le mode multi-écrivains)

## Ce que c'est

Avec plusieurs écrivains sur une table indexée, des instructions échouent sur :

> Runtime exception: HNSW index: node offset 310 is beyond the visited set of 310 nodes. The index
> graph or the row count of its table is wrong; drop the index and build it again.

Le message demande de rebâtir l'index. Le vérificateur d'intégrité ne trouve pourtant aucune
violation, ni dans le processus écrivain ni après fermeture et réouverture.

## Recette minimale

Aucune de déterministe pour l'instant.

## Témoin

`Launchers/ConcurrencyBench.H4_IndexedRandomMix/Thread_Hot` et `/Thread_Crash` (le mélange de C7 sur
une table indexée), contrôle `no-unexpected-error`. Ils rougissent **10 fois sur 10 sur master
`fd10b8c27` et `481afb550`, hors bascule**, et autant sur `4ee70e386`. L'erreur vient de plusieurs
écrivains par passe, avec un offset chaque fois égal à la taille de l'ensemble visité : 307, 310,
329. Le même témoin montrait aussi un refus des verrous (« no primary key to lock: it is not
visible »), corrigé par le cœur C++ sur `verrous-pk-lock` : après ce correctif, ce ticket suffit à
garder H4 rouge.

## Cause

À instruire. L'hypothèse du cœur C++ : la taille de l'ensemble visité est prise avant que l'autre
écrivain ait inséré ses lignes. Un offset égal à la taille, chaque fois, va dans ce sens : la
première ligne posée par l'autre est hors de l'ensemble.

## Pour le fermer

Dimensionner l'ensemble visité sur ce que la transaction peut voir au moment de la recherche, ou
ignorer un voisin au-delà de lui au lieu de refuser. H4 au vert.
