# Une ligne très loin des autres est injoignable dans un index bâti d'un coup

- **État** : ouvert — **bloque la stèle, provisoirement** (orchestration, 5 octobre 2026) : tant qu'une mesure sur un corpus réel n'a pas dit si c'est la rareté d'un jeu fait dur exprès ou un taux réel. Une ligne indexée invisible durablement n'est pas un rappel approché
- **Gravité** : réponse fausse
- **Atteignable en service** : oui, sans aucune mise à jour
- **Touche rag3weaver** : non mesuré. La forme du 5 octobre (plus bas) est celle de son chargement : index posé d'avance, morceaux par `COPY` successifs. Mesure demandée à l'arbre principal sur un corpus réel (taux de morceaux introuvables par leur propre vecteur)
- **Ouvert le** : 4 octobre 2026, session du banc (au journal des chantiers §6 depuis le 3 octobre)
- **Pour** : cœur C++ (extension vector)

## Ce que c'est

Dans un nuage serré de points (toutes les coordonnées sous 31), une ligne isolée très loin
des autres, à `[9000, 2, 3, 4]`, puis un index bâti d'un coup après le remplissage : la
ligne, ou d'autres, manquent à une recherche exhaustive environ un essai sur trois. Avec un
nuage plus étalé (la dernière coordonnée jusqu'à 499), c'est vert. Le défaut est encore
rouge sur master `93f4e81e8`, après `1ea49837f` qui borne autrement le parcours de la
construction (9 essais sur 30 le 4 octobre au soir).

## Recette minimale

```cypher
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, vec FLOAT[4]);
UNWIND range(0, 499) AS i CREATE (n:Doc {id: i}) SET n.vec = [i % 17, i % 23, i % 29, i % 31];
MATCH (n:Doc) WHERE n.id = 250 SET n.vec = [9000, 2, 3, 4];
CALL CREATE_VECTOR_INDEX('Doc', 'doc_index', 'vec', metric := 'l2');
-- recherche exhaustive (k = efs = 500) : moins de 500 lignes rendues, un essai sur trois
```

## Témoin

`test/transaction/concurrence/vector_index_update_test.cpp` —
`FarRowFromATightCloudInAnIndexBuiltAtOnce` (rouge connu, 30 essais, étiquette
`vector-index-exact`) ; le garde-fou vert `FarRowInAnIndexBuiltAtOnce` (nuage étalé).

## Cause

Inconnue. Parent probable du défaut de l'index bâti sur cent lignes qui perd le nœud 13
(`MinimalReproduction.HnswBuiltOnTheFirstHundredRowsLosesANode`). Là, `ml := 20, mu := 10`
ne fait manquer aucun nœud, ce qui désigne l'élagage des voisins à la construction.

## Correctif de l'amont (à lire, ne pas copier)

Aucun trouvé.

## Pour le fermer

Le témoin passe au vert sur ses 30 essais.

## La même famille par des COPY successifs (5 octobre 2026)

D'abord lu comme un dégât de l'annulation d'un `COPY` (ticket
`2026-10-05-annulation-d-un-copy-abime-l-index-vectoriel.md`, requalifié) ; ce n'est pas
l'annulation. Une ligne peut être injoignable **dès sa construction par des `COPY`
successifs dans une table déjà indexée**, sans aucune annulation ni mise à jour.

