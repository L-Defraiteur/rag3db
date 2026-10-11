# Le chaînage des formes : un sous-graphe confié entier au dialecte

*11 octobre 2026. Session embarquements (chantier F). Page de conception, aucun
code. Suite des pages 01 à 07 de `8-octobre-2026-16h29/embarquements/`.*

Lucie veut que « tout parle en DAG partout ». Aujourd'hui une forme de l'IR
(`Hop`, `Count`, `Select`, `Write`) part seule vers le dialecte. Le code au-dessus
enchaîne lui-même : il lit les uuids rendus par un `Select`, puis les passe à un
`Hop`. La proposition est la suivante : une requête devient un **sous-graphe de
formes**, et ce sous-graphe est confié **entier** au dialecte. Le dialecte décide
alors ce qu'il traduit en une seule instruction et ce qu'il exécute en plusieurs.

## 1. La structure

- **Un nœud est une forme**, celle d'aujourd'hui, sans changement : `Hop`,
  `Count`, `Select` ou `Write` (avec ses sept variantes). Chaque nœud porte un
  identifiant local au graphe.
- **Une arête dit que la sortie de l'un devient l'entrée de l'autre.** Les formes
  ont déjà leurs entrées : les uuids d'un `Select::by_uuids`, le départ d'un
  `Hop`, les uuids d'un `Count::Edges`, la cible `by` d'un `Delete`. Chacune de
  ces entrées peut recevoir deux choses :
  - une **valeur**, comme aujourd'hui ;
  - une **référence** de la forme « la colonne *c* rendue par le nœud *n* ».
- **Un nœud peut nourrir deux suivants.** Par exemple, un `Select` des
  déclarations d'un fichier nourrit à la fois un `Hop` vers leurs usages et un
  `Count::Edges` de leur degré. Le graphe est un DAG, pas une chaîne.
- **Les sorties sont désignées.** L'appelant dit quels nœuds il veut lire. Il
  reçoit les lignes de chacun de ces nœuds, rangées par identifiant de nœud. Un
  nœud intermédiaire peut ne rien rendre à l'appelant.

Avant de passer le graphe au dialecte, une vérification **commune à tous les
dialectes** a lieu, une seule fois. Elle porte sur trois points :

- le graphe n'a pas de cycle ;
- chaque référence désigne une colonne que le nœud amont rend ;
- le type de la colonne convient à l'entrée. Par exemple, un compte ne va pas
  dans une entrée d'uuids.

Une faute à cette étape donne `TranslateError::Invalid`, comme aujourd'hui pour
une forme seule. Elle ne dépend d'aucun moteur.

## 2. Ce qui reste à la couche au-dessus

**Rien de textuel.** La couche au-dessus construit le graphe en valeurs Rust :
des formes et des références. Elle n'écrit aucune instruction et ne cite aucun
nom. Les noms du schéma voyagent tels que déclarés, et c'est le dialecte qui les
échappe, selon sa règle, en écrivant l'instruction (page 07).

**Aucun conflit avec nos verbes.** Les mots `Select`, `Hop`, `Count` et `Write`
sont des types Rust. Ils ne sont jamais des mots d'une requête. La page 07 l'a
prouvé pour les formes seules : `e2e_nom_reserve` passe de bout en bout avec des
entités nommées `Select`, `hop` et `Count`. Le graphe ne fait qu'assembler ces
formes et n'ajoute aucun mot.

Une seule chose remonte du dialecte vers la couche au-dessus : le **rapport**
(§ 5).

## 3. Ce qui se fond en une instruction, et ce qui ne se fond pas

La règle est la même pour les deux dialectes : **ne pas fondre n'est jamais un
refus.** Un dialecte qui ne sait pas fondre deux nœuds les exécute l'un après
l'autre. Il passe les uuids rendus par le premier en paramètre du second, comme
le fait aujourd'hui la couche au-dessus. Fondre est une optimisation et le
découpage est le cas de repli. Seule une **forme** que le dialecte ne sait pas
exécuter du tout est un refus (§ 4).

### rag3db (Cypher)

**Se fond.** Une chaîne `Select` → `Hop` → `Count` qui finit sur une seule
sortie. Par exemple, « les déclarations de ce fichier, ce qu'elles définissent,
et le degré de chacune » :

```
MATCH (m:`Order`) WHERE m.`_uuid` IN $uuids
MATCH (m)-[:`defines`]->(n:`Symbol`)
OPTIONAL MATCH (n)<-[r:`uses`]-()
RETURN n.`_uuid`, count(r)
```

Les clauses `MATCH`/`WITH` de Cypher sont faites pour cela. Une écriture qui
suit une lecture se fond aussi : un `Select` suivi d'un `Write::Mark` donne
`MATCH … SET`.

