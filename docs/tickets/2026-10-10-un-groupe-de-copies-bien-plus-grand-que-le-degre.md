# Un groupe de copies bien plus grand que le degré perd des lignes dans l'index vectoriel

- **État** : ouvert, confort.
- **Gravité** : réponse fausse. Des lignes ne sont plus joignables par la recherche exhaustive.
  Aucun plantage.
- **Atteignable en service** : seulement avec des centaines de lignes du même vecteur. Le plus
  grand groupe d'un corpus réel mesuré est de 49 copies, et il est retrouvé.
- **Touche rag3weaver** : non. L'arbre principal n'écrit plus de vecteur nul (10 octobre), et ce
  sont les vecteurs nuls du MockEmbedder qui avaient fait plusieurs centaines de copies en août.

## Ce que c'est

1 402 lignes portent le même vecteur, 23 fois le degré de l'index (ml = 60). Une partie de ces
lignes n'est plus atteinte par la recherche exhaustive (k = efs = 1 402).

## Recette minimale

```cypher
CREATE NODE TABLE T(id INT64, v FLOAT[384], PRIMARY KEY(id));
CALL CREATE_VECTOR_INDEX('T', 'idx', 'v', metric := 'cosine');   -- ou après les lignes
-- 1 402 lignes du même vecteur (nul, ou non nul), une à une ou par COPY
CALL QUERY_VECTOR_INDEX('T', 'idx', <le vecteur>, 1402, efs := 1402) RETURN node.id;
```

Elle vient de rag3db-9c (10 octobre) : une note d'`embedder.rs` du 25 août 2026 disait que l'index
plantait sur quelques centaines de vecteurs identiques. **Ce plantage ne se reproduit plus** :
ni signal ni erreur, et la recherche rend ses 10 résultats. Une norme nulle ne divise pas par
zéro, puisque le cosinus de simsimd rend 0 quand les deux normes sont nulles.

## Témoin

`vector_index_update_test.cpp`, `Cases/SameVectorForEveryRow.NeitherCrashesNorLosesRows/*`, en
huit cas joués dans un fils : vecteur nul (Zero) ou même vecteur non nul (Same), index posé avant
les lignes (IndexFirst) ou après (RowsFirst), lignes une à une (LineByLine) ou par COPY.

| cas | master avant l'élagage classique | anneau (non retenu) | au plus près, places reprises du plus proche (non retenu) | élagage retenu (places reprises du plus lointain) |
|---|---|---|---|---|
| zéro, index avant, une à une | 1 193 | 1 293 | 1 286 à 1 290 | 1 083 |
| zéro, index avant, COPY | 1 298 | 1 402 | 1 402, ou 1 222 une fois sur trois | 1 402 |
| zéro, lignes avant, une à une | 1 298 | 1 402 | 1 222 | 1 222 |
| zéro, lignes avant, COPY | 1 298 | 1 402 | 1 222 | 1 222 |
| même, index avant, une à une | 1 190 | 366 | 1 286 à 1 290 | 1 083 |
| même, index avant, COPY | 1 298 | 1 402 | 1 402 | 1 402 |
| même, lignes avant, une à une | 1 298 | 1 402 | 1 222 | 1 222 |
| même, lignes avant, COPY | 1 298 | 1 402 | 1 222 | 1 222 |

**Pourquoi l'élagage retenu fait moins que l'essai d'avant sur « index avant, une à une »**
(1 083 contre 1 286 ; master 1 190) : non élucidé, mais une lecture le rend probablement
fortuit. Dans ce témoin, toutes les lignes sont des copies les unes des autres. Les candidats
d'une liste sont donc tous des copies : la borne en garde la moitié du degré, et il ne reste aucun
écarté non copie à reprendre. Les deux variantes ne diffèrent que par l'ordre de ce remplissage,
qui n'a ici rien à remplir : elles font les mêmes choix. L'écart viendrait alors du tirage des
niveaux du graphe, qui varie d'une passe à l'autre ; il est à confirmer par des répétitions.

Rouges connus : les six cas toujours rouges sont dans `known_red.txt`, le cas instable
(ZeroIndexFirstCopy, rouge une fois sur trois au plus près ; vert sur la mesure de l'élagage
retenu) dans `probabilistic.txt`. Les deux cas « une à une, index avant » sont dans
`long.txt`.

## Cause

L'élagage retenu (`selectNeighbours`, `hnsw_index.cpp`) garde au plus la moitié du degré en
copies du vecteur du nœud, les plus proches en décalage. Les copies au-delà sont jetées. Dans un
groupe de 1 402, chaque liste ne voit qu'une partie des copies : celles que la recherche
d'insertion a trouvées. Qui est visé dépend donc de la manière dont ces sous-ensembles se
recouvrent.

- **Les copies choisies en anneau** (essayé le 10 octobre) : toute ligne insérée pointait vers
  les premières lignes de la table, et le milieu n'était plus visé. D'où les 366 sur 1 402 en
  insertion une à une.
- **Le bâti en mémoire, sur une table pleine, perd 180 lignes avec les copies au plus près**,
  alors que l'anneau les gardait toutes : **non élucidé**. Mon hypothèse, non vérifiée : le bâti
  en mémoire est parallèle, et les candidats d'un nœud ne sont qu'un sous-ensemble des copies.
  Au plus près, un nœud ne garde que ses voisins de décalage présents parmi ces candidats, et un
  nœud qu'aucun de ses voisins de décalage n'a eu pour candidat n'est plus visé. L'anneau, qui
  vise les successeurs, couvrait toute la table par construction. Une autre piste : à distances
  toutes égales, la recherche exhaustive peut s'arrêter tôt sur les égalités. Le compte exact et
  stable (1 222 sur trois répétitions) va plutôt vers un effet du graphe que de la recherche.

## Cas voisins, pathologiques, notés ici

- **-0.0 et 0.0** : deux vecteurs qui ne diffèrent que par le signe d'un zéro ne sont pas des
  copies (leurs octets diffèrent), mais ils sont à distance 0. L'inégalité stricte les garde tous
  les deux, sans borne. Une liste peut s'en remplir.
- **NaN** : deux vecteurs NaN identiques sont des copies, donc bornés. Leurs distances sont NaN,
  et le tri par distance n'est déjà plus un ordre strict faible sur master.

## Pour le fermer

- Un choix des copies qui couvre le groupe quel que soit l'ordre d'arrivée : par exemple une
  moitié au plus près et une moitié en anneau, ou un tirage stable par une empreinte du décalage.
  Chaque essai se mesure sur ce témoin et sur les vrais vecteurs (`RealVectorsReachability`),
  avec l'extension prouvée (`~/.cache/rag3db-banc-notes/batir-colonnes.sh`).
- Ou décider que ce cas n'a pas à être servi, puisque le produit n'écrit plus de vecteur nul et
  que les corpus réels restent sous la borne.
