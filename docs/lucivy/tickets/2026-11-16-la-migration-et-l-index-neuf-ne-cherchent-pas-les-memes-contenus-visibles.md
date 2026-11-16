# La migration et l'index neuf ne cherchent pas les mêmes contenus visibles

- **État** : ouvert (16 novembre 2026).
- **Gravité** : écart de classement. Une base migrée, cherchée à son rang visible, et un index neuf du même arbre
  ne cherchent pas le même ensemble de versions. Une recherche lexicale peut donc remonter des versions que l'index
  neuf n'a pas, ou en perdre qu'il a. Le banc le voit sur « translate a hop into a query » en bm25 seul.
- **Atteignable en service** : oui, sur toute base migrée vers les générations, tant qu'elle n'est pas rebâtie.
- **Touche rag3weaver** : oui (l'écrivain, la migration, le masque du rang visible), pas lucivy.
- **Famille** : produit
- **Pour** : everything declarative (ED), avec banc
- **Signalé par** : lucivy, au départage du bm25 seul sous 4.3.9 (16 novembre 2026)

## Ce qui se passe

Départage fait par lucivy le 16 novembre 2026 à luciepc, avec `test_compare_two_indexes` de lucivy main 4.3.9.
L'outil de détail a été ajouté en local au test : `LUCIVY_CMP_DETAIL=1`, `LUCIVY_CMP_ONLY_OUT`. Il compare deux
copies reflink de `Lucivy_ScopeContent` :
- B′, l'index neuf (`banc-parite/migration-bprime-20261116`), cherché sans filtre ;
- A, la migration (`banc-parite/migration-lot-A-20261116`), cherchée sous les 61 122 ids visibles que rag3weaver
  passe en `allowed_ids` (ensemble tiré par banc, confirmé par ED).

Les documents se comparent par leur identité, l'empreinte de `content` et de `name` :

| | documents cherchés | identités distinctes | absentes de l'autre côté |
|---|---:|---:|---:|
| B′ (neuf) | 61 087 | 61 013 | **879** |
| A visible (migration) | 61 122 | 61 045 | **911** |

Les `_node_id` de ces documents sont à luciepc, sous `~/.cache/rag3weaver-build/lucivy-bm25-migration/` :
- `contenus-only-A.txt` : 879 offsets de ScopeContent **dans B′** ;
- `contenus-only-B.txt` : 911 offsets de ScopeContent **dans A**.

(Dans l'outil, A est B′ et B est la migration.)

**L'ensemble visible de A** a été tiré par banc, en lecture seule sur une copie de A. Fichier :
`~/.cache/rag3weaver-build/banc-parite/migration-lot-A-20261116-visibles.txt` (61 122 lignes, un offset décimal par
ligne, triés). La requête est celle qu'ED a donnée, celle que rag3weaver joue pour l'`allowed_ids` lexical d'une base
à générations :

```
MATCH (g:_generation) WHERE g.couche = 'backend' RETURN max(g.rang)        -- R = ce maximum + 1
MATCH (t:Scope)-[:ScopeContent_DERIVED_FROM]->(idx:ScopeContent)
WHERE t._couche = 'backend' AND t._depuis <= $R AND (t._jusqu_a IS NULL OR t._jusqu_a > $R) AND t._uuid IS NOT NULL
RETURN OFFSET(id(idx))                                                     -- dédoublonné, trié
```

Sur A, une seule génération (`backend:0`), R = 1, aucune Scope hors de `backend` et aucune fermée après R : la
condition revient à `_jusqu_a IS NULL`. C'est la forme de banc :
`MATCH (d:ScopeContent) WHERE EXISTS { MATCH (d)-[:ScopeContent_DERIVED_FROM]->(s:Scope) WHERE s._jusqu_a IS NULL } RETURN offset(id(d))`.

Lucivy est hors de cause : sur ces copies, ses scores sont égaux à 1,8e-3 relatif près, sous le contrat de 5e-3 de
la 4.3.9. Ses classements sont les mêmes, sauf deux cas :
- les ex æquo stricts (point 2 plus bas) ;
- les documents absents d'un côté.

Sur « parse translate a hop into a query », les rangs 4, 8, 12, 14 et 19 de A sont tous des contenus absents de B′.

## Ce qu'il faudrait

1. **Lire les 911 et les 879 dans leurs bases** (Cypher, sur copie) : le chemin, la ligne, le nom et la longueur
   de chaque version. On saura alors s'il s'agit de versions découpées autrement (un même scope, deux contenus), de
   noms qui diffèrent (le `name` entre dans l'identité), ou de versions que la migration garde ouvertes à tort.
2. **Les ex æquo plus haut que lucivy.** Sur `contains read_text`, `value` et `transaction rollback`, tous les
   documents du top 20 existent des deux côtés. Les divergences ne sont que des permutations entre scores strictement
   égaux :
   - read_text : 4 ex æquo ;
   - value : 3 ;
   - transaction rollback : 2.
   
   Le tri par score puis uuid (07c274b1a8) est dans le master qu'a joué le banc (fe5d81f811). Si ces cas changent
   encore de rang chez le banc, l'uuid d'une même version diffère peut-être entre migration et neuf, ou bien le rang
   RRF de la fusion se prend avant ce tri. C'est à vérifier.

   **ED a nommé la cause (16 novembre 2026)** : sous les générations, l'uuid d'une ligne `Scope` vaut
   `hash(clé, couche, _depuis)` dès que `_depuis` est plus grand que 0. Une même occurrence n'a donc pas le même uuid
   dans A, où elle est réécrite au rang 1, et dans B′, où elle est née au rang 0. Or la projection vers les
   occurrences (`search.rs`) trie par score puis uuid d'occurrence. **Correctif proposé par ED** : départager par une
   identité qui ne dépend ni de la base ni de son histoire. Pour une occurrence, c'est `Scope.key` ; pour une version,
   son uuid de contenu.

## Comment le rejouer

Le script est à luciepc, `~/.cache/rag3weaver-build/lucivy-bm25-migration/cmp-detail.sh`, sous `poste lourd`. Son
journal complet est `cmp-detail.raw`, à côté. Le correctif de l'outil y est en `detail.patch`, et lucivy le propose
dans `test_compare_two_indexes`.