**Ne se fond pas.**

- **Deux sorties désignées.** Une requête Cypher rend un seul tableau de lignes.
  Un `Select` qui nourrit un `Hop` et un `Count`, tous deux demandés par
  l'appelant, donne deux instructions. Le `Select` est rejoué dans chacune, ou
  bien exécuté une fois et ses uuids passés en paramètre. Le dialecte choisit.
  Tout replier dans un `collect()` serait possible, mais ce serait un détour
  illisible, et on ne le fait pas.
- **`Write::Load`.** `COPY` est une instruction à part dans le moteur. Elle ne se
  compose avec aucune clause.

### PostgreSQL

**Se fond.** Le même `Select` → `Hop` → `Count`, écrit en CTE :

```
WITH m AS (SELECT v.u FROM unnest($1::text[]) AS v(u)
           JOIN "order" AS o ON o."_uuid" = v.u),
     n AS (SELECT d."to_uuid" AS u FROM m JOIN "defines" AS d ON d."from_uuid" = m.u)
SELECT n.u, count(r.*) FROM n LEFT JOIN "uses" AS r ON r."to_uuid" = n.u GROUP BY n.u
```

PostgreSQL fond aussi des **écritures chaînées** en une instruction, grâce aux
CTE qui modifient des données. Par exemple, `WITH d AS (DELETE … RETURNING
"_uuid") DELETE FROM "defines" WHERE "from_uuid" IN (SELECT "_uuid" FROM d)` est
un `Delete` suivi d'un `Unlink`, en une seule instruction atomique. rag3db ne
sait pas faire cela en une instruction : il lui en faut deux, dans une
transaction.

**Ne se fond pas.**

- **Une lecture de table placée après une écriture de la même instruction.**
  Toutes les CTE d'une instruction voient le même instantané. Un `Select` sur
  `"order"` placé après un `Write::Update` de `"order"` dans la même instruction
  verrait les anciennes valeurs. Une arête reste sûre : le nœud aval lit ce que
  rend le `RETURNING` du nœud amont, pas la table. Le dialecte coupe donc
  l'instruction **dès qu'un nœud lit une table qu'un nœud antérieur écrit sans
  arête entre eux**.
- **`Write::Load`.** `COPY` ne va pas dans une CTE, et le dialecte ne déclare
  d'ailleurs pas `bulk_load` aujourd'hui.
- **Deux sorties désignées.** C'est la même limite que pour rag3db : une
  instruction rend un seul résultat.

## 4. Quand un dialecte refuse un sous-graphe (P3)

Le refus a lieu **avant toute exécution**. Le dialecte parcourt d'abord le graphe
entier. Il refuse si un nœud porte une forme qu'il ne sait exécuter **ni seule ni
découpée**, par exemple un `Load` sur PostgreSQL sans `bulk_load`. Rien n'est lu
et rien n'est écrit. Un graphe à moitié exécuté ne peut donc pas laisser une
moitié d'écriture.

Le refus garde le nom de la règle P3 et ajoute le nœud fautif :

```
PlanError::Refused { node: "charger", message: "RAG3WEAVER_REFUS__le_dialecte_postgresql_ne_traduit_pas__Write::Load" }
→ RAG3WEAVER_REFUS__le_dialecte_postgresql_ne_traduit_pas__Write::Load (nœud « charger »)
```

Le dialecte nomme le **premier** nœud refusé dans l'ordre topologique. Le
message est celui du refus d'une forme seule (`SchemaDialect::untranslated`) ;
le nœud est le seul champ nouveau.

## 5. Le rapport

Le rapport reprend la règle que la session mémoire vient de poser pour les liens
(`2d0687a71`), étendue à n'importe quel graphe :

