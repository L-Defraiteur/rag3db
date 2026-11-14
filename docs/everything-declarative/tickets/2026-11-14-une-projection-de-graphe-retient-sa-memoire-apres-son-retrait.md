# Analyser un Cypher à grande liste littérale prend 1 200 fois sa taille en mémoire

> **Reformulé le 14 novembre 2026, 20 h 50**, au verdict du cœur. Le nom du fichier garde le premier
> constat : « une projection de graphe retient sa mémoire après son retrait ». Ce n'est pas une
> rétention : la mémoire est rendue. Le défaut est le pic d'analyse.

- **État** : ouvert (14 novembre 2026).
- **Famille** : moteur
- **Gravité** : pic de mémoire. Analyser et exécuter **une fois** un Cypher de 2,9 Mo, dont 72 652
  littéraux de liste, prend environ 3,6 Gio, soit environ 50 Kio par élément et 1 200 fois la taille
  du texte. La mémoire est rendue ensuite : `malloc_trim(0)` fait retomber le RSS. Mais d'un fil à
  l'autre, les arènes de glibc la gardent, et un processus qui recherche sur plusieurs fils (tokio)
  la voit monter linéairement : +3,58 Gio par recherche, tué à 33 Gio
- **Atteignable en service** : oui, toute requête qui écrit une grande liste en clair. Côté
  rag3weaver, c'est la projection du dense filtré sur une base à versions, retirée par la vue de
  2a′-2 (`docs/everything-declarative/14-11-2026/05-la-vue-publiee-par-couche-et-rang.md`)
- **Touche rag3weaver** : non pour le défaut. Oui pour la forme de l'appel (la vue de 2a′-2 et la
  règle « jamais une grande liste en clair dans un Cypher »)
- **Pour** : le cœur (la phase qui fait le pic : l'analyse, la liaison ou le plan) ; une piste
  pour G : `MALLOC_ARENA_MAX` sur l'hôte, à mesurer
- **Signalé par** : everything declarative, 14 novembre 2026, luciepc, `poste mesure` seul, lib
  0c41f9fe6 Release, sonde `extension/rag3weaver/tests/sonde_vecteur_filtre.rs` (branche
  generations-2a-prime-2, 4efc6d1ff) ; le verdict C++ du cœur, cas projection-rend-sa-memoire
  (3d98a391b)

## Le verdict du cœur (14 novembre 2026, 20 h 45, chaque variante dans son processus, six tours)

| variante | RSS brut | après `malloc_trim(0)` |
|---|---|---|
| (p) la liste en paramètre | 155 Mio, plat | |
| (a) la projection courte | 67 Mio, plat | |
| (b) la requête longue seule, hors projection | 3 690 Mio à chaque tour | 60 à 70 Mio |
| (c) la séquence de rag3weaver telle quelle | 3 685 à 3 699 Mio | 78 à 91 Mio |

Le pic vient de la requête longue, pas de la projection. Rien ne grossit d'un tour à l'autre dans un
même fil. La pente vue côté rag3weaver vient d'un fil neuf à chaque recherche, avec une arène
glibc par fil.

## Ce que c'est

Les tables : `ScopeContent` (72 652 lignes, clé `_uuid` STRING) et `ScopeContent_Chunk` (119 887
lignes, index HNSW `ScopeContent_Chunk_vec__granite_278m`), reliées par `ScopeContent_CHUNKED_FROM`
du morceau vers son parent.

La séquence, rejouée six fois :

```
CALL DROP_PROJECTED_GRAPH('_vf_sonde', skip_if_not_exists := true)
CALL PROJECT_GRAPH_CYPHER('_vf_sonde', 'MATCH (n:`ScopeContent_Chunk`) MATCH (n)-[:`ScopeContent_CHUNKED_FROM`]->(p:`ScopeContent`) WHERE p.`_uuid` IN [<72 652 chaînes en clair>] RETURN n')
CALL DROP_PROJECTED_GRAPH('_vf_sonde', skip_if_not_exists := true)
```

Le Cypher projeté fait 2 906 200 octets. Aucune requête vectorielle n'est jouée entre les deux.

| tour | durée | RSS du processus |
|---|---|---|
| départ | | 3 009 Mio |
| 1 | 1 989 ms | 6 591 Mio |
| 2 | 1 910 ms | 10 179 Mio |
| 3 | 1 954 ms | 13 758 Mio |
| 4 | 1 945 ms | 17 339 Mio |
| 5 | 1 949 ms | 20 919 Mio |
| 6 | 1 940 ms | 24 499 Mio |

Le RSS monte de +3 580 Mio par tour, exactement linéaire.

**Les témoins, dans le même processus** :
- `QUERY_VECTOR_INDEX` sans projection, six tours : 6 à 8 ms, RSS plat ;
- le même `IN` de 72 652 clés, avec la liste **en paramètre**, dans une requête ordinaire
  (`MATCH (n:ScopeContent) WHERE n._uuid IN $p RETURN OFFSET(id(n))`), rejoué 30 fois : +20 Mio,
  plat.

**Une projection courte ne fuit pas** (même soir, 20 h 40, base d'avant les versions) : 20 projections
par requête, avec un Cypher d'environ 150 octets
(`MATCH (n:Scope_Chunk) MATCH (n)-[:Scope_CHUNKED_FROM]->(p:Scope) WHERE starts_with(p.file_path, "src/include/") RETURN n`),
chacune suivie du `DROP`. Le RSS est plat, environ +5 Mio par appel sur 10 appels. **La rétention
tient donc à la taille du Cypher projeté**, et non à la projection elle-même. C'est la variante (b)
ci-dessous plutôt que la (a). Les hôtes d'aujourd'hui, sur des bases d'avant les versions, ne sont
pas touchés, même par une recherche dense filtrée.

## Ce qu'il restait à trancher (fait par le cœur, ci-dessus)

Deux variantes, à jouer en C++ :
- (a) la même projection avec un Cypher court qui rend autant de nœuds (tous les morceaux, sans
  `IN`). Si elle fuit, c'est le graphe projeté que `DROP_PROJECTED_GRAPH` ne libère pas ;
- (b) le même Cypher de 2,9 Mo joué seul, hors projection. S'il fuit, c'est l'analyse d'une grande
  requête (son texte, son AST, un cache de plans) qui reste.

## Comment le voir

`RAG3WEAVER_SONDE_BASE` et `RAG3WEAVER_SONDE_MANIFESTE` sur une copie d'une base à versions,
`RAG3WEAVER_SONDE_VARIANTES=V3,V2,V1`, sous `POSTE_MEM_MAX=32G` : les lignes `V2 tour`.
