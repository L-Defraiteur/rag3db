# Une projection de graphe retient sa mémoire après son retrait

- **État** : ouvert (14 novembre 2026).
- **Famille** : moteur
- **Gravité** : fuite. Chaque `PROJECT_GRAPH_CYPHER` suivi de `DROP_PROJECTED_GRAPH` laisse environ 3,6 Gio que le processus ne rend jamais ; le processus est tué par le noyau en quelques recherches
- **Atteignable en service** : oui, toute recherche dense filtrée. rag3weaver projette un graphe à chaque recherche vectorielle qui porte un filtre (`Rag3dbSearchBackend::vector_search_filtered`). Sur une base à versions, c'est chaque recherche, puisque le masque des versions est un filtre
- **Touche rag3weaver** : non pour la fuite. Oui pour la forme de l'appel : une liste écrite en clair dans le Cypher projeté, et une projection par requête (voir le ticket de la recherche à versions)
- **Pour** : le cœur
- **Signalé par** : everything declarative, 14 novembre 2026, luciepc, `poste mesure` seul, lib 0c41f9fe6 Release, sonde `extension/rag3weaver/tests/sonde_vecteur_filtre.rs` (branche generations-2a-prime-2, 4efc6d1ff)

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

## Ce qu'il reste à trancher

Deux variantes, à jouer en C++ :
- (a) la même projection avec un Cypher court qui rend autant de nœuds (tous les morceaux, sans
  `IN`). Si elle fuit, c'est le graphe projeté que `DROP_PROJECTED_GRAPH` ne libère pas ;
- (b) le même Cypher de 2,9 Mo joué seul, hors projection. S'il fuit, c'est l'analyse d'une grande
  requête (son texte, son AST, un cache de plans) qui reste.

## Comment le voir

`RAG3WEAVER_SONDE_BASE` et `RAG3WEAVER_SONDE_MANIFESTE` sur une copie d'une base à versions,
`RAG3WEAVER_SONDE_VARIANTES=V3,V2,V1`, sous `POSTE_MEM_MAX=32G` : les lignes `V2 tour`.
