# Write et Tx — la page, avant le code

11 octobre 2026, session embarquements, chantier F. Ce sont les deux dernières
formes du langage intermédiaire (page 02). `Write` naît de l'ingestion, le
vrai client (note de la session mémoire). `Tx` s'appuie sur la connexion
tenue que la session recherche vient d'écrire. La lecture est faite sur
`83207a625` ; trois points vérifiés à la main.

## Ce que l'ingestion écrit aujourd'hui

Le graphe d'ingestion (`Catalog::ingest_entities`) enchaîne les écritures
suivantes : insertion (COPY ou MERGE), découpe, insertion des chunks,
vecteurs, liens chunk → parent, marque de découpe. Puis viennent les marques
de session, la table de mise de côté et les annulations. Tout passe déjà par
le dialecte, sauf trois choses (§3).

Les 31 méthodes d'écriture du trait se rangent en **sept formes** ; cinq de
ces méthodes n'ont aucun appelant en production :

| Forme | Méthodes aujourd'hui | Paramètres |
|---|---|---|
| **Upsert** des lignes par une clé | `batch_upsert`, `upsert_aside`, `upsert_meta`, `upsert_scope_node` (+ `kb_upsert_index`, sans appelant) | `$items` : une carte par ligne |
| **Lier** des lignes par leurs uuids | `batch_link`, `batch_link_labeled` | `$items` `{from_uuid, to_uuid, props…}` |
| **Mettre à jour** des champs par uuid | `batch_update_fields`, `embed_set`, `embed_clear`, `revert_lifecycle_state` (+ deux sans appelant) | `$items` `{_uuid, champs…}` (ou `uuid`, `emb`, `hash`, `state`) |
| **Marquer** une liste d'uuids (champ ← valeur, ou NULL) | `batch_set_null`, `mark_snapshot_session`, `mark_written_session`, `mark_absent_since`, `marquer_derivees_a_rendre`, `clear_aside` ; `purge_aside_before` (condition + limite) | `$uuids` + scalaires |
| **Supprimer** par uuid, avec ou sans les arêtes | `batch_delete`, `batch_cascade_delete`, `batch_cascade_delete_returning_count` (+ deux sans appelant) | `$uuids` |
| **Délier** une paire, ou les arêtes de nœuds retenus par une condition | `batch_delete_relation` ; et du Cypher brut (§3) | `$items` `{from, to}` ; condition |
| **Charger en masse** | `copy_nodes_from_csv`, `copy_links_from_csv` | un fichier |

Les migrations de tables entières (`set_column_null`, `copier_colonne`) et la
réclamation atomique (`reclamer_chunks_sans_marqueur`) restent hors de `Write` :
elles vont avec un DDL ou rendent des lignes. Une forme de plus s'ajoutera
quand un client en aura besoin.

**Le défaut que l'écart des clés cache déjà** (vérifié) : l'annulation d'un
lien envoie `{from, to}`, que rag3db lit, alors que PostgreSQL attend
`{from_uuid, to_uuid}`. Sur PostgreSQL, défaire un lien ne supprime donc
rien, sans erreur. Ticket :
`docs/tickets/2026-10-11-postgresql-defaire-un-lien-ne-trouve-rien.md`.
`Write` fixe les clés une fois pour toutes les formes, et c'est ce qui ferme
cette famille.

## La forme Write

```text
Write::Upsert  { table, key: "_uuid", columns, returning_ids: bool }
Write::Link    { relation, from_table, to_table, props }
Write::Update  { table, columns }                 // par _uuid
Write::Mark    { table, set: [(field, Value|Param|Null)], scope: Uuids | Predicate, limit }
Write::Delete  { table, edges: Keep | Cascade, returning_count: Option<field> }
Write::Unlink  { relation, by: Pairs | NodesWhere { table, predicate } }
Write::Load    { table | relation, path, columns }   // COPY ; refusé sans bulk_load
```

- Les **clés des lignes** sont celles des colonnes ; pour un lien,
  `from_uuid`, `to_uuid` et les propriétés ; pour une suppression ou une
  marque, `$uuids`. Il n'y a plus de convention propre à une méthode.
