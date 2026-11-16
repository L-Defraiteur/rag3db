# Un appel C vers une fonction C++ à définisseur unique n'est pas résolu

- **État** : fermé (16 novembre 2026) — deux défauts de codeparsers (`6aec30d`), voir « La cause » et « La correction »
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

## La cause (16 novembre 2026)

L'abstention n'était pas voulue. C'est la visibilité par les `#include` (25 octobre) qui la produisait,
sur deux défauts de codeparsers. Un définisseur C/C++ n'est candidat que si son fichier est dans la
fermeture des includes du mentionneur, si un en-tête de même nom de base y est, ou si une de ses
**déclarations** y est. `connection.cpp` n'a pas d'en-tête `connection.h`, et sa déclaration est dans
`rag3db.h` :

1. **Une source `.c` ne relevait aucun `#include`.** Le parseur C prenait ses imports par la voie
   générique, qui ne voit pas les `#include`, et `File.includes` de `main.c` valait `""`. Sa fermeture se
   réduisait à elle-même : tout appel d'une source C vers un autre fichier était « invisible ».
2. **`rag3db.h` perdait ses 167 prototypes.** La garde `#ifdef __cplusplus` / `extern "C" {` / `#endif`,
   avec l'accolade fermée dans un autre `#ifdef`, puis un `#if defined(__APPLE__)` dans le corps d'une
   struct plus loin, faisait perdre à tree-sitter tout ce qui suivait. Le scope de fichier n'avait
   aucune déclaration. La macro d'export `RAG3DB_C_API`, soupçonnée d'abord, était déjà masquée.

La recette à deux fichiers (`main.c` et `connection.cpp` seuls) ne pouvait pas rendre l'arête : sans
l'en-tête, rien ne déclare la fonction, et l'abstention y est juste. La recette juste prend les trois
fichiers du dépôt.

## La correction

Codeparsers `6aec30d` : le parseur C relève ses includes comme le C++ (`extract_cpp_includes`), et les
deux lignes de garde `extern "C"` sont masquées pour l'arbre, comme les macros d'export, sans changer
les octets ni les lignes.

Les témoins : `tests/e2e_usages.rs`, `c_api_prototype_is_visible_through_the_includes_of_a_c_source`, sur
les vrais `examples/c/main.c`, `src/include/c_api/rag3db.h` et `src/c_api/connection.cpp`. Il est rouge
sans l'une ou l'autre des deux corrections (13 abstentions « invisible »), vert avec les deux. Côté
codeparsers, `tests/prototypes_c.rs`, `a_c_source_records_its_includes`, vu rouge puis vert. La garde ne
se reproduit pas sur une forme réduite : son témoin est le vrai en-tête.

Une sonde en mémoire (`tests/sonde_marque_nom.rs`, corpus figé de rag3db-garde, `src`, `examples` et
`test` ensemble, 1 725 fichiers) : 27 couples naissent (`main.c` → 16 fonctions de l'API C, `test/c_api`
→ 11), aucun ne part. Le reste des 2 533 de la base du dépôt est dans des dossiers que la sonde n'a pas
pris (`tools`, `third_party`, `extension`). La base repassée le dira, puisque REGLES_DE_L_ANALYSE monte
avec ce lot.

## La mesure sur la base du dépôt (banc, 16 novembre 2026 : deux index neufs de l'arbre 1de1f86d88 sur base vide, sans embarqueur — avant par f10336d3f2, après par 41144f8d22 ; sorties à luciepc, `~/.cache/rag3weaver-build/banc-parite/sorties/codeparsers-stack/`)

- Arêtes vers l'API C (cible sous `src/c_api/`, `visibility = 'declaration'`) : 0 → 30, toutes depuis `test/`.
- CONSUMES des mentionneurs `.c` : 892 → 1 481 (third_party 892 → 1 465, examples 0 → 16).
- Le reste des 2 533 d'ED : sur l'arbre figé, la sonde `probe_unique_definer_misses` montre que la plupart
  des « définisseur unique sans arête » sont des abstentions justes (valeurs d'enum, méthodes std, un
  appel de `src/` vers un fichier de test) ; les vrais manques sont des méthodes héritées, le ticket de
  [la définition liée à l'en-tête](../../codeparsers/tickets/2026-10-25-une-definition-cpp-se-lie-a-sa-declaration-d-en-tete.md).