| Exécution | `atomic` | `atomic_reason` |
|---|---|---|
| une seule instruction | `true` : atomique par le moteur | — |
| plusieurs instructions, dialecte à `transactions`, toutes dans la transaction | `true` : enveloppées de `begin` / `commit` / `rollback` | — |
| plusieurs instructions dont une hors transaction (`Load` si le moteur ne l'y admet pas, plein texte et vecteurs rebâtis après coup) | `false` | la partie qui sort, nommée |
| plusieurs instructions, dialecte sans `transactions` | `false` | « le dialecte n'a pas de transactions » |

Le rapport porte aussi deux autres champs :

- **`instructions`**, le nombre d'instructions envoyées ;
- **`fused`**, les groupes de nœuds partis ensemble, par exemple
  `[["sel","hop","deg"],["mark"]]`.

Ces champs disent ce que le dialecte a fait de ce qu'on lui a confié. Un test les
lit, et un agent aussi.

**Un point pour une lecture découpée en plusieurs instructions.** Sous
PostgreSQL, le niveau par défaut (`READ COMMITTED`) prend un instantané par
instruction. Pour annoncer `atomic: true` sur une lecture en plusieurs
instructions, le dialecte ouvre la transaction en `REPEATABLE READ`. Sinon il dit
`false`.

## 6. Écarts avec ce que fait un planificateur SQL

Le reste est ce que fait n'importe quel planificateur SQL : un graphe d'opérateurs
où une sortie nourrit une entrée, où les CTE se fondent ou se matérialisent, et
où l'instruction est atomique. Seuls les points suivants diffèrent.

1. **Pas de coûts, des règles.** Le dialecte fond selon la forme du graphe, sans
   statistiques, et ne réordonne pas les nœuds. Le planificateur du moteur
   optimise toujours *à l'intérieur* de chaque instruction. Il y a donc deux
   étages : notre découpe, puis son plan.
2. **Plusieurs sorties.** Une requête SQL rend un seul résultat. Notre graphe
   peut en rendre plusieurs. Ce point est la première raison de couper en
   plusieurs instructions.
3. **La découpe est visible.** SQL cache son plan, sauf avec `EXPLAIN`. Ici, le
   rapport dit toujours combien d'instructions sont parties et quels nœuds sont
   partis ensemble.
4. **L'atomicité est annoncée, pas supposée.** En SQL, une instruction est
   toujours atomique. Ici, un graphe peut ne pas l'être, et il le dit, avec sa
   raison.
5. **Le refus a lieu avant l'exécution, et il nomme une forme.** Ce n'est pas une
   erreur de syntaxe ou d'exécution en cours de route.
6. **L'ordre ne vient que des arêtes.** Deux nœuds sans arête entre eux n'ont pas
   d'ordre garanti, même si l'un écrit ce que l'autre lit. En SQL, l'ordre
   d'écriture des instructions suffit à fixer cet ordre.

## Ce que Lucie a tranché (11 octobre 2026)

- **Pas d'arête d'ordre seul pour commencer.** Mais l'arête porte « zéro ou
  plusieurs colonnes » (`Edge::columns`), pour qu'une arête d'ordre soit plus
  tard un cas de l'arête existante, pas un type de plus. La vérification
  commune refuse aujourd'hui une arête sans colonne, en le disant.
- **Un graphe non atomique s'exécute et le dit** (`atomic: false` et sa raison,
  `instructions`, `fused`). Le refus seulement si l'appelant exige l'atomicité
  (`PlanOptions::require_atomic`), avant toute exécution.

## Le premier lot

- Le type du graphe et la vérification commune, dans `rag3weaver-ir`
  (`ir/src/plan.rs`) : `Form`, `PlanNode`, `Edge`, `Plan::check`.
- Le repli découpé, commun aux deux dialectes (`src/plan.rs`, `run_plan`,
  `Catalog::run_plan`) : un nœud, une instruction, dans l'ordre topologique.
  Une arête passe en `$uuids` les valeurs **distinctes** de sa colonne, dans
  l'ordre où elles paraissent ; une fusion devra respecter cette règle (un
  `WITH DISTINCT`, pas une ligne par chemin). Une entrée vide n'envoie pas
  d'instruction. Plusieurs instructions partent dans une transaction quand le
  dialecte en déclare ; PostgreSQL y fixe l'instantané
  (`SchemaDialect::snapshot_read`, `REPEATABLE READ`) dès que le graphe lit.
- Pour l'instant, une arête ne nourrit que l'entrée `$uuids` d'une forme ;
  l'entrée `$items` d'une écriture (`Upsert`, `Link`, `Update`, `Unlink`) se
  remplit encore par l'appelant, et la vérification le dit.
- Les témoins : `tests/e2e_chainage.rs` sur une base rag3db vivante, l'oracle
  étant la même suite de formes exécutée à la main ; les refus et l'atomicité
  par des tests unitaires qui prouvent que rien n'est parti.

Poussé : `76cf86cbc`.

## La fusion rag3db (deuxième lot)

- `SchemaDialect::fuse(texte d'amont, forme d'amont, colonne, texte d'aval)` :
  `None` par défaut, et le graphe découpe. rag3db fond une sélection ou un saut
  **sans ordre ni limite** en amont, et toute forme qui commence par
  `UNWIND $uuids AS <alias>` en aval (saut, sélection par uuids, degré, marque,
  suppression). Le `RETURN` d'amont devient
  `WITH DISTINCT <colonne> AS __chain WHERE __chain IS NOT NULL WITH __chain AS <alias>`,
  puis l'aval reprend après son `UNWIND`. Le second `WITH` ne garde que l'alias :
  les variables d'amont sortent de portée et l'aval les redéclare comme s'il
  était seul. C'est la règle du repli (valeurs distinctes, non nulles), dite en
  Cypher.
