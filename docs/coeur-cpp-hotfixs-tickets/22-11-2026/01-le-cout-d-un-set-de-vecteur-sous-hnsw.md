# Le coût d'un SET de vecteur sous un index HNSW : l'instruction coûte, pas le graphe

Les 21 et 22 novembre 2026, pour ED, au mot de c7. ED mesurait 36,6 ms par `SET` de vecteur à l'indexation d'une vue
(161 873 morceaux, `ScopeContent_Chunk_vec__granite_278m`, mu 30, ml 60, efc 200, cosinus), soit 404 s pour 412
fichiers. Deux questions : une insertion groupée coûte-t-elle moins que N insertions, et que coûte `efc` contre le
rappel ?

## La mesure

`VectorIndexBridgeMeasure.InsertionCostByBatchAndEfc` (`concurrence_test`, branche `hnsw-insertion-cost-measure`,
9ba214e817, sur master 05301576ba), en Release à luciepc, sous `poste cpu` (sous charge : les rapports entre variantes
d'un même tour comptent, les valeurs absolues moins).

- **Le corpus** : les 127 198 vecteurs de `Scope_Chunk` de code-lourd (granite-278m, 768 dimensions), en cosinus.
  Les 200 derniers sont des requêtes, hors de la table ; sur les 126 998 autres, les 4 096 dernières lignes naissent
  sans vecteur, et l'index est bâti sur le reste.
- **Les variantes** : leurs vecteurs posés par `SET n.vec = n.staged`, par lots de 1 (sur 256 lignes), 64, 512 et
  4 096 lignes par instruction, pour efc 200, 100 et 50 ; et la reconstruction (DROP de l'index, SET sans index,
  CREATE).
- **Le rappel** : chaque variante, rappel@10 de ses 200 requêtes contre une recherche exhaustive.

| efc | 1 ligne / SET | lots de 64 | lots de 512 | lots de 4 096 | CREATE (122 902 lignes) | rappel@10 |
|---|---|---|---|---|---|---|
| 200 | 81,4 ms | 9,2 ms | 7,6 ms | 8,9 ms | 123 à 127 s | 0,997 à 0,9985 |
| 100 | 73,7 ms | 8,0 ms | 7,2 ms | 8,6 ms | 75 à 87 s | 0,9975 à 0,9985 |
| 50 | 79,0 ms | 9,6 ms | 7,2 ms | 7,9 ms | 55 à 57 s | 0,997 à 0,998 |

Reconstruire pour 4 096 vecteurs neufs (DROP, SET sans index en 0,3 s, CREATE) coûte 34,3 ms par vecteur neuf à efc
200, 20,1 ms à efc 100, et 14,1 ms à efc 50, avec un rappel de 0,9975 à 0,9985.

## Ce que ça dit

1. **Une instruction par vecteur coûte dix fois plus qu'un lot.** De 74 à 81 ms par vecteur à l'unité, contre 7 à
   9,6 ms dès 64 lignes par instruction. Au-delà de 64 lignes, la taille du lot ne change plus rien. L'essentiel du
   prix d'un `SET` isolé tient à l'instruction et à sa transaction : la validation, et l'état d'insertion de l'index
   (`HNSWInsertState`), dimensionné à la table à chaque fois. Ce n'est pas la recherche dans le graphe. Les 36,6 ms
   d'ED ont la forme d'un SET par instruction, à charge moindre.
2. **`efc` ne change rien au coût d'un SET.** La recherche de l'insertion sur disque prend son `ef` dans
   `QueryHNSWConfig{}` (`hnsw_index.cpp:613`, `:626`), la configuration de requête par défaut, et non dans l'`efc` de
   l'index : `efc` ne sert qu'au CREATE. Là, il compte : 124 s à efc 200, 56 s à efc 50, pour le même rappel sur ce
   corpus (0,997 à 0,9985).
3. **La reconstruction ne vaut que pour de très gros lots.** À ces prix, elle bat les SET par lots au-delà d'environ
   7 000 vecteurs neufs à efc 50 (57 s ÷ 8 ms), ou 17 500 à efc 200.
4. **Le rappel ne bouge pas** entre les variantes : de 0,997 à 0,9985 partout, l'écart d'une passe à l'autre.

## Ce que je recommande

- **Côté rag3weaver (ED)** : poser les vecteurs d'une vue par lots d'au moins 64 lignes par instruction (`UNWIND`
  d'une liste de paires, ou un `MATCH … WHERE id IN …` posé depuis une table d'attente), dans une seule
  transaction. On peut en attendre environ un facteur dix sur la part HNSW. Le moteur n'a rien à changer pour cela :
  une instruction qui pose plusieurs vecteurs existe déjà.
- **Côté moteur, deux lots possibles, à décider** :
  - **(a) `efc` pour l'insertion** : la recherche d'une insertion prendrait l'`efc` de l'index au lieu de l'`efs` par
    défaut. Cela rend cohérent un paramètre qui ne sert aujourd'hui qu'au CREATE. Le gain est à mesurer : la part de
    la recherche dans les 7 à 9 ms n'est pas isolée ici.
  - **(b) le CREATE à efc 50 par défaut**, ou pour les bases de code : deux fois plus rapide, au même rappel sur
    code-lourd. À juger sur d'autres corpus, avec le banc, avant d'en faire un défaut.
- **Ce que je n'ai pas mesuré** : le profil des 70 ms d'un SET isolé (la validation, le WAL, l'état d'insertion).
  Si rag3weaver ne peut pas grouper ses SET, c'est la prochaine mesure.
