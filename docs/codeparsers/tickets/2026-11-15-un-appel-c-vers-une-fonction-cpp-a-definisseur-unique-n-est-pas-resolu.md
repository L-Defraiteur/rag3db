# Un appel C vers une fonction C++ à définisseur unique n'est pas résolu

- **État** : ouvert (15 novembre 2026)
- **Famille** : produit (la résolution des arêtes)
- **Gravité** : défaut. `usages`, `impact` et les appelants manquent, pour un appel que l'index a pourtant vu
- **Atteignable en service** : oui. Toute base qui indexe `examples/c/main.c` et `src/c_api/` du dépôt
- **Touche rag3weaver** : oui (`src/code.rs`, `Catalog::materialiser_les_symboles`, la résolution d'une MENTIONS en arête)
- **Pour** : codeparsers
- **Signalé par** : ED, en départageant les CONSUMES perdus du juge étendu de banc (décision de c7, 15 novembre 2026, 20 h 40)

## Ce que c'est

`examples/c/main.c|main` appelle `rag3db_connection_init`, défini une seule fois, dans `src/c_api/connection.cpp`. Au lot
comme au parent, l'index a :

- la MENTIONS `main → Symbol(rag3db_connection_init)`, de genre `CONSUMES`, sans type de qualificateur, sans module d'import,
  sans différé ;
- un seul DEFINES vers ce symbole, depuis `src/c_api/connection.cpp|rag3db_connection_init`.

Et pourtant aucune arête `CONSUMES` ne se pose, même à une première indexation. Les deux fichiers sont de la même famille
de langage (`famille_de_langage` : `c` et `cpp` sont `c++`). Les anciennes bases avaient cette arête : la règle qui la
refuse est donc arrivée après leur indexation. Sur la base migrée du dépôt, 2 533 CONSUMES vers un nom à définisseur unique
sont dans ce cas (`examples/c/main.c` → les fonctions de `src/c_api/`, et d'autres). Seule une base indexée sous l'ancienne
règle les garde.

## La recette

Le témoin 19 de l'écrivain (`code::tests_de_l_ecrivain::un_repassage_paquet_par_paquet_garde_les_consumes`, branche
`ecrivain-2a-prime-2-pile-2`, 73ae2e57c) indexait les vrais fichiers par `sync_source`. Il a rougi dès la base :

```
définisseurs [["src/c_api/connection.cpp", "rag3db_connection_init"]] ; mentions [["examples/c/main.c", "main", "CONSUMES"]]
appelants de rag3db_connection_init : []
```

Avec l'en-tête `src/include/c_api/rag3db.h` dans l'index, le nom a plusieurs définisseurs, et l'abstention est alors voulue.
Le cas à expliquer est celui sans l'en-tête, ou bien celui où les déclarations de l'en-tête ne comptent pas comme des
définisseurs : le nom n'a qu'un définisseur, et l'arête manque.

Le témoin, rouge d'abord, à écrire chez codeparsers : `main.c` et `connection.cpp` du dépôt indexés seuls, et
`appelants(rag3db_connection_init) == ["main"]`. Ou bien la règle qui refuse est voulue, et ce ticket se ferme en le disant,
avec la raison.

## Comment le fermer

Par le témoin vert, ou par la raison écrite ici. Le témoin 19 de l'écrivain ne juge que la non-perte au repassage : il ne
change pas avec ce ticket.