- L'exécuteur fait des **groupes** : un nœud rejoint le groupe de son amont quand
  l'amont en est le dernier nœud, n'a que lui pour suivant, n'est pas une sortie,
  que leurs paramètres ne se recouvrent pas, et que le dialecte sait les fondre.
  Une instruction par groupe ; l'atomicité se décide sur le nombre de groupes
  (un seul : atomique par le moteur, sans transaction autour).
  `PlanOptions::split` force le repli.
- Exemple, la chaîne `Select` → `Hop` → `Count` du témoin, en une instruction :

```
UNWIND $uuids AS u MATCH (m:`File` {_uuid: u})
WITH DISTINCT u AS __chain WHERE __chain IS NOT NULL WITH __chain AS u
MATCH (d:`File` {_uuid: u})-[r:`defines`]->(m:`Symbol`)
WITH DISTINCT m.`_uuid` AS __chain WHERE __chain IS NOT NULL WITH __chain AS u
MATCH (d:`Symbol` {_uuid: u})<-[r:`uses`]-() RETURN u, count(r)
```

- Témoins : chaque scénario d'`e2e_chainage` joue fondu puis découpé, et chaque
  sortie a les mêmes lignes ; l'écriture défaite par l'échec du nœud suivant
  tient dans les deux modes (fondu : `[sel, marquer]` puis `[fautif]`, deux
  instructions dans une transaction, défaite). Rouges sous un bouchon qui coupait
  la fusion, verts sans.
Poussé : `7da725287`.

## Le saut PostgreSQL, puis la fusion en CTE (troisième et quatrième lots)

- **Le saut** (`d0aacfcf7`) : `unnest($uuids)` joint aux arêtes par le bout de
  départ (`from_uuid` en sortant, `to_uuid` en entrant), puis à la table
  d'arrivée par l'autre bout ; la table de départ, connue, est jointe aussi. Sans
  table d'arrivée, un champ, une étiquette, un filtre ou un ordre sur le nœud
  atteint se refusent en se nommant, et la ligne entière aussi. Le scénario
  d'`e2e_chainage` est devenu commun aux deux moteurs ; `e2e_postgres` le joue sur
  le conteneur (`le_chainage_*`).
- **La fusion en CTE** : l'amont devient `WITH __chain(c0, c1, …) AS (…)`, et
  chaque `$uuids` de l'aval devient
  `ARRAY(SELECT DISTINCT cK FROM __chain WHERE cK IS NOT NULL)`. Une chaîne déjà
  fondue devient le corps de la CTE suivante. Une sélection, un saut ou un compte
  se fondent en amont, même ordonnés ou bornés : la CTE garde ses lignes, ce que
  rag3db ne sait pas faire avec un `WITH` de Cypher. **Jamais une écriture en
  amont**, dans aucun des deux dialectes : toutes les parties d'une instruction
  PostgreSQL voient le même instantané, et la lecture placée après ne verrait pas
  l'écriture. Le témoin `une_lecture_apres_une_ecriture_reste_decoupee` (les deux
  moteurs) : un `Upsert` nourrit la relecture de la même table, le rapport dit
  `[["ecrire"], ["relire"]]`, et la relecture voit la nouvelle valeur.
- La chaîne du témoin, sur PostgreSQL :

```
WITH __chain(c0, c1) AS (
  WITH __chain(c0, c1) AS (
    SELECT v.u, m."_uuid" FROM unnest($uuids::text[]) AS v(u) JOIN "file" AS m ON m."_uuid" = v.u)
  SELECT v.u, r."to_uuid" FROM unnest(ARRAY(SELECT DISTINCT c0 FROM __chain WHERE c0 IS NOT NULL)::text[]) AS v(u)
    JOIN "file" AS d ON d."_uuid" = v.u JOIN "defines" AS r ON r."from_uuid" = v.u
    JOIN "symbol" AS m ON m."_uuid" = r."to_uuid")
SELECT v.u, count(*) FROM unnest(ARRAY(SELECT DISTINCT c1 FROM __chain WHERE c1 IS NOT NULL)::text[]) AS v(u)
  JOIN "uses" AS r ON r."to_uuid" = v.u GROUP BY v.u
```

- Les deux écarts entre dialectes, dits ici plutôt que cachés : PostgreSQL fond
  un amont ordonné ou borné, rag3db non ; ni l'un ni l'autre ne fond deux sorties
  désignées (une instruction rend un seul tableau).
