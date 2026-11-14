# Une recherche sur une base à versions prend des Gio et des dizaines de secondes

> **1b n'atterrit sur aucune vraie base tant que ce ticket n'est pas fermé** (c7, 14 novembre 2026).
> Aucune base de Lucie n'est encore à versions : rien n'est touché aujourd'hui.

- **État** : ouvert (14 novembre 2026).
- **Famille** : produit
- **Gravité** : bloquant pour 1b. Un `search_code` sans filtre sur une base à versions de la taille du dépôt rag3db prend 22 à 24 s avec le dense, et la mémoire résidente monte de plusieurs Gio par appel : le processus a été tué à 33 Gio au sixième appel. Avant les versions, la même recherche prend environ 1 s
- **Atteignable en service** : oui, dès qu'une base monte aux versions (1b)
- **Touche rag3weaver** : oui (`src/dataflow/generic_search_nodes.rs` : `occurrence_mask_options`, `allowed_ids_for` ; `src/read_view.rs` : `resoudre_le_filtre`, `compile_filter_for_vector_in`) ; le moteur pour l'`IN`
- **Pour** : everything declarative
- **Signalé par** : everything declarative, mesures du 14 novembre 2026 à luciepc sous `poste mesure` (page `docs/everything-declarative/14-11-2026/04-le-pas-2a-prime-2-l-ancre-immuable.md`, § 2 bis)

## Ce que c'est

Les mesures ont été faites sur une copie de `code-rag3db-depot` migrée par 1b : 79 662 occurrences
`Scope`, 72 652 versions `ScopeContent`. La référence R est la même base avant la migration.

**Le lexical**, médianes sur 50 appels, sans le dense :

| forme | R | à versions |
|---|---|---|
| sans filtre | 965 ms | 7 032 ms |
| chemin `src/include/` | 247 ms | 2 154 ms |
| langage python | 229 ms | 1 371 ms |

Dans le nœud lexical, sans filtre :
- les racines : 20 ms ;
- le masque des versions : 0,9 s ;
- **la résolution de `_uuid IN [72 652]` en offsets : 5,0 s**.

Le plan de cette résolution est `SCAN_NODE_TABLE ScopeContent` puis
`FILTER LIST_CONTAINS($filter_p0)`. Le même ensemble par `UNWIND $ids AS u MATCH (n {_uuid: u})`
se résout en **112,6 ms**. lucivy ne prend, lui, que 6 à 10 ms de filtre par requête.

**Le dense**, avec l'embarqueur connecté, sans filtre :

| appel | durée | RSS |
|---|---|---|
| 1 | 23,1 s | 16,1 Gio (2,8 avant) |
| 2 | 21,8 s | 19,5 Gio |
| 3 | 23,1 s | 23,4 Gio |
| 4 | 22,9 s | 27,7 Gio |
| 5 | 24,6 s | 22,6 Gio |
| 6 | 24,4 s | 29,3 Gio, tué à 33,4 |

**Le moteur seul ne retient rien** : l'étape `IN` rejouée 30 fois laisse le RSS à +20 Mio. La
mémoire vient du chemin de la recherche, et vraisemblablement du dense, qui refait le masque et
filtre les morceaux par `p._uuid IN [72 652]` (`compile_filter_for_vector_in`).

## La cause de la mémoire, départagée (14 novembre 2026, 20 h 34)

- **La base d'avant les versions ne fuit pas** : 10 recherches denses, environ +5 Mio par appel
  après le premier, 284 ms en médiane. Les hôtes de Lucie d'aujourd'hui ne sont pas touchés.
- **Sur la base à versions, c'est la projection de graphe du dense filtré.**
  `PROJECT_GRAPH_CYPHER` sur un Cypher où le filtre des 72 652 versions est écrit en clair (2,9 Mo),
  puis `DROP_PROJECTED_GRAPH`, retient +3,58 Gio par tour, sans aucune requête vectorielle. C'est une
  fuite du moteur : ticket `2026-11-14-une-projection-de-graphe-retient-sa-memoire-apres-son-retrait.md`,
  pour le cœur.
- **Côté rag3weaver**, deux défauts de forme :
  - le filtre est écrit en clair dans le Cypher projeté ;
  - un graphe est projeté puis retiré à chaque recherche.

  La forme à viser est un graphe projeté posé une fois par (couche, rang), à la publication, et
  cité par son nom. C'est la vue de 2a′-2.

## Ce qu'on pourrait faire

- **Le cas sans filtre** : le court-circuit d'arbre principal (lot de 1a). Sans filtre ni vue, il
  n'y a pas de masque, seulement la remontée des résultats retenus. Il supprime l'`IN` du cas commun.
- **L'`IN` sur `_uuid`** : rag3weaver l'écrit en `UNWIND` et accès par clé (×44) dans
  `resoudre_le_filtre`. Un ticket moteur demande que le planificateur le fasse de lui-même.
- **Le cas filtré, et sous vue** : l'ensemble admis posé une fois par (couche, rang), en offsets
  (2a′-2).
- **La mémoire du dense** : départager étage par étage (le dense seul, avec le RSS par appel), puis
  fermer la cause. Le court-circuit sans filtre ne suffit pas, puisque la forme filtrée garde le
  même chemin.

## Comment le voir

`tests/sonde_recherche_sans_filtre.rs` et `tests/sonde_noeud_lexical.rs`, sur une copie d'une base à
versions (`RAG3WEAVER_SONDE_BASE`, `RAG3WEAVER_SONDE_MANIFESTE`), sous `POSTE_MEM_MAX=32G`.