- **Rendre** (les ids d'un upsert, un compte) est un champ de la forme, pas
  une méthode de plus.
- Le prédicat est celui de `Select` (page 02 : `Equals`, `Contains`,
  `AtLeast`, `AnyOf`, `Compiled`). `Unlink { NodesWhere }` prend en charge la
  suppression des arêtes d'un fichier (`code_sync.rs`, à A).
- Chaque dialecte traduit, et refuse ce qu'il ne sait pas en le nommant. Sur
  PostgreSQL, `Load` est refusé, ce qui veut dire : passer par `Upsert`.

**Par où commencer** (ingestion d'abord) : `Upsert` et `Link`, les deux
écritures de chaque paquet, puis `Update` et `Mark` (vecteurs, marques), puis
`Delete` et `Unlink` (les annulations). Chaque pas exige la même parité que
pour `Hop` : le texte rag3db identique au caractère près quand c'est
possible, sinon les mêmes lignes en base après l'écriture.

## La forme Tx

```text
Tx::Begin | Tx::Commit | Tx::Rollback
```

`Tx` n'est pas une requête : c'est un appel sur la connexion.
`DbConnection::begin/commit/rollback` arrive avec la branche
`postgres-connexion-epinglee` de la session recherche. Le défaut passe par le
texte, comme aujourd'hui sur rag3db ; PostgreSQL tient une session du début à
la fin. Ce qui reste à faire de mon côté :

1. **`code_sync.rs`** (fichier de A) : `commencer` et `terminer` appellent
   `begin`, `commit` et `rollback` au lieu de passer `BEGIN TRANSACTION` en
   texte. `CALL force_checkpoint_on_copy=true` reste un réglage propre à
   rag3db, posé seulement si le dialecte déclare `bulk_load`. Diff proposé à A.
2. **`PostgresDialect`** déclare `transactions: true` quand la branche est sur
   master. Son témoin, le test 5 de la batterie (« un paquet défait ne laisse
   rien, un paquet validé tient »), est écrit et laissé `#[ignore]` derrière
   `RAG3WEAVER_PG` : **non prouvé vivant** tant qu'aucune base PostgreSQL ne
   tourne sur les postes.

Ce que le catalogue interdit pendant une transaction (pas de DDL, pas de
rebâti d'index, MERGE gardé jusqu'à la validation) reste à lui : `Tx` ne le
change pas.

## Ce qui reste du Cypher hors du dialecte, après Write et Tx

- `dataflow/record.rs`, l'enregistreur d'exécutions (gardé par Lucie). Il
  passera par `Upsert`, `Link` et `Delete`, plus une rétention (« les N plus
  anciennes ») qui est un `Mark`/`Delete` avec ordre et limite.
- `CypherNode` des migrations : du Cypher fourni par l'utilisateur, légitime.
  Son annulation, elle, écrite par le crate, deviendra un `Update`.
- `CypherCheckpointStore` et `CypherBlobStore` : ce sont les implémentations
  rag3db elles-mêmes, sous le dialecte, à leur place.

## Décidé (orchestration, renversable par Lucie)

- Les sept variantes restent distinctes. `Mark` se distingue d'`Update` en
  une phrase : **`Update` porte une valeur par ligne** (une carte par ligne
  dans `$items`, que le dialecte joint à la table), **`Mark` porte une seule
  valeur pour toute une liste d'uuids** (`$uuids` et des scalaires, un `SET`
  sans jointure : `WHERE _uuid = ANY($uuids)` sur PostgreSQL, `UNWIND $uuids`
  sur rag3db). Les deux dialectes les écrivent différemment aujourd'hui. Si
  un jour aucun ne les distingue plus, `Mark` se fond dans `Update`.
- `Load` reste dans `Write`, refusé en le nommant sans `bulk_load`.

## Premier pas : Upsert et Link

`Write::Upsert` et `Write::Link` se traduisent par les méthodes que chaque
dialecte écrit déjà (`batch_upsert`, `batch_link_labeled`) : le texte reste
celui d'avant, au caractère près, dans les deux dialectes (test
`l_ecriture_est_le_texte_d_avant`). L'insertion par MERGE, la recréation
d'une ligne à l'annulation et la pose des liens (`record_nodes.rs`) passent
par elles. Les appels de `catalog.rs` (le verrou de migration) sont à A.

## Pour décider (d'origine)

- Les sept variantes de `Write` ci-dessus, ou moins (fondre `Mark` dans
  `Update`, `Unlink` dans `Delete`) ? Je propose de les garder distinctes :
  elles n'ont ni les mêmes clés ni le même rendu.
- `Load` dans `Write`, ou à part ? Il ne parle pas en lignes mais en fichier.
  Je le garde dans `Write`, refusé sans `bulk_load`.
