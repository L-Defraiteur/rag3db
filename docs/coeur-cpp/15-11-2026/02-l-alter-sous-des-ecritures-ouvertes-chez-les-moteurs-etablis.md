# L'ALTER sous des écritures ouvertes : ce que font les moteurs établis, et ce que rag3db a choisi

15 novembre 2026, session hotfixs du cœur, à la demande de Lucie, par c7. C'est une page de lecture : il n'y a
ni code ni mesure. Toutes les citations viennent de la documentation officielle de chaque moteur, lue le
15 novembre ; les liens sont en fin de page.

La question : pendant qu'une transaction d'écriture est ouverte sur une table, quelqu'un fait un `ALTER`
(ajout ou retrait d'une colonne), un `DROP` ou un `CREATE INDEX`. On veut savoir :
- **qui attend qui** ;
- **ce que voit l'écrivain ouvert** ;
- **ce que voit un lecteur d'instantané** ;
- **qui est refusé**, et s'il doit recommencer.

## En une table

| moteur | l'ALTER face à un écrivain ouvert | un lecteur pendant l'ALTER | CREATE INDEX | le refus et le recommencement |
|---|---|---|---|---|
| **PostgreSQL** | il **attend** : `ACCESS EXCLUSIVE` contre `ROW EXCLUSIVE` | bloqué, même un simple `SELECT` (seul ce verrou bloque un `SELECT`) ; un ALTER **en file** bloque aussi ceux qui arrivent après | `SHARE` : il bloque les écritures, pas les lectures ; `CONCURRENTLY` ne bloque ni l'un ni l'autre, en deux balayages | pas de refus de schéma : on attend ; `lock_timeout` borne l'attente ; le schéma n'est pas versionné par instantané |
| **MySQL / InnoDB** | l'ALTER **en ligne** laisse passer les écritures ; il prend un verrou de métadonnées exclusif **bref**, au début et à la validation, et attend les transactions qui tiennent la table | une demande de verrou exclusif **en attente** bloque les transactions suivantes sur la table | `ADD INDEX` en place, lectures et écritures permises, il finit après les transactions qui touchent la table | pas de refus : on attend, jusqu'au délai du verrou |
| **SQLite** | un seul écrivain à la fois dans la base : l'ALTER *est* une écriture, il attend le verrou d'écriture | un lecteur WAL garde son instantané | même règle d'un seul écrivain | **`SQLITE_SCHEMA`** : une requête préparée sur l'ancien schéma est **re-préparée seule**, jusqu'à 50 fois |
| **DuckDB** | contrôle optimiste ; l'ALTER passe | MVCC, chacun voit son instantané | — | **refus** : « Transaction conflict: cannot update a table that has been altered! » ; on rejoue la transaction |
| **CockroachDB** | **ni attente ni verrou de table** : l'ALTER est un travail de fond, sur deux versions du schéma qui vivent ensemble | normal, sans effet sur la latence | en ligne, en fond | un ALTER dans une transaction explicite n'est **pas atomique** et peut échouer au COMMIT (`XXA00`) ; un plan préparé peut tomber sur « cached plan must not change result type » |
| **Neo4j** | il n'y a **pas d'ALTER de colonne** : le schéma des propriétés est libre, une propriété neuve est une écriture | — | **en ligne** : l'index se peuple en fond (`POPULATING`), il ne sert qu'une fois `ONLINE` | une transaction ne peut **pas mêler** schéma et écriture de données (erreur GQL 25N01) |

## Moteur par moteur

### PostgreSQL : l'attente, sur un verrou de table à huit modes

- **Les verrous.**
  - `ALTER TABLE` prend `ACCESS EXCLUSIVE` « unless explicitly noted » : c'est le cas d'`ADD COLUMN` et de
    `DROP COLUMN`. `DROP TABLE` le prend aussi.
  - Un écrivain (`INSERT`, `UPDATE`, `DELETE`, `MERGE`) prend `ROW EXCLUSIVE`, qui entre en conflit avec
    `SHARE`, `SHARE ROW EXCLUSIVE`, `EXCLUSIVE` et `ACCESS EXCLUSIVE`.
  - **L'ALTER attend donc l'écrivain ouvert**, et un écrivain qui arrive attend l'ALTER.
- **Les lecteurs.** « Only an ACCESS EXCLUSIVE lock blocks a SELECT » : pendant l'ALTER, même un `SELECT` attend.
- **La file d'attente.** C'est le piège connu. Un `ALTER` qui attend derrière une longue transaction
  bloque tous ceux qui arrivent après lui, lecteurs compris. MySQL l'écrit noir sur blanc ; PostgreSQL le
  fait de même, par la file de son gestionnaire de verrous. D'où l'usage de `lock_timeout` avant un ALTER
  en service. `lock_timeout` s'applique à « each lock acquisition attempt », verrous implicites compris.
- **Ce que voit l'écrivain ouvert.** Rien ne change pendant sa transaction : l'ALTER l'attend. Le verrou
  se prend **à l'analyse de la requête** (`parserOpenTable`), si bien qu'une requête qui a attendu l'ALTER
  se lie sur le schéma neuf, sans erreur.
- **Le lecteur d'instantané.** Le catalogue n'est pas lu à l'instantané. La documentation l'avoue :
  `TRUNCATE` et les formes réécrivantes d'`ALTER TABLE` « are not MVCC-safe ». Une transaction dont
  l'instantané précède la validation voit la table vide. Ce n'est pas le cas d'une transaction qui avait
  déjà touché la table : elle tient `ACCESS SHARE`, ce qui fait attendre le DDL.
- **Le coût de l'ALTER.**
  - `ADD COLUMN` avec un défaut non volatil ne réécrit rien : le défaut est rangé dans le catalogue, « very
    fast even on large tables ».
  - `DROP COLUMN` rend la colonne invisible, sans la retirer physiquement.
- **CREATE INDEX.**
  - Sans `CONCURRENTLY`, il prend `SHARE` : il « locks out writes (but not reads) », en un seul balayage.
  - Avec `CONCURRENTLY`, il prend `SHARE UPDATE EXCLUSIVE`, qui ne bloque pas les écritures. L'index naît
    « invalid », attend les transactions qui ont écrit la table, fait deux balayages, attend les
    instantanés plus vieux que le second, puis devient valide. En cas d'échec, il laisse un index
    « invalid » qui coûte aux écritures sans servir aux lectures.
- **Le refus.** Rien de propre au schéma : on attend. Les seuls refus à recommencer sont ceux de
  sérialisation (« could not serialize access »), sous `REPEATABLE READ` et `SERIALIZABLE`. Ils portent sur
  des lignes, pas sur le schéma.

### MySQL / InnoDB : l'ALTER en ligne, sous un verrou de métadonnées bref

- **Les algorithmes.**
  - `ADD COLUMN` est `INSTANT` par défaut depuis 8.0.12, et `DROP COLUMN` depuis 8.0.29 : seules les
    métadonnées changent, et les écritures concurrentes sont permises.
  - En `INPLACE`, la table se reconstruit, et les écritures restent permises.
- **Le verrou de métadonnées.** Il est exclusif, mais pris brièvement : à la préparation, puis à la
  validation de la définition neuve. Entre les deux, les écritures passent.
- **L'attente.** « an online DDL operation may have to wait for concurrent transactions that hold metadata
  locks on the table to commit or rollback ». Une transaction ouverte qui a seulement lu la table suffit
  à la faire attendre.
- **La file.** « a pending exclusive metadata lock requested by an online DDL operation blocks subsequent
  transactions on the table ». L'exemple de la documentation : une transaction ouverte fait un `SELECT`,
  un ALTER attend, puis un troisième `SELECT` est bloqué derrière l'ALTER.
- **ADD INDEX.** En place, « The table remains available for read and write operations ». Il ne finit
  qu'après « all transactions that are accessing the table », pour que l'index parte du dernier état.

### SQLite : un seul écrivain, et la requête re-préparée en silence

- **L'attente.** SQLite n'a qu'un écrivain à la fois par base. Un `ALTER` est une écriture : il attend la
  fin de l'écrivain ouvert, comme n'importe quelle écriture. En mode WAL, un lecteur garde son instantané.
- **Ce que voit une requête liée sur l'ancien schéma.** Le code `SQLITE_SCHEMA` : « the database schema
  has changed ». Une requête préparée par `sqlite3_prepare_v2()` est **re-préparée automatiquement**,
  jusqu'à `SQLITE_MAX_SCHEMA_RETRY` fois (50). L'application ne voit l'erreur que si elle persiste.

### DuckDB : l'optimisme, et le refus à rejouer

- DuckDB fait du contrôle optimiste sur un MVCC. Deux écrivains ne se gênent que sur une même ligne, et le
  second « will fail with a conflict error ».
- Un écrivain dont la table a été changée sous lui reçoit « Transaction conflict: cannot update a table
  that has been altered! ».
- La documentation conseille de rejouer : « A common workaround when a transaction conflict is
  encountered is to rerun the transaction ».
- C'est exactement la forme du refus nommé de rag3db, sans l'attente.

### CockroachDB : ni attente ni verrou, deux versions du schéma à la fois

- Le changement de schéma tourne « without holding locks on the underlying table data », comme un
  travail de fond, qu'on peut suspendre et reprendre.
- Il suit la méthode de F1 (Google, Rae et al., VLDB 2013) : une « bridging strategy involving concurrent
  uses of multiple versions of the schema ». Le schéma passe par des états intermédiaires : une colonne
  neuve est d'abord écrite sans être lue, puis rattrapée, puis rendue lisible. Deux versions voisines
  seulement vivent en même temps, si bien que lectures et écritures continuent « with no effect on
  read/write latency ».
- Le prix est dans les transactions explicites : CockroachDB « does not support schema changes within
  explicit transactions with full atomicity guarantees », sauf `CREATE TABLE` et `CREATE INDEX`. Une
  transaction peut échouer au COMMIT avec des instructions déjà passées (`XXA00`), et un plan préparé peut
  tomber sur « cached plan must not change result type ».

### Neo4j : un schéma libre, et des index en ligne

- Neo4j n'a pas d'ALTER de colonne. Les propriétés n'ont pas de schéma : en ajouter une revient à
  l'écrire, et la question de l'écrivain ouvert ne se pose pas.
- Le schéma, ce sont les index et les contraintes.
  - **Un index se crée en ligne.** Il se peuple en fond, dans l'état `POPULATING`, et ne sert qu'une fois
    `ONLINE`. On l'attend par `db.awaitIndexes`.
  - **Une transaction ne mêle pas schéma et écriture.** « they must be either of the same type, or be a
    combination of schema modifications and read commands » (code GQL 25N01). Le DDL vit dans sa propre
    transaction.

## Ce que rag3db a choisi cette nuit

C'est le lot du verrou de schéma du cœur, sur la branche `alter-attend-les-ecrivains` (tête `d5f5325f0`). Il
n'est **pas encore sur master** : le SIGSEGV de `DdlEveryMinute` le retient. Voir `01-rapport-de-session.md` et
le ticket `2026-11-15-une-ecriture-se-lie-avant-de-prendre-le-verrou-de-sa-table.md`.

- **L'attente, façon PostgreSQL.**
  - Un genre `TABLE` s'ajoute au gestionnaire de verrous.
  - Un écrivain le prend **en partagé** à sa première écriture dans une table.
  - `ALTER` (ADD, DROP ou RENAME d'une colonne) et `DROP TABLE` le prennent **en exclusif**, et attendent,
    dans la limite du délai de verrou habituel. Le graphe d'attente détecte les interblocages.
  - L'écrivain ouvert finit sur le schéma de son début, et son COMMIT n'a plus de défaut à rattraper.
- **CREATE_VECTOR_INDEX en partagé.** Il prend le verrou de table en partagé, comme un écrivain : il
  n'empêche pas les écritures, comme le `CREATE INDEX` de MySQL ou le `CONCURRENTLY` de PostgreSQL, mais
  il fait attendre un ALTER.
- **Le lecteur d'instantané** ne prend aucun verrou. Il n'est pas attendu, ne fait pas attendre, et lit
  le catalogue de son instantané. rag3db fait ici **mieux que PostgreSQL** : le catalogue est versionné,
  sans le trou « not MVCC-safe ».
- **Le refus nommé, façon DuckDB, en filet.** Une écriture dont la table a changé de disposition de
  colonnes depuis le début de sa transaction est refusée. Le message commence par `Schema changed, retry:`
  (`Transaction::SCHEMA_CHANGED_RETRY`), et l'écriture se recommence. rag3weaver reconnaît ce préfixe
  et recommence ; c'est un petit lot d'ED et d'archi. Deux ALTER d'une même table suivent la même règle :
  le second attend le premier, est refusé, puis passe relancé.

## Ce qui serait « plus cool »

1. **Le verrou pris à la liaison, comme PostgreSQL** (ticket du cœur cité plus haut). Aujourd'hui, le verrou
   se prend à la première écriture, après la liaison. Un écrivain en validation automatique qui a attendu
   un ALTER se fait donc refuser, alors qu'il pourrait se lier sur le schéma neuf. Pris à la liaison, il
   passerait sans refus. Le refus resterait le filet de la seule transaction ouverte avant l'ALTER.
2. **Recommencer dans le moteur, comme SQLite.** `SQLITE_SCHEMA` re-prépare la requête jusqu'à 50 fois,
   sans que l'application le voie. Pour une instruction en validation automatique, rag3db pourrait
   relier et rejouer lui-même. L'appelant n'aurait alors à recommencer que dans le cas d'une vraie
   transaction ouverte, dont seul lui connaît le reste.
3. **Se garder de la file, comme MySQL et PostgreSQL l'enseignent.** Un ALTER qui attend un long écrivain
   fait attendre tous les écrivains qui arrivent derrière lui. Deux mesures simples :
   - un délai d'attente propre au DDL, court (l'équivalent d'un `lock_timeout` avant l'ALTER), qui
     **refuse l'ALTER** plutôt que d'arrêter les écrivains, pour le relancer plus tard ;
   - ou un ALTER qui ne bloque pas les nouveaux venus tant qu'il n'a pas le verrou.
   C'est ce que `DdlEveryMinute` mesure déjà (`…-holds-writers-at-most-its-duration`).
4. **Un ALTER sans verrou du tout, comme CockroachDB et F1.** Une colonne neuve passerait par des états
   intermédiaires (écrite sans être lue, rattrapée, puis lisible), avec deux versions du schéma vivantes à
   la fois. C'est le plus beau, mais c'est un chantier. rag3db a déjà un catalogue versionné et un ADD
   sans réécriture : la marche est moins haute qu'ailleurs. Ce n'est utile que si l'ALTER devient
   fréquent en service. Aujourd'hui, l'hôte n'en fait qu'à l'ouverture ou à une montée de schéma.
5. **Un index qui se peuple en fond, comme Neo4j et `CONCURRENTLY`.** `CREATE_VECTOR_INDEX` durerait sans
   rien bloquer, dans un état « en construction » que les lectures ignorent, puis deviendrait « en
   ligne ». Le ticket `2026-11-15-un-create-vector-index-sous-quarante-ecrivains-dure-quatre-minutes.md`
   en montre le besoin : un index de quatre minutes sous quarante écrivains.

## Sources

- PostgreSQL, *Explicit Locking* : https://www.postgresql.org/docs/current/explicit-locking.html
- PostgreSQL, *ALTER TABLE* : https://www.postgresql.org/docs/current/sql-altertable.html
- PostgreSQL, *CREATE INDEX* (CONCURRENTLY) : https://www.postgresql.org/docs/current/sql-createindex.html
- PostgreSQL, *Caveats* (DDL non MVCC-safe) : https://www.postgresql.org/docs/current/mvcc-caveats.html
- PostgreSQL, *Client Connection Defaults* (`lock_timeout`) : https://www.postgresql.org/docs/current/runtime-config-client.html
- PostgreSQL, *Transaction Isolation* (erreurs de sérialisation) : https://www.postgresql.org/docs/current/transaction-iso.html
- MySQL 8.0, *Online DDL Operations* : https://dev.mysql.com/doc/refman/8.0/en/innodb-online-ddl-operations.html
- MySQL 8.0, *Online DDL Performance and Concurrency* (verrous de métadonnées) : https://dev.mysql.com/doc/refman/8.0/en/innodb-online-ddl-performance.html
- SQLite, *Result and Error Codes* (`SQLITE_SCHEMA`) : https://www.sqlite.org/rescode.html
- DuckDB, *Concurrency* : https://duckdb.org/docs/stable/connect/concurrency.html
- CockroachDB, *Online Schema Changes* : https://www.cockroachlabs.com/docs/stable/online-schema-changes
- Rae, Rollins, Shute, Sodhi, Vingralek, *Online, Asynchronous Schema Change in F1*, VLDB 2013 : https://research.google/pubs/online-asynchronous-schema-change-in-f1/
- Neo4j, code GQL 25N01 (mélange schéma et écriture) : https://neo4j.com/docs/status-codes/current/errors/gql-errors/25N01/
- Neo4j, *Indexes for search performance* (population en fond) : https://neo4j.com/docs/cypher-manual/4.4/indexes-for-search-performance/
