# Une propriété de chaîne d'une relation se lit fausse dans un sens ou l'autre après un point de reprise

- **État** : corrigé le 4 octobre 2026 (« fix(stockage): la relecture partielle d'un segment de chaînes ne permute plus les chaînes de ses lignes — un point de reprise faussait les propriétés de chaîne des relations »). Le correctif empêche les dégâts suivants ; il ne répare pas une base déjà atteinte.
- **Gravité** : réponse fausse, écrite sur disque
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (la marque `resolution` des arêtes d'usage, que lisent les filtres de Liens ; `usage`, `kind`)

## Ce que c'est

Un point de reprise qui suit l'ajout ou la mise à jour de relations échange
les chaînes d'une propriété **très dupliquée** (`resolution`, `usage`, `kind`)
entre des relations de la même table qu'aucune écriture n'a touchées. Aucune
erreur. Les autres propriétés de ces relations restent justes, et un seul des
deux rangements de la table (une relation est rangée une fois par sens) est
faussé à la fois : la même relation rend alors deux valeurs selon la forme de
la requête. **Lequel des deux est faux dépend de la région réécrite, et le
planificateur choisit lequel une requête lit : aucun sens n'est sûr par
principe.**

Vu par l'arbre principal (4 octobre, soir) : 140 arêtes `CONSUMES` internes à
des fichiers neufs lues `resolution = nom` par une forme et `fichier` par
l'autre. D'abord lu comme une mise à jour qui aurait manqué un sens ; ce
n'était pas ça — ces arêtes n'ont très probablement jamais reçu d'écriture
`nom` (inféré de la recette, non rejoué sur l'historique de cette base).

Le dégât reste sur disque : une base qui a reçu des ajouts ou des mises à
jour de relations après un premier point de reprise est suspecte, et se
réindexe une fois le correctif passé.

## La recette (Cypher brut)

```
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY);
CREATE REL TABLE M(FROM Doc TO Doc, res STRING, line INT64);
UNWIND range(0, 2399) AS i CREATE (:Doc {id: i});
UNWIND range(0, 1999) AS i MATCH (a:Doc {id: i}) WITH a, i
  MATCH (b:Doc {id: (i * 11) % 2400}) CREATE (a)-[:M {res: 'vieux', line: -1}]->(b);
CHECKPOINT;
UNWIND range(2400, 2599) AS i CREATE (:Doc {id: i});
UNWIND range(0, 2029) AS i MATCH (a:Doc {id: i % 2400}) WITH a, i
  MATCH (b:Doc {id: (i * 7 + 3) % 2400}) CREATE (a)-[:M {res: 'fichier', line: i}]->(b);
UNWIND range(0, 139) AS i MATCH (a:Doc {id: 2400 + i}) WITH a, i
  MATCH (b:Doc {id: 2400 + (i * 3 + 1) % 60}) CREATE (a)-[:M {res: 'fichier', line: 5000 + i}]->(b);
CHECKPOINT;
MATCH (a:Doc {id: 2400})-[r:M]->(b:Doc) SET r.res = 'nom';
CHECKPOINT;
```

Après le dernier point de reprise, 700 des 4 170 relations ont changé de
chaîne dans l'un des deux rangements (413 « fichier » rendent « vieux », 287
« vieux » rendent « fichier ») ; `line` est juste partout ; l'autre rangement
est juste. Avant ce point de reprise, tout est juste. Avec une seule
génération de relations (sans les « vieux »), rien. Les relations de la
deuxième génération peuvent venir d'un COPY, c'est égal.

## La cause

Le point de reprise d'une table de relations réécrit ses régions modifiées
(1 024 nœuds chacune) et, pour cela, relit de chaque région les lignes déjà
sur disque (`CSRNodeGroup::checkpointColumnInRegion`). Pour une colonne de
chaînes, cette relecture **partielle** d'un segment
(`StringColumn::scanSegment` vers un bloc, branche « pas tout le segment »)
ne charge que les chaînes de la région et renumérote les lignes. Elle
numérotait les chaînes dans leur **ordre d'apparition** dans la région, puis
demandait au dictionnaire de les charger ; celui-ci, pour une colonne très
dupliquée (moins d'une chaîne distincte pour deux lignes), **trie** d'abord
la demande par indice du disque et range dans cet ordre. Quand les deux
ordres diffèrent — une région dont la première ligne porte une chaîne arrivée
tard dans le dictionnaire — les lignes de la région échangent leurs chaînes.
Le même appel écrivait en outre un indice dans la mauvaise case de la colonne
d'indices (il prenait un numéro de chaîne pour un numéro de ligne).

C'est du code d'origine : les lignes fautives sont celles de Kuzu (le tri
dans `DictionaryColumn::scan`, la numérotation dans
`StringColumn::scanSegment`, retouchée par l'amont en septembre 2025 sans
changer ce point). Ni `80e3f2c32` (régions non réécrites) ni `308ebd17e`
(colonnes par position) n'y touchent.

## L'étendue (exécutée sur le moteur non corrigé)

La même recette par type de colonne, toutes les lignes comparées avant et
après le dernier point de reprise :

| colonne | effet |
|---|---|
| relation `STRING` | 700 relations faussées sur 4 170, un rangement |
| relation `BLOB` | 700 |
| relation `STRUCT(s STRING)` | 700 |
| relation `STRING[]` à deux éléments (`['vieux','vieux']` puis `['fichier','vieux']`) | 2 530 et 2 738 relations faussées, **les deux rangements** |
| **nœud** `STRING[]` (listes alternées de deux ou trois chaînes, mises à jour éparses) | 4 lignes faussées sur 400 000 |
| relation `STRING[]` à un élément, ou à chaînes toutes distinctes dans la liste ; `INT64[]` | aucune |
| nœud `STRING`, `BLOB` | aucune (400 000 lignes, deux générations, mises à jour éparses, chaînes alternées) |

**Les listes de chaînes sont donc atteintes aussi, et sur les tables de nœuds**
(trouvé à la relecture croisée du banc) : `ListColumn::scanSegment` ne relit
du disque que la plage de ses listes, donc passe par la même relecture
partielle des chaînes, y compris quand le segment de la liste est relu en
entier. Les nœuds à `STRING` ou `BLOB` simple ne l'atteignent pas : le point
de reprise d'une table de nœuds relit des segments entiers
(`NodeGroup::checkpoint`), ce qui prend l'autre branche. Le correctif porte
sur la relecture elle-même, donc sur tous ses appelants.

## Le correctif de l'amont

Kuzu (archivé) porte le défaut tel quel. Ladybug l'a corrigé, par une
fonction à part qui rend la correspondance des indices. Lu, puis réécrit :
même principe (le dictionnaire dit où il a rangé chaque chaîne, les lignes
sont renumérotées ensuite), code à nous. Vela : non regardé.

## Témoin

`test/transaction/rel_string_property_checkpoint_test.cpp`, cinq cas : une
seule mise à jour puis un point de reprise ; le lot de 140 mises à jour de la
base du produit ; la réouverture ; une liste de chaînes sur des relations ;
une liste de chaînes sur des nœuds. Chacun compare toutes les lignes avant et
après (les relations par les deux formes). Les trois premiers joués rouges
avant le correctif ; les deux cas de listes vus fautifs par le même scénario
en programme à part sur le moteur non corrigé ; les cinq verts après.

## Savoir si une base est atteinte

`tools/check_rel_directions/check_rel_directions.cpp`, programme à part
(base ouverte en lecture seule) : il lit chaque table de relations par les
deux formes et compare, relation par relation, propriété par propriété. Un
écart prouve que la table est atteinte ; il ne dit pas lequel des deux
rangements est juste, et l'absence d'écart ne prouve pas que la base est
saine (les deux rangements peuvent avoir été faussés au même endroit). **Il
ne voit pas les listes de chaînes des tables de nœuds** : un nœud n'a qu'un
rangement, rien à quoi le comparer — seule la source le dit, donc une
réindexation.
Éprouvé sur la base de la recette (700 relations sur `res`) et sur une copie
de la base du produit (`CONSUMES` et `CONSUMED_BY`, 140 relations sur
`resolution`, les treize autres tables égales).

## Ce que ça fait à rag3weaver

Toute colonne `STRING[]` d'une table de nœuds ou de relations, et tout ce qui
lit `resolution`, `usage` ou `kind` sur une arête : les filtres
d'arête (Liens, impact, impact d'un fichier), la mention « (par le nom) »
d'`usages`, le genre d'usage, et le genre des rendez-vous. Aucune de ces
lectures n'est sûre sur une base écrite avant le correctif.

Les mesures de la session codeparsers du 4 octobre (banc des relations,
sondes des marques) viennent d'index neufs, construits en mémoire à chaque
passe ; qu'un point de reprise y soit survenu en cours d'ingestion n'est pas
établi. À rejouer après le correctif, sur une indexation de zéro, avant d'y
fonder une décision.

## Ce qui reste, hors du moteur

Une réindexation de zéro des bases écrites avant le correctif ; le contrôle
ci-dessus à zéro écart ; puis le banc des relations rejoué. Le témoin
générique « les deux formes rendent les mêmes propriétés avant et après un
point de reprise » entre au banc de concurrence (session du banc).