Recette (la forme du banc, `ProductReloadRecovery`) : une table `Chunk(id, emb FLOAT[64])` ;
l'index posé sur la table vide (`mu := 30, ml := 60, pu := 0.05, metric := 'cosine',
alpha := 1.1, efc := 200`) ; quatre fois `BEGIN`, `COPY` de 32 lignes, `COMMIT`, sans point
de reprise ; puis, pour chacune des 128 lignes, la recherche de son vecteur exact
(`k = 3, efs := 200`).

Exécuté par la session cœur C++ dans un seul processus, huit passes, sur `35d09c466` :
- quatre passes : la ligne 15 ne sort pas (« row 15 gives 87 ») dès la fin des quatre
  `COPY` validés ; le même manque, lui seul, se retrouve identique après trois `COPY`
  annulés, après un point de reprise, après une réouverture ;
- quatre passes : aucun manque, à aucune étape.
L'annulation rend donc exactement l'état d'avant, dans les huit passes. Dans la transaction
des `COPY` (avant le `ROLLBACK`), une à trois lignes anciennes manquent ; l'annulation les
rend.

Ce que ça dit : `efs = 200` sur 128 lignes visite tout ce qui est joignable ; une ligne
absente pour son propre vecteur n'a plus d'arête entrante. L'élagage des voisins à
l'insertion (`OnDiskHNSWIndex::shrinkForNode`) garde les meilleurs voisins d'une ligne et
jette les autres sans vérifier que la ligne jetée reste atteinte par quelqu'un. La garantie
« toutes joignables » n'existe que pour la mise à jour et la suppression
(`keepNodeReachable`, `c8fdaf196`). Le « une fois sur deux » vient du tirage des lignes de
la couche haute (`pu`). Les vecteurs de ce jeu sont très proches les uns des autres, faits
durs exprès ; la fréquence sur des vecteurs réels n'est pas mesurée.

Pourquoi le banc l'avait d'abord lu comme l'annulation : sa fin « mort » rejoue le journal,
donc rebâtit le graphe par le chemin de l'insertion ordinaire ; ses fins avec annulation
passent par un point de reprise et relisent le graphe tel que le `COPY` l'a bâti. **Confirmé
par le banc** avec une quatrième fin, sans aucune annulation — quatre `COPY` validés, une
fermeture propre, une réouverture : rouge 10 passes sur 10, toujours la ligne 15, qui rend 87.

Avec un seul fil (`CALL threads=1` dans la session du chargement, désormais dans le témoin
du banc), sur 30 passes :

| Fin | Rouge | La ligne fautive |
|---|---|---|
| Mort après les `COPY` validés (le rejeu rebâtit le graphe) | 0 sur 30 | — |
| Annulation, point de reprise, un `COPY`, mort | 29 sur 30 | toujours 15, qui rend 234 |
| Annulation, fermeture propre | 29 sur 30 | toujours 15, qui rend 87 |

Le hasard qui reste est la graine de l'index (`RandomEngine` amorcé par
`std::random_device`, sans réglage). Une graine réglable rendrait ces cas déterministes —
utile au témoin d'une passe de rattrapage.

Témoin de diagnostic, hors dépôt parce que rouge une fois sur deux par nature :
`extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/annexes/vector_index_rollback_test.avec-temoin-des-lignes-injoignables.cpp`
(classe `VectorIndexRollbackOfUncheckpointedRowsTest`). Au banc :
`Ends/ProductReloadRecovery.VectorsSetAfterRecoveryAreFound` (dans `probabilistic.txt`).

## Ce que font les index établis à l'insertion (lu le 5 octobre 2026)

Lecture des sources par un agent, branches principales du jour (hnswlib `ca426729`, pgvector
`f37c13f6`, Qdrant `6ab21cac`) ; les numéros de ligne valent pour ces commits.

| | hnswlib | pgvector | Qdrant |
|---|---|---|---|
| Une arête entrante garantie à l'élagage | non (`mutuallyConnectNewElement`, `hnswalg.h`) | non (`HnswUpdateConnection`, `hnswutils.c`) | non (`connect_with_heuristic`, `links_container.rs`) |
| Les voisins écartés comblent la liste (« keepPrunedConnections », algorithme 4 de l'article) | absent | présent, toujours actif (`SelectNeighbors`, l. 1154-1156) : au plus une arête perdue par voisin mis à jour | absent |
| Réparation après coup des orphelins de l'élagage | aucune ; `checkIntegrity()` n'est qu'un assert de test | aucune ; `VACUUM` ne répare que les trous des suppressions | aucune ; le « healer » ne soigne que les voisins de points supprimés ; `subgraph_connectivity` mesure sans réparer |
| La parade pratique | reconstruire | la perte bornée ci-dessus, `VACUUM`, `REINDEX` | recherche exacte sous un seuil de taille (`indexing_threshold_kb`, `full_scan_threshold_kb`), graphe rebâti à l'optimisation de segment, début du bâti sur un seul fil |

Donc aucune des trois ne garantit la joignabilité à l'insertion : la parité avec Qdrant
n'exige pas cette garantie. Ce qu'elles ont et que nous n'avons pas : pgvector borne la
perte (une arête par élagage, liste toujours pleine) ; Qdrant ne passe pas par le graphe pour
les petits ensembles.

Les chiffres trouvés, lus par résumé de page et non dans du code : à la construction pure la
probabilité est dite « extrêmement faible » (Xiao et al., arXiv 2407.07871), mais une issue
hnswlib ouverte montre des degrés entrants nuls dès la construction (#586) ; avec des
suppressions et réinsertions répétées, 2 à 4 % de points injoignables ; chez Qdrant, sur
LAION 2M, environ 0,3 % d'injoignables restent après leur correctif des doublons exacts
(PR #10239). Non vérifié par nous.

Une piste que cette lecture ouvre, non décidée : combler la liste avec les voisins écartés,
comme pgvector — c'est dans l'élagage lui-même, sans recherche de plus.

## La décision (orchestration, 5 octobre 2026)

Dans cet ordre : (c) mesurer d'abord, sur un corpus réel, le taux de lignes introuvables par
leur propre vecteur (arbre principal) ; puis (b), si le taux n'est pas nul, une passe de
rattrapage hors du chemin chaud — en fin de `COPY` ou au point de reprise, chercher les
lignes sans arête entrante et les rattacher ; (a), la garantie à l'insertion même
(`keepNodeReachable` après chaque élagage), seulement si (b) ne suffit pas. Rien n'est codé
avant la mesure.

## La mesure sur un corpus réel (arbre principal, 5 octobre 2026)

Le corpus est `src/` de rag3weaver : 138 fichiers, 12 396 morceaux (12 233 Scope_Chunk, 138
File_Chunk, 25 Library_Chunk). Les vecteurs sont ceux de granite-278m, par le service
distant, en cosine, sur le moteur `35d09c466`. Pour chaque morceau, on cherche par son propre
vecteur avec k = 10. Le morceau est trouvé s'il est parmi les dix, ou si un vecteur identique
au sien sort en tête. Le témoin est `extension/rag3weaver/tests/e2e_mesure_joignabilite.rs`
(`MESURE_JOIGNABILITE=tout|fond|masse`).

| construction | passe 1 | passe 2 |
|---|---|---|
| `tout` : les vecteurs avec les lignes, des COPY successifs dans une table indexée (le défaut) | 1 | 0 |
| `fond` : un premier index sans vecteurs, puis la dette posée par SET ; l'index tombe au-delà de 2 000 de dette et se rebâtit à dette nulle | 18 | 24 |
| `masse` : `bulk_vector_index` (l'index tombe, la synchronisation pose les vecteurs, puis l'index est rebâti) | 1 | 0 |

- `tout` et `masse` : 0 ou 1 introuvable sur 12 396, la rareté d'un index approché.
- `fond` : 0,15 à 0,2 %. C'est pourtant le même CREATE sur une table pleine que `masse` :
  seule l'origine des vecteurs change (des SET plutôt qu'un COPY). C'est la piste d'un défaut
  du moteur, confiée au banc pour un témoin sans rag3weaver. Elle est peut-être voisine de
  « mise à jour massive de vecteurs ».
- Les introuvables changent d'une passe à l'autre. Tous sont minuscules, de 24 à 150
  caractères : des blocs de `use` en tête de fichier, des méthodes d'une ligne, de petites
  structures. Pour la moitié d'entre eux, le premier rendu est à 0,03 à 0,12 (un voisin
  quasi identique) ; pour l'autre moitié, à 0,3 à 0,4 (des isolés).

**Repli possible côté rag3weaver, non décidé** : rattraper une grosse dette de vecteurs par
le chemin `bulk_vector_index` plutôt que par SET puis CREATE. On n'y touche pas tant que le
banc n'a pas dit ce qu'est l'écart de `fond` (décision de l'orchestration) : si c'est un
défaut du moteur, il se corrige là-bas.

## L'écart du mode `fond` : non reproduit (5 octobre 2026)

**Statut : non reproduit, pas fermé.** L'écart ne bloque plus la stèle. On ne change pas la
conception, et `e2e_mesure_joignabilite` reste au dépôt.

**Les cinq passes `fond` de rag3weaver** (src/ de rag3weaver, granite-278m, distance cosine),
en nombre d'introuvables par leur propre vecteur :

| passe | morceaux | introuvables |
|---|---|---|
| fond 1 | 12 233 Scope_Chunk | 18 |
| fond 2 | 12 233 Scope_Chunk | 24 |
| fond 3, celle de l'export | 12 278 Scope_Chunk | 3 |

En regard, sur les deux mêmes séries de passes : `tout` 1 et 0, `masse` 1 et 0.

**Ce que le banc a éliminé** : il a rejoué sur les 12 278 vecteurs exportés, sans
rag3weaver, avec le même CREATE sans options.
- Masse (COPY des vecteurs puis CREATE) : 6 et 1. Fond (COPY sans vecteur, réclamation par
  lots de 512 en autocommit, SET des vecteurs par lots de 32, puis le même CREATE) : 5 et 3.
  Sur les mêmes vecteurs, il n'y a **aucun écart** entre les deux chemins.
- Les vecteurs identiques : 575 lignes en partagent un, par groupes de 42 au plus. Ils sont
  couverts par la définition (un vecteur identique en tête compte comme trouvé), des deux
  côtés. Les introuvables de rag3weaver ne viennent pas de là : leur premier rendu est à
  0,03 à 0,4, jamais à 0.

**Ce qui différait encore dans les deux passes à 18 et 24**, la piste qui reste si quelqu'un
le revoit :
1. le point de reprise automatique du moteur, au seuil du journal, pendant les SET ;
2. l'ordre d'écriture des vecteurs : chaque lot de 512 est trié par longueur de texte
   croissante (le banc a suivi l'ordre de la réclamation) ;
3. deux fils producteurs, dont les lots reviennent dans un ordre non garanti ;
4. un src/ un peu différent (12 233 morceaux au lieu de 12 278), donc d'autres vecteurs que
   ceux de l'export.

**Une passe de plus, sans urgence**, quand le correctif de l'index de clé primaire
(`splitSlots`, cœur C++) sera sur master : le vrai déroulé `fond` une fois, pour un chiffre
sur le moteur corrigé.

**La passe sur le moteur corrigé** (5 octobre 2026, 07 h 40, moteur ≥ `eb2d78e46`, le vrai
déroulé `fond` de rag3weaver, src/ à 12 461 morceaux) : **1 introuvable** (un Scope_Chunk,
premier rendu à 0,05), 0 repli en masse. C'est le niveau de `tout` et `masse`. Le correctif
de la division des cases de l'index de clé primaire touche tout point de reprise qui agrandit
un index de clé, ce qui inclut les points de reprise automatiques pendant les SET du mode
`fond` : il a pu jouer sur les 18 et 24 des premières passes, sans que rien ne le prouve.

## La règle d'élagage, mise à côté de l'article et de hnswlib (5 octobre 2026)

**« Des îlots de vecteurs identiques » : réfuté.** Après le `i = 0`, j'avais lu les
introuvables comme des groupes de doublons devenus îlots. Une sonde l'a démenti. La recherche
exhaustive (k et efs = 12 278) atteint les 12 278 lignes à chaque passe. Les 46 introuvables
de trois passes « masse » y sont tous, et aucun n'est retrouvé par son propre vecteur, même à
efs = 2000. Ils sont dans le graphe, mais leurs arêtes entrantes viennent de loin.
« Le groupe perdu en entier » était un artefact de comptage : des copies identiques lancent la
même requête, elles ont forcément le même sort.

**Ce qui les distingue : les quasi-doublons.** Sur les 11 874 vecteurs distincts, 40,5 % des
introuvables ont un voisin distinct à moins de 1e-4 en cosinus, contre 1,5 % de l'ensemble ;
à moins de 0,015, c'est 48,6 % contre 3,9 %. Les autres introuvables sont des points
ordinaires.

**Les trois règles**, pour la liste d'un nœud `q` dont les candidats sont triés par distance
croissante :

| | qui est comparé | un candidat est écarté si |
|---|---|---|
| la nôtre (`shrinkForNode`, depuis `725046754`) | `i` contre tout candidat **plus lointain** `j`, gardé ou non | `alpha·d(i, j) < d(q, i)` |
| l'article (Malkov et Yashunin, algorithme 4) | le candidat `e` contre les voisins **déjà gardés**, plus proches | `e` n'est pas plus proche de `q` que de chacun d'eux |
| hnswlib (`getNeighborsByHeuristic2`, lu) | idem, du plus proche au plus lointain, jusqu'à M gardés | `d(gardé, c) < d(q, c)`, inégalité stricte |

Notre règle est inversée : le plus lointain d'un amas serré est toujours gardé, et le plus
proche part. Dans un amas de quasi-doublons, un nœud voisin de l'amas ne garde que le membre
le plus loin de lui. Un membre peut ainsi ne recevoir d'arêtes que de nœuds placés « derrière »
l'amas, ce que la sonde voit.

**alpha.** Il vaut 1,1 par défaut chez nous. L'article et hnswlib n'en ont pas, ce qui revient
à alpha = 1. Le RobustPrune de DiskANN l'emploie dans le sens classique : `p'` est écarté si
`alpha·d(p*, p') ≤ d(p, p')`, avec alpha ≥ 1, ce qui garde davantage d'arêtes longues. La règle
classique s'entend donc avec notre alpha sans changer son sens. Nous gardons l'inégalité
stricte de hnswlib : avec `≤` et alpha = 1, une copie exacte gardée couvrirait tous les autres
candidats (`d ≤ d`).

