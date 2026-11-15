# Un `IN` sur la clé primaire balaie toute la table au lieu de prendre la clé

- **État** : ouvert (14 novembre 2026). Contournement rag3weaver sur master en 9fd946ec3 : 5 008 → 113,6 ms (sonde `sonde_in_par_cle`, recherche, 15 novembre 2026, lib 373b5477b, mêmes 72 652 offsets). Les deux correctifs du cœur sont sur master (15 novembre 2026, 4 h 2x) : (B) l'IN haché, e5e626803, sans N² sur une liste constante ; (A) 7897f2453, où `n.pk IN <liste littérale ou paramètre du type de la clé>` sur une seule table se planifie en PRIMARY_KEY_SCAN, sans balayage ni FILTER (10 clés sur 200 000 lignes : 10,4 → 0,16 ms ; 40 000 sur 40 000 : 1,12 × la forme UNWIND ; l'ordre de la liste, chaque nœud une fois, NULL sans résultat). Reste à fermer : la sonde `sonde_in_par_cle` et l'EXPLAIN de `MATCH (n:ScopeContent) WHERE n._uuid IN $filter_p0 RETURN OFFSET(id(n))` sur la lib qui portera 7897f2453, pas encore posée.
- **Famille** : moteur
- **Gravité** : lenteur. ×44 sur une résolution de 72 652 clés : 5 008 ms au lieu de 113 ms
- **Atteignable en service** : oui. C'est le filtre `_uuid IN [...]` que rag3weaver passe à chaque recherche sur une base à versions (le masque des versions), et toute requête d'utilisateur qui filtre sur une liste d'identifiants
- **Touche rag3weaver** : non pour le défaut (le planificateur). Oui pour le contournement : `resoudre_le_filtre` peut écrire l'`IN` en `UNWIND` et accès par clé, un lot confié à recherche le 14 novembre 2026
- **Pour** : le cœur (le planificateur)
- **Signalé par** : everything declarative, mesure du 14 novembre 2026 à luciepc sous `poste mesure`, lib 0c41f9fe6, sur une copie de `code-rag3db-depot` migrée par 1b (`tests/sonde_noeud_lexical.rs`, branche generations-2a-prime-2) ; même famille que le ticket du planificateur sur les sauts (`2026-11-14-un-predicat-sur-les-noeuds-et-l-arete-d-un-saut-fait-balayer-toute-la-table.md`)

## Ce que c'est

La table : `ScopeContent`, 72 652 lignes, clé primaire `_uuid` (STRING). Le paramètre : une liste
de 72 652 chaînes, toutes des clés de la table.

**La forme `IN`**, telle que rag3weaver l'écrit :

```
MATCH (n:`ScopeContent`) WHERE n.`_uuid` IN $filter_p0 RETURN OFFSET(id(n))
```

Son plan (EXPLAIN) :

```
RESULT_COLLECTOR[3]   OFFSET(n._ID)
PROJECTION[2]         OFFSET(n._ID)
FILTER[1]             LIST_CONTAINS($filter_p0)
SCAN_NODE_TABLE[0]    Tables: ScopeContent, Properties: n._uuid
```

La table est balayée en entier, et chaque ligne est cherchée dans la liste.

**La forme par clé** :

```
UNWIND $ids AS u MATCH (n:`ScopeContent` {_uuid: u}) RETURN OFFSET(id(n))
```

Elle rend les mêmes 72 652 offsets.

| forme | médiane | max |
|---|---|---|
| `IN` (balayage puis LIST_CONTAINS) | **5 008,3 ms** | 5 025,5 ms |
| `UNWIND` et accès par clé | **112,6 ms** | 116,4 ms |

5 tours après 1 tour de chauffe. La mémoire ne bouge pas : la forme `IN` rejouée 30 fois laisse le
RSS du processus à +20 Mio.

## Ce qu'on attend du planificateur

Un `IN` de liste sur la clé primaire, comparé au paramètre lui-même, devrait se planifier comme la
forme par clé : un parcours de la liste et un accès à l'index de la clé primaire par élément. Ou,
pour une liste plus grande que la table, une jointure par hachage. Pas un balayage avec un
`LIST_CONTAINS` par ligne.

## Comment le voir

Une table de N lignes à clé STRING, et une liste de N clés en paramètre : les deux requêtes
ci-dessus. Le cas se rejoue en C++ sans rag3weaver.
