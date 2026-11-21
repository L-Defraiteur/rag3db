# La classe et ses membres ne se demandent pas ensemble

- **État** : ouvert (22 novembre 2026)
- **Famille** : produit
- **Gravité** : basse. La donnée est là, c'est la question qui manque.
- **Atteignable en service** : oui, depuis le lot `class-users` : une classe ne porte plus les références de ses
  méthodes.
- **Touche rag3weaver** : oui (les gabarits des outils `callees` et `impact`)
- **Pour** : session codeparsers, après la v1 (c7)
- **Signalé par** : Lucie, par c7, au lot `class-users` : « il y a intérêt des fois à rechercher toutes les
  références de la classe, mais c'est à faire par gabarit, pas forcé dans le parsage »

## Constat

Depuis que codeparsers n'attribue une référence qu'au scope le plus étroit, la question « tout ce que la classe et ses
méthodes utilisent », comme « qui utilise la classe ou ses méthodes », ne se pose par aucun outil :
- `usages` ne part que des définitions du nom ;
- le `also_path` d'`impact` n'atteint que la méthode de base de même nom ;
- `callees` n'a pas d'`also_path`.

Le moteur le permet déjà (`NeighborhoodNode`, `also_path` et `also_same` vide).

## Ce qu'il faut

Un paramètre typé, par exemple `include_members: true`, sur `callees` et `impact`. Il ajoute le départ
`also_path='PARENT_OF>'` avec `also_same=''`, et `PARENT_OF>+|PARENT_OF>?` pour les classes imbriquées. La requête
est détaillée dans `docs/codeparsers/22-11-2026/01-ce-que-codeparsers-fabrique.md`.

## Témoin attendu

Une classe `C`, avec une méthode `m` qui appelle `f`, et une fonction `u` qui appelle `C::m` :
- `callees C include_members` rend `f` (par `m`) ;
- `impact C include_members` rend `u`.