**Les copies exactes.** Avec l'inégalité stricte, deux copies du vecteur de `q` ne se couvrent
jamais (`0 < 0` est faux). Elles s'accumulent dans la liste, comme chez hnswlib, et un groupe
plus grand que le degré n'aurait plus de sortie. Il faut donc une borne.
- Une copie se reconnaît octet pour octet, pas par une distance nulle : le cosinus de simsimd
  passe par une racine inverse approchée et rend 0 ou un bruit d'environ 1e-7 pour deux
  vecteurs identiques.
- Les copies d'un autre voisin déjà gardé sont couvertes par la règle classique elle-même
  (`alpha·0 < d(q, c)`).

**Les index existants** gardent leur graphe d'avant. Pour profiter de la nouvelle règle, il
faut les recréer (DROP puis CREATE_VECTOR_INDEX). Le format ne change pas.

**État au 5 octobre, midi (en pause).** La règle classique et la borne des copies sont
écrites, pas commitées. (Une mesure du 5 donnait 9 à 22 introuvables contre 6 et 10 : elle est
invalide, les deux colonnes tournaient sur l'extension du i = 0 ; voir la note du 10 octobre.)

**État au 10 octobre.** Le code est commité sur la branche `banc-elagage-en-cours`
(`9c030091d`), pas sur master. Les deux colonnes sont à rejouer avec l'extension rebâtie et
vérifiée. Seul chiffre valable de la vraie règle de master : en « masse », 33, 2 et 3
introuvables, et un rappel@10 de 0,992 à 0,996. Le détail et la suite sont au rapport du banc
(`extension/rag3weaver/docs/3-octobre-2026-23h31/banc-de-concurrence/01-rapport-de-session.md`,
« En pause depuis le 5 octobre à midi »).
