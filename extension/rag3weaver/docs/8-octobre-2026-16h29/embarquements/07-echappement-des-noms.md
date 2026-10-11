# L'échappement des noms — la page, avant le lot

11 octobre 2026 au matin, chantier F. **Décision de Lucie : la voie (b), par
l'échappement systématique.** Chaque identifiant que le dialecte écrit est
cité, toujours, réservé ou non : entité, champ, relation, colonnes internes,
DDL compris. Chaque dialecte a sa règle, comme tout constructeur de requêtes
établi. Le concepteur de gabarits ne voit rien : les noms restent ceux qu'il
a écrits, aucune base n'est migrée, aucune table de correspondance.

## Ce qui change dans le texte

| Dialecte | Avant | Après |
|---|---|---|
| rag3db | `MATCH (n:Order {_uuid: u}) RETURN n.title` | ``MATCH (n:`Order` {`_uuid`: u}) RETURN n.`title` `` |
| rag3db (DDL) | `CREATE NODE TABLE IF NOT EXISTS Order(_uuid STRING, …)` | ``CREATE NODE TABLE IF NOT EXISTS `Order`(`_uuid` STRING, …)`` |
| PostgreSQL | `SELECT title FROM Order WHERE _uuid = ANY($1)` | `SELECT "title" FROM "order" WHERE "_uuid" = ANY($1)` |

- **rag3db** : l'accent grave ne change pas le nom physique, donc une base
  existante se rouvre telle quelle.
- **PostgreSQL** : un nom cité garde sa casse, alors qu'un nom non cité est
  replié en minuscules. Une table créée hier sous `Document` s'appelle
  physiquement `document`. Pour **ne rien migrer**, le dialecte PostgreSQL
  cite le nom **replié** : `"document"`. C'est le seul point où la règle
  diffère d'un simple « entourer » : chaque dialecte dit comment il cite
  (`SchemaDialect::quote`).
- Les **littéraux** ne changent pas : une valeur, un paramètre `$x`, le nom
  d'un index passé en chaîne (`CALL QUERY_VECTOR_INDEX('Order', …)`).
- Les **alias** (`n`, `m`, `r`, `u`) ne sont pas des noms du schéma : ils
  restent nus.

## Où le texte s'écrit

- `dialect.rs` : les deux dialectes, toutes les méthodes et les cinq formes
  de l'IR. C'est le gros du lot.
- `schema.rs` : le DDL passe déjà par `create_table` / `create_rel_table` du
  dialecte ; il cite à travers eux.
- **Les deux moteurs de recherche** (`rag3db_search_backend.rs`,
  `postgres_search_backend.rs`) écrivent eux-mêmes leurs requêtes de vecteurs
  et d'enrichissement. Ils reçoivent la règle du dialecte. Sans cela,
  `Order` passerait le chargement et casserait à la recherche.
- **Ce qui ne bouge pas** : les tables internes à nom fixe (`_index_blobs`,
  `_catalog_meta`, les magasins de reprise) et le Cypher de `code_sync` et des
  nœuds du graphe de code, qui ne nomme que le schéma du code (`Scope`,
  `Symbol`…) : ces noms ne sont pas réservés. Ils restent nus jusqu'à ce que
  leurs sites passent par l'IR.

## Le cas limite : un nom qui contient lui-même le caractère de citation

Un nom avec un accent grave (rag3db) ou un guillemet (PostgreSQL) ne peut
pas être cité sans échappement interne. Il est **déjà refusé**, en le nommant,
par la validation des identifiants (`[a-zA-Z_][a-zA-Z0-9_]*`, `schema.rs`),
avant toute requête. Le lot ajoute un test qui épingle ce refus pour les deux
caractères. Assouplir cette validation (accents, espaces) serait une autre
décision.

## Ce que devient le refus de cette nuit

Le refus des noms réservés (`validate_name`, `reserved_words`, la liste
générée et son script, le test qui la suit) n'a plus de raison d'être :
plus rien ne le lirait. **Il est retiré** dans le même lot. Le ticket
« Order », fermé cette nuit, est mis à jour : fermé par l'échappement.

## Le témoin

`tests/e2e_nom_reserve.rs` change de sens : l'entité `Order` et le champ
`order` **passent de bout en bout** sur rag3db (chargement, écriture, lecture
par `Select`, saut, recherche), et le mot-clé non réservé aussi. Sur
PostgreSQL, le même témoin s'ajoute au test 5 du contrat
(`e2e_contrat_transactions.rs`, derrière `RAG3WEAVER_PG`, non prouvé vivant
tant qu'aucune base ne tourne).

## Les parités

Les tests qui épinglent le texte au caractère près (dialecte, graph_walk,
voisinage, usages) se refont une fois, mécaniquement : la requête attendue
est celle d'avant, avec les noms cités. Les e2e (code, usages, impact, liens,
ingestion par paquets, arrêts) prouvent que les lignes sont les mêmes.
