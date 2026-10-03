//! Schema dialect abstraction for multi-backend support.
//!
//! The [`SchemaDialect`] trait generates DDL and DML statements for a specific
//! database backend. Implementations exist for rag3db (Cypher) and PostgreSQL (SQL).

use crate::config::FieldType;

/// Column definition for table generation.
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: ColumnType,
}

/// Abstract column types mapped to backend-specific types.
#[derive(Debug, Clone)]
pub enum ColumnType {
    /// Text/string column (STRING in rag3db, TEXT in PostgreSQL).
    Text,
    /// 64-bit integer (INT64 / BIGINT).
    Int64,
    /// Double precision float (DOUBLE / DOUBLE PRECISION).
    Double,
    /// Boolean (BOOLEAN).
    Boolean,
    /// Timestamp (TIMESTAMP).
    Timestamp,
    /// Binary blob (BLOB / BYTEA).
    Blob,
    /// Fixed-size float vector for embeddings (FLOAT[N] / vector(N)).
    Vector(usize),
    List(Box<ColumnType>),
    Struct(std::collections::BTreeMap<String, ColumnType>),
}

impl ColumnType {
    /// Convert from a user-facing [`FieldType`] to a [`ColumnType`].
    pub fn from_field_type(ft: &FieldType) -> Self {
        match ft {
            FieldType::String
            | FieldType::Text
            | FieldType::Json
            | FieldType::Tags
            | FieldType::Choice => ColumnType::Text,
            FieldType::Int64 | FieldType::Integer => ColumnType::Int64,
            FieldType::Double | FieldType::Number => ColumnType::Double,
            FieldType::Boolean => ColumnType::Boolean,
            FieldType::Timestamp => ColumnType::Timestamp,
            FieldType::List(item) => ColumnType::List(Box::new(Self::from_field_type(item))),
            FieldType::Struct(fields) => ColumnType::Struct(fields.iter().map(|(k, v)| (k.clone(), Self::from_field_type(v))).collect()),
        }
    }
}

/// Trait for generating backend-specific DDL and DML.
///
/// Each method returns one or more SQL/Cypher statements as strings.
/// The caller executes them via [`DbConnection`](crate::connection::DbConnection).
/// Les colonnes de chunk, dans **l'ordre que lit `resolve_and_enrich_chunked`**.
///
/// Extraites pour que les deux dialectes ne puissent pas diverger : cette
/// lecture se fait par position, donc un ordre différent ne produirait pas une
/// erreur mais des champs échangés — un `start_char` lu comme un `index`.
pub fn colonnes_de_chunk(alias: &str, has_source_refs: bool) -> Vec<String> {
    let mut v = vec![
        format!("{alias}._uuid AS c_uuid"),
        format!("{alias}._text AS c_text"),
        format!("{alias}._index AS c_idx"),
        format!("{alias}._parent_field AS c_field"),
        format!("{alias}._start_char AS c_start"),
        format!("{alias}._end_char AS c_end"),
        format!("{alias}._start_line AS c_sline"),
        format!("{alias}._end_line AS c_eline"),
        format!("{alias}._content_offset AS c_content_offset"),
    ];
    if has_source_refs {
        v.push(format!("{alias}._source_entity AS c_source_entity"));
        v.push(format!("{alias}._source_uuid AS c_source_uuid"));
    }
    v
}

/// **Le NULL d'un CSV de chargement en masse.** Un mot convenu, jamais une
/// cellule vide : la cellule vide est une chaîne vide.
pub const CSV_NULL: &str = "__rag3weaver_null__";

/// **Retrouver par sa clé le nœud de chaque ligne d'un lot** — la forme que
/// toutes les requêtes `UNWIND` du dialecte prennent (3 octobre 2026).
///
/// `MATCH (n:T {_uuid: item.champ})` balayait la table entière : le moteur
/// ne sait faire une jointure de hachage que si l'égalité porte sur une
/// **variable simple**, pas sur un champ de structure. Le plan était un
/// produit cartésien de la table et du lot, puis un filtre — un coût qui
/// suit le nombre de nœuds de la table, pas la taille du lot (session cœur
/// C++ : 169 ms au lieu de 6,5 pour 300 clés à 200 000 nœuds ; 348 ms au
/// lieu de 2,8 pour 300 liens à 20 000).
///
/// La recette, éprouvée sur le moteur par `EXPLAIN` : `item` voyage d'étape
/// en étape, et **chaque clé est extraite dans le `WITH` qui précède
/// immédiatement le `MATCH` ou le `MERGE` qui s'en sert**. Une clé extraite
/// plus tôt et passée par un second `WITH` fait refuser un `MERGE`
/// (« Cannot evaluate expression with type VARIABLE »). Deux nœuds dans le
/// même `MATCH` retombent sur le produit cartésien : un `WITH` entre eux.
///
/// Un seul endroit à changer le jour où le moteur saura mieux faire.
/// `e2e_plans_par_lot` lit le plan de chaque forme et refuse tout produit
/// cartésien.
pub(crate) fn cle_de_ligne(lies: &[&str], champ: &str, alias: &str) -> String {
    let mut garde: Vec<&str> = lies.to_vec();
    garde.push("item");
    format!("WITH {}, item.{champ} AS {alias}", garde.join(", "))
}

/// Un nœud à retrouver : sa variable, son étiquette (sans étiquette, toutes
/// les tables), la propriété clé, et le champ de `item` qui la porte.
pub(crate) struct ParCle<'a> {
    pub var: &'a str,
    pub label: Option<&'a str>,
    pub prop: &'a str,
    pub champ: &'a str,
}

/// `UNWIND $param AS item`, puis, pour chaque nœud dans l'ordre, l'extraction
/// de sa clé et son `verbe` (`MATCH` ou `MERGE`). Rend la requête jusqu'au
/// dernier motif inclus ; l'appelant ajoute `SET`, `RETURN`, `DELETE`…
pub(crate) fn unwind_par_cle(param: &str, noeuds: &[ParCle], verbe: &str) -> String {
    let mut q = format!("UNWIND ${param} AS item");
    let mut lies: Vec<&str> = Vec::new();
    for (i, n) in noeuds.iter().enumerate() {
        let alias = format!("__cle_{i}");
        let etiquette = n.label.map(|l| format!(":{l}")).unwrap_or_default();
        q.push(' ');
        q.push_str(&cle_de_ligne(&lies, n.champ, &alias));
        q.push_str(&format!(" {verbe} ({}{etiquette} {{{}: {alias}}})", n.var, n.prop));
        lies.push(n.var);
    }
    q
}

fn par_uuid<'a>(var: &'a str, label: Option<&'a str>, champ: &'a str) -> ParCle<'a> {
    ParCle { var, label, prop: "_uuid", champ }
}

pub trait SchemaDialect: Send + Sync {
    /// Backend name for diagnostics (e.g. "rag3db", "postgresql").
    fn name(&self) -> &'static str;

    /// Schema namespace for internal tables (e.g. "rag3weaver" in PostgreSQL).
    /// Returns `None` if the backend doesn't support schemas (rag3db).
    fn internal_schema(&self) -> Option<&'static str> { None }

    /// Qualify a table name with the internal schema if applicable.
    fn internal_table(&self, name: &str) -> String {
        match self.internal_schema() {
            Some(schema) => format!("{schema}.{name}"),
            None => name.to_string(),
        }
    }

    // ── Setup ────────────────────────────────────────────────────────────

    /// Statements to run before any DDL (e.g. CREATE SCHEMA, CREATE EXTENSION).
    fn setup_statements(&self) -> Vec<String> { vec![] }

    /// **Ce dialecte parle-t-il Cypher ?**
    ///
    /// Deux organes du catalogue sont encore écrits en Cypher **en dur**, sans
    /// passer par ce trait : le magasin de checkpoints
    /// (`CypherCheckpointStore`) et le magasin de blobs (`CypherBlobStore`).
    ///
    /// `initialize()` ne les monte d'office que là où ils peuvent tourner.
    /// Ailleurs, l'appelant fournit l'équivalent (`set_checkpoint_store`,
    /// `set_blob_store`) — et s'il ne le fait pas, le catalogue **le dit** par
    /// un `CatalogEvent::Warning` au lieu de démarrer amputé en silence.
    ///
    /// Vrai par défaut : c'était l'hypothèse implicite de tout le code
    /// existant, et un dialecte qui ne se prononce pas ne doit rien changer.
    fn speaks_cypher(&self) -> bool { true }

    /// Poser le nœud d'une cellule (`_Org`, `_Project`) s'il n'y est pas.
    ///
    /// Créer sans écraser : un `name` déjà renseigné ne doit pas retomber sur
    /// l'identifiant. Le paramètre sert **deux fois** — identité et nom par
    /// défaut — et c'est voulu.
    fn upsert_scope_node(&self, table: &str, id_param: &str) -> String;

    // ── Row identity ──────────────────────────────────────────────────

    /// Expression to get the stable row offset for a matched node.
    /// Used by sparse index as node_id.
    /// - rag3db: `OFFSET(id(n))` — builtin internal offset
    /// - pg: `n._row_id` — BIGSERIAL column added by create_table
    fn node_offset_expr(&self, alias: &str) -> String;

    /// Expression to get the internal node ID (for NodeIdCache).
    /// - rag3db: `ID(n)` — internal node ID struct
    /// - pg: `n._row_id` — same BIGSERIAL
    fn node_id_expr(&self, alias: &str) -> String;

    // ── Types ────────────────────────────────────────────────────────────

    /// Map a column type to the backend's type string.
    fn type_name(&self, ct: &ColumnType) -> String;

    /// Default value literal for a column type (used in ALTER TABLE ADD).
    fn default_value(&self, ct: &ColumnType) -> String;

    // ── Tables ───────────────────────────────────────────────────────────

    /// CREATE TABLE IF NOT EXISTS with a `_uuid` primary key.
    fn create_table(&self, name: &str, columns: &[ColumnDef]) -> String;

    /// CREATE TABLE IF NOT EXISTS for a relation (join table).
    /// `from_table` and `to_table` are the endpoint tables.
    /// `props` are optional relation properties.
    fn create_rel_table(
        &self,
        name: &str,
        from_table: &str,
        to_table: &str,
        props: &[ColumnDef],
    ) -> String;

    /// ALTER TABLE ADD column with the type's default value.
    fn alter_add_column(&self, table: &str, col: &ColumnDef) -> String {
        let def = self.default_value(&col.col_type);
        self.alter_add_column_default(table, col, &def)
    }

    /// ALTER TABLE ADD column with an explicit default literal (already quoted).
    fn alter_add_column_default(&self, table: &str, col: &ColumnDef, default_literal: &str) -> String;

    // ── Indexes ──────────────────────────────────────────────────────────

    /// Create a vector similarity index on an embedding column.
    fn create_vector_index(&self, table: &str, column: &str, index_name: &str) -> String;

    /// **Les index qu'une table de données réclame au-delà de sa clé primaire.**
    ///
    /// La clé primaire est `_uuid`, mais ce n'est pas par là qu'on cherche le
    /// plus souvent : une recherche BM25 ou sparse rend des **décalages de
    /// ligne**, et les résout par `_row_id`. Sans index dessus, chaque
    /// résolution balaie la table entière — sur le chemin le plus chaud du
    /// moteur.
    ///
    /// Vide par défaut : rag3db gère ses index lui-même.
    fn secondary_indexes(&self, _table: &str) -> Vec<String> { vec![] }

    /// Idem pour une table de relations.
    ///
    /// Sa clé primaire est `(from_uuid, to_uuid)`. Un index composite ne sert
    /// que les requêtes qui commencent par sa **première** colonne : traverser
    /// dans le sens entrant (`to_uuid` seul) ne peut pas s'en servir, et c'est
    /// exactement ce que fait toute résolution chunk → parent.
    fn relation_indexes(&self, _rel_table: &str) -> Vec<String> { vec![] }

    /// Idem pour la table des blobs d'index.
    ///
    /// Elle est listée par préfixe (`_key LIKE 'nom/%'`) à chaque ouverture
    /// d'index lucivy ou sparse — et un btree ordinaire **ne sert pas** un
    /// `LIKE 'préfixe%'` sous une collation autre que C. Vérifié : la base de
    /// test est en `en_US.utf8` et le plan est un balayage séquentiel.
    fn blob_store_indexes(&self, _table: &str) -> Vec<String> { vec![] }

    /// Index de recherche plein texte, s'il y en a un à poser côté base.
    ///
    /// Vide par défaut : un backend qui laisse lucivy s'en charger n'a rien à
    /// indexer. PostgreSQL, lui, pose un GIN trigramme par champ cherché.
    fn text_search_indexes(&self, _table: &str, _fields: &[String]) -> Vec<String> { vec![] }

    /// Drop a vector similarity index — le pendant du précédent, pour charger
    /// en masse puis reconstruire (doc 18).
    fn drop_vector_index(&self, table: &str, index_name: &str) -> String;

    // ── Internal tables ──────────────────────────────────────────────────

    /// CREATE TABLE for the `_catalog_meta` key-value store.
    fn create_meta_table(&self) -> String;

    /// CREATE TABLE for the `_index_blobs` blob store.
    fn create_blob_table(&self) -> String;

    // ── DML ──────────────────────────────────────────────────────────────

    /// Upsert a single meta key-value pair.
    fn upsert_meta(&self, key_param: &str, value_param: &str) -> String;

    /// Load meta entries matching a key prefix.
    fn load_meta_by_prefix(&self, prefix_param: &str) -> String;

    /// Batch upsert entities. Returns statement expecting `$items` parameter
    /// (UNWIND in rag3db, unnest in PostgreSQL).
    fn batch_upsert(&self, table: &str, columns: &[&str]) -> String;

    /// Delete entities by UUID list.
    fn batch_delete(&self, table: &str) -> String;

    /// Cascade delete entities + all relationships by UUID list.
    fn batch_cascade_delete(&self, table: &str) -> String;

    /// Batch upsert relation rows (from_uuid, to_uuid, optional props).
    /// Expects `$items` param as List<Map{from_uuid, to_uuid, ...}>.
    fn batch_link(&self, rel_table: &str, prop_columns: &[&str]) -> String;

    /// [`Self::batch_link`] **avec les étiquettes des deux bouts**, quand la
    /// relation les déclare. Sans étiquette, un `MATCH (a {_uuid: …})`
    /// balaie toutes les tables de nœuds pour chaque élément : 1,6 s pour
    /// 5 000 arêtes de symboles sur 30 fichiers (6 septembre 2026). Avec,
    /// c'est l'index de clé primaire de la table nommée. Le défaut ignore
    /// les étiquettes — SQL n'en a pas besoin, la table est déjà nommée.
    fn batch_link_labeled(&self, rel_table: &str, ends: Option<(&str, &str)>, prop_columns: &[&str]) -> String {
        let _ = ends;
        self.batch_link(rel_table, prop_columns)
    }

    /// **Charger des arêtes en masse depuis un CSV** (`from_uuid, to_uuid,
    /// props…`, sans en-tête), quand le moteur sait le faire. `None` : pas
    /// de chemin de masse, l'appelant reste sur [`Self::batch_link`].
    ///
    /// Mesuré le 6 septembre 2026 sur 200 000 arêtes : `UNWIND … MERGE`
    /// 158 s (1 263 arêtes/s), `COPY … FROM` 47 ms (4,2 M arêtes/s). C'est le
    /// chargement en masse du moteur, celui d'un index HNSW reconstruit sur
    /// table pleine (doc 18). Il ne dédoublonne pas : l'appelant le fait.
    fn copy_links_from_csv(&self, rel_table: &str, ends: (&str, &str), prop_columns: &[&str], path: &str) -> Option<String> {
        let _ = (rel_table, ends, prop_columns, path);
        None
    }

    /// Les paires `(from_uuid, to_uuid)` déjà posées pour ces sources —
    /// pour qu'un chargement en masse garde la sémantique de `MERGE`.
    /// Paramètre `$froms`.
    fn existing_links(&self, rel_table: &str, ends: (&str, &str)) -> String {
        let (from, to) = ends;
        format!(
            "UNWIND $froms AS f MATCH (a:{from} {{_uuid: f}})-[:{rel_table}]->(b:{to}) RETURN a._uuid, b._uuid"
        )
    }

    /// Le nombre d'arêtes d'une relation.
    fn count_links(&self, rel_table: &str) -> String {
        format!("MATCH ()-[r:{rel_table}]->() RETURN count(r)")
    }

    /// Batch update specific fields on entities matched by UUID.
    /// Expects `$items` param as List<Map{_uuid, field1, field2, ...}>.
    fn batch_update_fields(&self, table: &str, field_columns: &[&str]) -> String;

    /// Select entities by UUID list, returning specified fields.
    fn select_by_uuids(&self, table: &str, fields: &[&str]) -> String;

    /// Batch update fields and return updated UUIDs + extra columns.
    /// Used by EmbedNode to SET _embed_hash and get back the node offset.
    /// Expects `$items` param as List<Map{_uuid, ...}>.
    /// `set_columns`: columns to SET from item
    /// `return_columns`: columns to RETURN (e.g. ["item.uuid", "OFFSET(id(n))"])
    fn batch_update_returning(
        &self,
        table: &str,
        set_columns: &[&str],
        return_exprs: &[(&str, &str)],  // (expression, alias) pairs
    ) -> String;

    /// Cascade delete related entities by UUID and return count.
    /// E.g. delete all chunks for a parent UUID, return how many were deleted.
    /// `match_field`: the field to match on (e.g. "_parent_uuid")
    fn batch_cascade_delete_returning_count(
        &self,
        table: &str,
        match_field: &str,
    ) -> String;

    /// Delete rows matching a field value from a UUID list.
    /// E.g. delete chunks where _parent_uuid IN $uuids.
    fn batch_delete_by_field(&self, table: &str, field: &str) -> String;

    /// Delete relations between two entities by UUID pairs.
    /// Expects `$items` param as List<Map{from, to}>.
    fn batch_delete_relation(&self, rel_table: &str) -> String;

    /// Batch select with UNWIND: match by a field in each item, return specified fields.
    /// Expects `$items` param as List<Map{match_field: value}>.
    /// `match_field`: which item field to match on (e.g. "uuid" matched against "_uuid")
    fn batch_select(
        &self,
        table: &str,
        match_field: &str,
        table_match_col: &str,
        return_fields: &[&str],
    ) -> String;

    /// Batch SET a field to NULL for entities by UUID list.
    fn batch_set_null(&self, table: &str, field: &str) -> String;

    /// Join two tables via a relation table and return fields.
    /// E.g. MATCH (a:From)-[:REL]->(b:To {_uuid: uuid}) RETURN a.field, b.field
    fn join_select(
        &self,
        from_table: &str,
        rel_table: &str,
        to_table: &str,
        direction_forward: bool,
        match_col: &str,
        return_fields: &[&str],
    ) -> String;

    /// Select full entity rows by UUID list (all columns).
    /// rag3db: RETURN n (returns node as Map with all properties)
    /// pg: SELECT * FROM table WHERE _uuid = ANY($uuids)
    fn select_entity_all_by_uuids(&self, table: &str) -> String;

    /// Delete related entities via a relation, returning count per source UUID.
    /// rag3db: MATCH (e {_uuid})-[:REL]->(c:Target) DETACH DELETE c RETURN uuid, count
    /// pg: DELETE via subquery on rel table, GROUP BY source
    fn join_delete_returning_count(
        &self,
        source_table: &str,
        rel_table: &str,
        target_table: &str,
    ) -> String;

    /// Select all rows from a table, returning specified fields.
    /// `order_by`: optional column to ORDER BY (ascending).
    fn select_all(&self, table: &str, fields: &[&str], order_by: Option<&str>) -> String;

    /// Select rows whose `field` equals `$value`, returning `fields`.
    fn select_by_field(&self, table: &str, field: &str, fields: &[&str]) -> String;

    /// Check if an entity exists by UUID. Expects `$uuid` param.
    fn exists_by_uuid(&self, table: &str) -> String;

    /// Count rows in a table.
    fn count_rows(&self, table: &str) -> String;

    /// Tous les uuids d'une table — pour re-rendre une entité dérivée entière.
    fn select_all_uuids(&self, table: &str) -> String {
        format!("MATCH (n:{table}) RETURN n._uuid")
    }

    /// **Poser la dette de rendu** sur les lignes d'une entité dérivée dont la
    /// racine est dans `$uuids` : `_render_hash = ''`. C'est la dette au
    /// niveau donnée du pas B (doc du 7 septembre 2026) : si le processus
    /// meurt avant le drain, [`Self::select_derivees_a_rendre`] la retrouve.
    fn marquer_derivees_a_rendre(&self, derived_table: &str) -> String {
        format!(
            "UNWIND $uuids AS u \
             MATCH (d:{derived_table} {{_source_uuid: u}}) \
             SET d._render_hash = ''"
        )
    }

    /// **Marquer les lignes d'une session de synchronisation** :
    /// `_snapshot = $session` sur les lignes de `$uuids`.
    fn mark_snapshot_session(&self, table: &str) -> String {
        format!("UNWIND $uuids AS u MATCH (n:{table} {{_uuid: u}}) SET n._snapshot = $session, n._absent_since = NULL")
    }

    /// **La marque d'une écriture** : `_snapshot = $mark` sur `$uuids`, sauf
    /// là où un lot de la session les a déjà portées (`_snapshot = $session`)
    /// — la marque monte, elle ne descend jamais. Une ligne écrite est
    /// présente : sa marque d'absence part.
    fn mark_written_session(&self, table: &str) -> String {
        format!(
            "UNWIND $uuids AS u MATCH (n:{table} {{_uuid: u}}) \
             WHERE n._snapshot IS NULL OR n._snapshot <> $session \
             SET n._snapshot = $mark, n._absent_since = NULL"
        )
    }

    /// Une colonne entière à NULL, sur toute la table (une migration).
    fn set_column_null(&self, table: &str, field: &str) -> String {
        format!("MATCH (n:{table}) SET n.{field} = NULL")
    }

    /// **Poser la marque d'absence** sur `$uuids`, à `$since`, sauf là où elle
    /// est déjà : c'est la *première* absence constatée qu'elle garde.
    fn mark_absent_since(&self, table: &str) -> String {
        format!(
            "UNWIND $uuids AS u MATCH (n:{table} {{_uuid: u}}) \
             WHERE n._absent_since IS NULL OR n._absent_since = 0 SET n._absent_since = $since"
        )
    }

    /// **Les lignes d'un périmètre de synchronisation** : chaque champ du
    /// périmètre égal à son paramètre `$p0`, `$p1`… (aucun : la table
    /// entière). Rend `_uuid`, `_snapshot`, puis `extra` dans l'ordre.
    fn select_snapshot_scope(&self, table: &str, scope: &[&str], extra: &[&str]) -> String {
        let filtre = scope.iter().enumerate()
            .map(|(i, f)| format!("n.{f} = $p{i}"))
            .collect::<Vec<_>>()
            .join(" AND ");
        let ou = if filtre.is_empty() { String::new() } else { format!(" WHERE {filtre}") };
        let mut rend = vec!["n._uuid".to_string(), "n._snapshot".to_string()];
        rend.extend(extra.iter().map(|f| format!("n.{f}")));
        format!("MATCH (n:{table}){ou} RETURN {}", rend.join(", "))
    }

    // ── La mise de côté avant purge (`_snapshot_aside`) ──────────────────
    //
    // Une ligne par ligne retirée, clé `{entité}:{uuid}`. La ligne (`_row`) et
    // les vecteurs denses de ses chunks (`_chunks`) sont du JSON. Une copie
    // consommée ou purgée est **vidée par SET**, jamais supprimée : rag3db ne
    // récupère pas la place d'une ligne supprimée, il récupère celle d'un SET.

    /// La table de la mise de côté.
    fn create_aside_table(&self) -> String {
        "CREATE NODE TABLE IF NOT EXISTS _snapshot_aside(\n    \
         _key STRING,\n    \
         _entity STRING,\n    \
         _uuid STRING,\n    \
         _content_hash STRING,\n    \
         _row STRING,\n    \
         _chunks STRING,\n    \
         _session STRING,\n    \
         _removed_at INT64,\n    \
         PRIMARY KEY(_key)\n)"
            .into()
    }

    /// Poser des copies : `$items` porte `key`, `entity`, `uuid`, `hash`,
    /// `row`, `chunks`, `session`, `at`.
    fn upsert_aside(&self) -> String {
        format!(
            "{} SET a._entity = item.entity, a._uuid = item.uuid, a._content_hash = item.hash, a._row = item.row, \
             a._chunks = item.chunks, a._session = item.session, a._removed_at = item.at",
            unwind_par_cle("items", &[ParCle { var: "a", label: Some("_snapshot_aside"), prop: "_key", champ: "key" }], "MERGE")
        )
    }

    /// Les copies vivantes de `$keys` : `_uuid`, `_content_hash`, `_row`,
    /// `_chunks`, `_session`.
    fn select_aside(&self) -> String {
        "UNWIND $keys AS k MATCH (a:_snapshot_aside {_key: k}) WHERE a._row <> '' \
         RETURN a._uuid, a._content_hash, a._row, a._chunks, a._session"
            .into()
    }

    /// Les copies vivantes qu'une session a posées pour `$entity`.
    fn select_aside_by_session(&self) -> String {
        "MATCH (a:_snapshot_aside) WHERE a._entity = $entity AND a._session = $session AND a._row <> '' \
         RETURN a._uuid, a._content_hash, a._row, a._chunks, a._session"
            .into()
    }

    /// Vider les copies de `$keys` (consommées ou périmées).
    fn clear_aside(&self) -> String {
        "UNWIND $keys AS k MATCH (a:_snapshot_aside {_key: k}) SET a._row = '', a._chunks = ''".into()
    }

    /// **La purge, bornée** : vider au plus `limit` copies de `$entity`
    /// posées avant `$before`. Rend le nombre vidé.
    fn purge_aside_before(&self, limit: usize) -> String {
        format!(
            "MATCH (a:_snapshot_aside) WHERE a._entity = $entity AND a._row <> '' AND a._removed_at < $before \
             WITH a LIMIT {limit} SET a._row = '', a._chunks = '' RETURN count(a)"
        )
    }

    /// Les vecteurs denses des chunks des lignes `$uuids`, pour une copie :
    /// `_uuid`, `_parent_uuid`, `_text_hash`, le marqueur, le vecteur. `None`
    /// quand le dialecte ne sait pas relire un vecteur : la copie part sans,
    /// et la ligne qui revient est réembarquée.
    fn select_chunk_vectors(&self, chunk_table: &str, column: &str, marker: &str) -> Option<String> {
        Some(format!(
            "MATCH (c:{chunk_table}) WHERE c._parent_uuid IN $uuids \
             RETURN c._uuid, c._parent_uuid, c._text_hash, c.{marker}, c.{column}"
        ))
    }

    /// **Rendre l'état d'avant** à des lignes qu'une fin a fait passer par
    /// une transition, et retirer leur marque d'absence : `$items` porte
    /// `uuid` et `state`. Écrit sans la garde de la machine à états — une
    /// annulation n'est pas une transition.
    fn revert_lifecycle_state(&self, table: &str, field: &str) -> String {
        format!(
            "{} SET n.{field} = item.state, n._absent_since = NULL",
            unwind_par_cle("items", &[par_uuid("n", Some(table), "uuid")], "MATCH")
        )
    }

    /// **Combien de liens de la relation `rel` touchent ces lignes**, dans
    /// les deux sens — ce que `DETACH DELETE` emportera de cette relation.
    /// `None` quand le dialecte ne sait pas le dire.
    fn count_relations_of(&self, table: &str, rel: &str) -> Option<String> {
        Some(format!("UNWIND $uuids AS u MATCH (n:{table} {{_uuid: u}})-[r:{rel}]-() RETURN count(DISTINCT r)"))
    }

    /// **Les dérivées en dette de rendu** : `_render_hash` nul ou vide. Rend
    /// `_source_uuid` (la racine à re-rendre), borné.
    fn select_derivees_a_rendre(&self, derived_table: &str, limite: usize) -> String {
        format!(
            "MATCH (d:{derived_table}) \
             WHERE d._render_hash IS NULL OR d._render_hash = '' \
             RETURN d._source_uuid LIMIT {limite}"
        )
    }

    /// **Les racines sans ligne dérivée** : posées sans que leur dérivée ait
    /// été rendue (un processus mort entre les deux, une base migrée). Rend
    /// `_uuid` de la racine, borné.
    fn select_racines_sans_derivee(&self, root_table: &str, derived_table: &str, rel_table: &str, limite: usize) -> String {
        format!(
            "MATCH (r:{root_table}) \
             OPTIONAL MATCH (d:{derived_table})-[:{rel_table}]->(r) \
             WITH r, d WHERE d._uuid IS NULL \
             RETURN r._uuid LIMIT {limite}"
        )
    }

    /// Supprime une table (nœud ou relation) — pour une migration qui retire
    /// ce qu'un schéma d'avant avait posé. Une table absente fait échouer
    /// l'instruction sur le moteur Cypher : l'appelant ignore cette erreur.
    fn drop_table(&self, table: &str) -> String {
        format!("DROP TABLE {table}")
    }

    /// **Charger des lignes en masse depuis un CSV** (sans en-tête, une
    /// cellule par colonne de `columns`, dans cet ordre), quand le moteur
    /// sait le faire. `None` : pas de chemin de masse, l'appelant reste sur
    /// [`Self::batch_upsert`].
    ///
    /// C'est le jumeau de [`Self::copy_links_from_csv`] pour les nœuds. Il
    /// ne fusionne pas : une clé déjà présente fait refuser tout le fichier,
    /// l'appelant réserve donc ce chemin à une table vide ou à des clés
    /// qu'il sait absentes. Les colonnes non listées prennent leur défaut.
    fn copy_nodes_from_csv(&self, table: &str, columns: &[&str], path: &str) -> Option<String> {
        let _ = (table, columns, path);
        None
    }

    /// Le moteur a-t-il un chargement en masse ([`Self::copy_nodes_from_csv`]) ?
    fn supports_copy_from(&self) -> bool {
        false
    }

    /// **Les identifiants internes de lignes désignées par uuid** — ce que
    /// `batch_upsert` rend en écrivant, relu après un chargement en masse
    /// qui ne rend rien. Paramètre `$uuids` ; colonnes `_uuid`, identifiant
    /// (au format de [`Self::node_id_expr`]).
    fn select_node_ids(&self, table: &str) -> String {
        let id = self.node_id_expr("n");
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (n:{table} {{_uuid: uuid}}) \
             RETURN n._uuid, {id}"
        )
    }

    /// **Combien de chunks doivent encore un embarquement**, pour un marqueur
    /// donné (`_embed_hash` pour le dense, `_sparse_hash` pour le sparse).
    ///
    /// « Doit encore » = marqueur nul **ou vide** : le vide est la valeur qu'un
    /// chunk porte à sa naissance, le nul celle qu'un `undo` y remet.
    /// **Ou périmé** : un marqueur qui n'est plus le `_text_hash` de la ligne
    /// dit un vecteur calculé sur un texte d'avant. C'est la même définition
    /// que celle d'`EmbedNode` (« à jour » = marqueur égal au hachage du
    /// texte) ; sans elle, une édition posée sans embarquement laissait
    /// l'ancien vecteur sous le nouveau texte, et personne ne le devait
    /// (`e2e_invariant_des_vecteurs`, 4 octobre 2026).
    ///
    /// C'est la dette rendue interrogeable. Elle ne vit pas en mémoire — elle
    /// est dans la base, donc elle survit à un processus qui meurt, et une
    /// passe de rattrapage la retrouve telle quelle.
    fn count_marqueur_manquant(&self, table: &str, marqueur: &str) -> String;

    /// **Les chunks qui doivent encore un embarquement**, bornés.
    ///
    /// Rend `_uuid, _text, _text_hash` — de quoi reconstruire le
    /// travail d'embarquement sans rien avoir gardé en mémoire. C'est la
    /// contrepartie de [`Self::count_marqueur_manquant`] : l'un dit combien,
    /// l'autre dit lesquels.
    ///
    /// La borne n'est pas une commodité : une passe de rattrapage doit pouvoir
    /// avancer par morceaux sur une base qui en doit des millions, sans tenir
    /// le tout en mémoire ni monopoliser la carte.
    fn select_chunks_sans_marqueur(&self, table: &str, marqueur: &str, limite: usize) -> String;

    /// Copie une colonne dans une autre, sur toute la table. Sert à une
    /// migration qui pose un marqueur là où l'invariant tenait déjà.
    fn copier_colonne(&self, table: &str, de: &str, vers: &str) -> String;

    /// **Les entités dont les chunks sont en retard** : `_chunked_hash` nul,
    /// vide ou différent de `_content_hash`. Rend `_uuid`, borné.
    fn select_entites_a_redecouper(&self, table: &str, limite: usize) -> String;

    /// **Réclame des chunks en retard, et les rend** — en une seule
    /// instruction, pour que deux processus qui rattrapent ne prennent pas
    /// les mêmes.
    ///
    /// Sélectionne jusqu'à `limite` chunks dont `marqueur` est vide **et**
    /// dont `_embed_claim` est libre : vide, périmée (`< $perime`, un
    /// horodatage zéro-rembourré suivi de `|`), ou **la nôtre** (`ENDS WITH
    /// $mien`, soit `|écrivain`) — reprendre sa propre réclamation est
    /// légitime, une passe interrompue ne doit pas s'attendre elle-même. Pose
    /// `$reclamation` dessus, et rend `_uuid, _text, _text_hash`.
    ///
    /// Paramètres : `$reclamation`, `$perime`, `$mien`.
    fn reclamer_chunks_sans_marqueur(&self, table: &str, marqueur: &str, limite: usize) -> String;

    // ── Search resolution ────────────────────────────────────────────

    /// Resolve chunk UUIDs to chunk metadata + parent entity data in one query.
    /// Used by resolve_vector_chunks_with_dialect for chunk→parent join with optional source_refs.
    ///
    /// Returns columns: chunk_uuid, parent_uuid, c_text, c_idx, c_sline, c_eline, c_start, c_end,
    /// [c_source_entity, c_source_uuid, c_source_field (= _parent_field) if has_source_refs],
    /// [parent_field1, parent_field2, ...]
    fn resolve_chunks_with_parent(
        &self,
        chunk_table: &str,
        parent_table: &str,
        rel_table: &str,
        rel_forward: bool,
        has_source_refs: bool,
        parent_fields: &[&str],
    ) -> String;

    // ── Embed operations ─────────────────────────────────────────────

    /// Les deux marqueurs d'embarquement d'un chunk : `_uuid`, le marqueur
    /// dense du modèle courant (`marker`) et `_sparse_hash`.
    ///
    /// **Trois colonnes et aucun filtre** depuis le schéma v3. Elle n'en
    /// rendait que deux, et ne rendait la ligne que si le marqueur dense n'était
    /// pas nul — ce qui cachait exactement le cas qui compte maintenant : un
    /// chunk embarqué en dense et pas en sparse. Le tri se fait chez l'appelant,
    /// qui seul sait quel signal l'intéresse.
    ///
    /// `marker` est celui du modèle courant — `_embed_hash` pour un stockage
    /// d'avant, `_embed_hash__{slug}` sinon. Un index porte plusieurs modèles,
    /// et chacun juge la fraîcheur par le sien (7 septembre 2026).
    fn embed_check_hashes(&self, table: &str, marker: &str) -> String;

    /// SET la colonne de vecteurs **et** son marqueur, sur les lignes citées.
    /// Les deux viennent du même `VectorStorage` : ils vont ensemble ou pas du
    /// tout — poser un vecteur sans son marqueur, c'est un chunk réputé en
    /// retard qu'on réembarquerait ; poser le marqueur sans le vecteur, c'est
    /// un chunk réputé fait qui ne répondra jamais.
    fn embed_set(&self, table: &str, embedding_col: &str, marker: &str) -> String;

    /// **Parmi les lignes citées, celles qui portent déjà un vecteur** — ou
    /// `None` si ce moteur remplace un vecteur par un autre sans rien perdre.
    ///
    /// N'existe que pour le contournement de
    /// `record_nodes::write_vectors` : l'index HNSW de rag3db perd des lignes
    /// quand un `SET` remplace un vecteur. PostgreSQL n'a pas ce défaut.
    fn embed_carrying_vector(&self, _table: &str, _embedding_col: &str) -> Option<String> {
        None
    }

    /// Remet à NULL le vecteur des lignes citées, et vide leur marqueur : la
    /// ligne redevient une dette honnête. Même contournement.
    fn embed_clear(&self, table: &str, embedding_col: &str, marker: &str) -> String;

    /// SET le marqueur dense et rend item.uuid + décalage de ligne (pour le
    /// handle sparse).
    fn embed_set_hash_returning_offset(&self, table: &str, marker: &str) -> String;

    /// Get node offset for entities (no SET, just return item.uuid + offset).
    fn embed_get_offset(&self, table: &str) -> String;

    /// Les offsets des chunks des lignes `$uuids` (`_parent_uuid`) — la clé
    /// de leurs entrées dans l'index creux, à retirer avant de les supprimer.
    fn select_chunk_offsets(&self, chunk_table: &str) -> String;

    // ── KB operations ────────────────────────────────────────────────

    /// Gather fields from entities by item.uuid, returning item.uuid + entity fields.
    /// `return_entity_fields`: entity column names to return
    fn kb_gather_fields(&self, table: &str, return_entity_fields: &[&str]) -> String;

    /// Gather content from related entities via traversal.
    /// Returns entity fields from the related side.
    fn kb_gather_content(
        &self,
        title_entity: &str,
        rel: &str,
        content_entity: &str,
        direction_forward: bool,
        return_fields: &[&str],
    ) -> String;

    /// MERGE ON CREATE SET all fields / ON MATCH SET update fields.
    /// `all_fields`: fields to SET on create, `update_fields`: fields to SET on match
    fn kb_upsert_index(
        &self,
        index_table: &str,
        all_fields: &[&str],
        update_fields: &[&str],
    ) -> String;

    // ── Filter expressions ──────────────────────────────────────────

    /// Generate a JOIN/MATCH clause for cross-entity filter traversal.
    /// `result_alias`: alias of the source node (e.g. "n")
    /// `rel_name`: relation table name
    /// `target_alias`: alias for the target entity (e.g. "e1")
    /// `target_entity`: target entity table name
    /// `forward`: true if result→target, false if target→result
    fn filter_join_clause(
        &self,
        result_alias: &str,
        rel_name: &str,
        target_alias: &str,
        target_entity: &str,
        forward: bool,
    ) -> String;

    /// Le magasin de checkpoints que ce dialecte sait servir, s'il en a un.
    ///
    /// **Le dialecte le dit, le catalogue ne le devine pas.** Sans cette
    /// méthode, `initialize()` choisissait sur `speaks_cypher()` puis sur le
    /// nom du dialecte — donc un backend neuf repartait sans reprise après
    /// incident sans que son auteur ait rien à décider, ni rien à voir.
    ///
    /// `None` = ce backend n'en a pas encore ; `initialize()` le **dit** au
    /// lieu de démarrer amputé en silence.
    fn nouveau_magasin_de_checkpoints(
        &self,
        conn: std::sync::Arc<dyn crate::connection::DbConnection>,
        dossier: std::path::PathBuf,
    ) -> Option<std::sync::Arc<dyn crate::dataflow::checkpoint::CheckpointStore>> {
        // Le défaut est le Cypher, comme partout ici.
        Some(std::sync::Arc::new(
            crate::dataflow::checkpoint_store::CypherCheckpointStore::with_directory(conn, dossier),
        ))
    }

    /// **Résoudre des décalages de parents, avec tous leurs chunks**, en une
    /// requête.
    ///
    /// C'est l'étape qui suit un hit lucivy : l'index rend un décalage de la
    /// table parente, il faut l'entité *et* ses chunks pour rattacher les spans
    /// de surlignage. La forme des lignes est fixée — `_offset`, `_uuid`, les
    /// champs demandés, puis neuf colonnes de chunk (plus deux si la cible
    /// porte des références de source) — parce que c'est elle que
    /// `resolve_and_enrich_chunked` lit ensuite, position par position.
    ///
    /// **Elle était écrite en Cypher en dur.** lucivy est un index Rust, donc
    /// utilisable sur n'importe quel backend ; sa résolution, elle, ne parlait
    /// que rag3db. `MoteurTexte::Lucivy` était donc impossible sur PostgreSQL —
    /// et comme rien ne l'empruntait, personne ne le savait.
    fn resolve_parents_with_chunks(
        &self,
        entity: &str,
        chunk_table: &str,
        chunk_rel: &str,
        chunk_rel_fwd: bool,
        offsets: &[u64],
        return_fields: &[String],
        has_source_refs: bool,
    ) -> String {
        let offset_list = offsets
            .iter()
            .map(|o| o.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let mut cols: Vec<String> = vec![
            "OFFSET(id(n)) AS _offset".to_string(),
            "n._uuid AS _uuid".to_string(),
        ];
        for f in return_fields {
            cols.push(format!("n.{f} AS {f}"));
        }
        cols.extend(colonnes_de_chunk("c", has_source_refs));
        let jointure = if chunk_rel_fwd {
            format!("OPTIONAL MATCH (n)-[:{chunk_rel}]->(c:{chunk_table})")
        } else {
            format!("OPTIONAL MATCH (n)<-[:{chunk_rel}]-(c:{chunk_table})")
        };
        format!(
            "MATCH (n:{entity}) WHERE OFFSET(id(n)) IN [{offset_list}] \
             {jointure} \
             RETURN {}",
            cols.join(", ")
        )
    }

    /// Joindre un **chunk à son parent**, pour qu'un filtre puisse porter sur
    /// les champs du parent alors que la recherche court sur les chunks.
    ///
    /// C'est le seul chaînon de filtrage qui était encore écrit en Cypher en
    /// dur dans le catalogue : sur PostgreSQL, le `WHERE` compilé référençait
    /// un `p` que la requête ne déclarait jamais, et le chemin vectoriel
    /// **filtré** échouait sur « missing FROM-clause entry for table "p" ».
    ///
    /// Le défaut par défaut est le Cypher, comme partout ailleurs ici.
    fn chunk_parent_join(
        &self,
        chunk_alias: &str,
        parent_alias: &str,
        parent_entity: &str,
    ) -> String {
        format!(
            "MATCH ({chunk_alias})-[:{parent_entity}_CHUNKED_FROM]->\
             ({parent_alias}:{parent_entity})"
        )
    }

    /// Expression for array/list length. E.g. `size(prop)` or `cardinality(prop)`.
    fn filter_size_expr(&self, prop: &str) -> String;

    /// starts_with filter expression.
    fn filter_starts_with(&self, prop: &str, param: &str) -> String;

    /// contains filter expression.
    fn filter_contains(&self, prop: &str, param: &str) -> String;

    /// Optional scalar membership lowering for engines whose projected graphs cannot bind lambdas.
    fn filter_list_scalar_contains(&self, _prop: &str, _param: &str) -> Option<String> { None }

    /// List has any match: at least one element of `prop` is in `param` list.
    fn filter_list_any_match(&self, prop: &str, param: &str) -> String;

    /// List has all: all elements of `param` list are contained in `prop`.
    fn filter_list_all(&self, prop: &str, param: &str) -> String;

    /// List has none: no element of `prop` is in `param` list.
    fn filter_list_none(&self, prop: &str, param: &str) -> String;

    /// Build the full filter resolution query: given parsed filter parts,
    /// return a query that resolves matching row offsets.
    /// `table`: the entity table to resolve offsets from
    /// `alias`: alias for that table (e.g. "n" or "idx")
    /// `join_clauses`: cross-entity join/match clauses
    /// `where_clause`: combined WHERE expression
    /// `join_from`: optional (from_alias, from_table, rel, to_alias) for KB indirection
    fn filter_resolve_offsets(
        &self,
        table: &str,
        alias: &str,
        join_clauses: &[String],
        where_clause: &str,
        join_from: Option<(&str, &str, &str)>,
    ) -> String;
}

// ─── Rag3db Dialect ──────────────────────────────────────────────────────────

/// Cypher DDL/DML for rag3db (Kuzu fork).
pub struct Rag3dbDialect;

impl SchemaDialect for Rag3dbDialect {
    fn upsert_scope_node(&self, table: &str, id_param: &str) -> String {
        format!("MERGE (n:{table} {{_uuid: ${id_param}}}) ON CREATE SET n.name = ${id_param}")
    }

    fn name(&self) -> &'static str {
        "rag3db"
    }

    fn node_offset_expr(&self, alias: &str) -> String {
        format!("OFFSET(id({alias}))")
    }

    fn node_id_expr(&self, alias: &str) -> String {
        format!("ID({alias})")
    }

    fn type_name(&self, ct: &ColumnType) -> String {
        match ct {
            ColumnType::Text => "STRING".into(),
            ColumnType::Int64 => "INT64".into(),
            ColumnType::Double => "DOUBLE".into(),
            ColumnType::Boolean => "BOOLEAN".into(),
            ColumnType::Timestamp => "TIMESTAMP".into(),
            ColumnType::Blob => "BLOB".into(),
            ColumnType::Vector(dim) => format!("FLOAT[{dim}]"),
            ColumnType::List(item) => format!("{}[]", self.type_name(item)),
            ColumnType::Struct(fields) => format!("STRUCT({})", fields.iter().map(|(k,v)| format!("`{}` {}", k.replace('`', "``"), self.type_name(v))).collect::<Vec<_>>().join(", ")),
        }
    }

    fn default_value(&self, ct: &ColumnType) -> String {
        match ct {
            ColumnType::Text => "''".into(),
            ColumnType::Int64 => "0".into(),
            ColumnType::Double => "0.0".into(),
            ColumnType::Boolean => "false".into(),
            ColumnType::Timestamp => "'1970-01-01 00:00:00'".into(),
            ColumnType::Blob => "''".into(),
            ColumnType::Vector(_) => "[]".into(),
            ColumnType::List(_) | ColumnType::Struct(_) => "NULL".into(),
        }
    }

    fn create_table(&self, name: &str, columns: &[ColumnDef]) -> String {
        let col_defs: Vec<String> = columns
            .iter()
            .map(|c| format!("{} {}", c.name, self.type_name(&c.col_type)))
            .collect();
        format!(
            "CREATE NODE TABLE IF NOT EXISTS {name}(\n    {},\n    PRIMARY KEY(_uuid)\n)",
            col_defs.join(",\n    ")
        )
    }

    fn create_rel_table(
        &self,
        name: &str,
        from_table: &str,
        to_table: &str,
        props: &[ColumnDef],
    ) -> String {
        let prop_str = if props.is_empty() {
            String::new()
        } else {
            let defs: Vec<String> = props
                .iter()
                .map(|c| format!("{} {}", c.name, self.type_name(&c.col_type)))
                .collect();
            format!(", {}", defs.join(", "))
        };
        format!("CREATE REL TABLE IF NOT EXISTS {name}(FROM {from_table} TO {to_table}{prop_str})")
    }

    fn alter_add_column_default(&self, table: &str, col: &ColumnDef, default_literal: &str) -> String {
        let typ = self.type_name(&col.col_type);
        // **Un défaut NULL ne s'écrit pas** : c'est déjà le défaut, et une
        // colonne ajoutée avec `DEFAULT NULL` explicite fait refuser tout
        // COPY qui l'omet (« Trying to create a vector with ANY type » — le
        // NULL n'y prend jamais le type de la colonne ; 3 octobre 2026). Les
        // morceaux d'une première indexation en plein texte omettent leur
        // colonne de vecteurs : leur COPY retombait sur le MERGE ligne à ligne.
        if default_literal.eq_ignore_ascii_case("NULL") {
            return format!("ALTER TABLE {table} ADD {} {typ}", col.name);
        }
        format!("ALTER TABLE {table} ADD {} {typ} DEFAULT {default_literal}", col.name)
    }

    fn create_vector_index(&self, table: &str, column: &str, index_name: &str) -> String {
        format!(
            "CALL CREATE_VECTOR_INDEX('{table}', '{index_name}', '{column}', metric := 'cosine', skip_if_exists := true)"
        )
    }

    fn drop_vector_index(&self, table: &str, index_name: &str) -> String {
        format!("CALL DROP_VECTOR_INDEX('{table}', '{index_name}', skip_if_not_exists := true)")
    }

    fn create_meta_table(&self) -> String {
        "CREATE NODE TABLE IF NOT EXISTS _catalog_meta(\n    \
         _key STRING,\n    \
         _value STRING,\n    \
         PRIMARY KEY(_key)\n)"
            .into()
    }

    fn create_blob_table(&self) -> String {
        "CREATE NODE TABLE IF NOT EXISTS _index_blobs(\n    \
         _key STRING,\n    \
         _data BLOB,\n    \
         PRIMARY KEY(_key)\n)"
            .into()
    }

    fn upsert_meta(&self, key_param: &str, value_param: &str) -> String {
        format!("MERGE (m:_catalog_meta {{_key: ${key_param}}}) SET m._value = ${value_param}")
    }

    fn load_meta_by_prefix(&self, prefix_param: &str) -> String {
        format!(
            "MATCH (m:_catalog_meta) WHERE m._key STARTS WITH ${prefix_param} RETURN m._key, m._value"
        )
    }

    fn batch_upsert(&self, table: &str, columns: &[&str]) -> String {
        let set_clause: Vec<String> = columns
            .iter()
            .filter(|c| **c != "_uuid")
            .map(|c| format!("n.{c} = item.{c}"))
            .collect();
        let id_expr = self.node_id_expr("n");
        format!(
            "{} SET {} RETURN {id_expr}, item._uuid",
            unwind_par_cle("items", &[par_uuid("n", Some(table), "_uuid")], "MERGE"),
            set_clause.join(", ")
        )
    }

    fn batch_delete(&self, table: &str) -> String {
        format!("UNWIND $uuids AS uuid MATCH (n:{table} {{_uuid: uuid}}) DELETE n")
    }

    fn batch_cascade_delete(&self, table: &str) -> String {
        format!("UNWIND $uuids AS uuid MATCH (n:{table} {{_uuid: uuid}}) DETACH DELETE n")
    }

    fn batch_link(&self, rel_table: &str, prop_columns: &[&str]) -> String {
        let prop_set = if prop_columns.is_empty() {
            String::new()
        } else {
            let assigns: Vec<String> = prop_columns.iter()
                .map(|c| format!("r.{c} = item.{c}"))
                .collect();
            format!(" SET {}", assigns.join(", "))
        };
        format!(
            "{} MERGE (a)-[r:{rel_table}]->(b){prop_set}",
            unwind_par_cle("items", &[par_uuid("a", None, "from_uuid"), par_uuid("b", None, "to_uuid")], "MATCH")
        )
    }

    fn copy_links_from_csv(&self, rel_table: &str, ends: (&str, &str), prop_columns: &[&str], path: &str) -> Option<String> {
        let (from, to) = ends;
        // **Les colonnes nommées**, dans l'ordre où le CSV les écrit : sans
        // elles, le moteur remplit les propriétés dans l'ordre de la table —
        // celui des `ALTER` successifs —, et deux colonnes texte s'échangent
        // sans erreur (3 octobre 2026, `e2e_copy_liens`). Les deux premières
        // colonnes du fichier restent les bouts.
        let colonnes = if prop_columns.is_empty() { String::new() } else { format!(" ({})", prop_columns.join(", ")) };
        // **Les options de lecture du COPY des nœuds** : les cellules sont
        // écrites par le même `cellule_csv`. Sans elles, le renifleur du
        // moteur décidait seul des guillemets — `"other,type"` comptait pour
        // deux colonnes (COPY refusé, repli par lots : 56 s et 145 s sur le
        // dépôt entier), ou une cellule changeait de valeur sans erreur, et
        // un saut de ligne restait échappé (3 octobre 2026, `e2e_copy_liens`).
        Some(format!(
            "COPY {rel_table}{colonnes} FROM '{path}' (from='{from}', to='{to}', escaped_newlines=true, auto_detect=false, null_strings=['{CSV_NULL}'])"
        ))
    }

    fn copy_nodes_from_csv(&self, table: &str, columns: &[&str], path: &str) -> Option<String> {
        // Sans en-tête (le défaut du moteur), les listes entre crochets dans
        // une cellule entre guillemets, le guillemet doublé pour s'échapper.
        // `escaped_newlines` : les sauts de ligne entre guillemets sont écrits
        // `\n` / `\r` (et la barre `\\`), ce qui garde le lecteur **parallèle**
        // — le physique le refusait (« Quoted newlines are not supported »),
        // et le séquentiel coûtait 1,4 s pour 20 132 chunks (7 septembre 2026,
        // option ajoutée au moteur par la session du cœur C++, d2b48ea68).
        // `auto_detect=false` : on écrit le CSV nous-mêmes, une barre invalide
        // doit être une erreur, pas un contournement du renifleur.
        // Et le NULL est un mot convenu, pas la cellule vide : le moteur lit
        // `""` comme un NULL, or une chaîne vide en est une (`_embed_hash` à
        // la naissance d'un chunk) — la comparer à NULL la ferait réécrire.
        Some(format!(
            "COPY {table} ({}) FROM '{path}' (escaped_newlines=true, auto_detect=false, null_strings=['{CSV_NULL}'])",
            columns.join(", ")
        ))
    }

    fn supports_copy_from(&self) -> bool {
        true
    }

    fn batch_link_labeled(&self, rel_table: &str, ends: Option<(&str, &str)>, prop_columns: &[&str]) -> String {
        let Some((from, to)) = ends else { return self.batch_link(rel_table, prop_columns) };
        let prop_set = if prop_columns.is_empty() {
            String::new()
        } else {
            let assigns: Vec<String> = prop_columns.iter().map(|c| format!("r.{c} = item.{c}")).collect();
            format!(" SET {}", assigns.join(", "))
        };
        // Mesuré le 6 septembre 2026 sur 225 000 arêtes : CREATE au lieu de
        // MERGE ne change rien (98 s dans les deux cas) — le coût n'est pas
        // la vérification d'existence, c'est l'insertion elle-même.
        format!(
            "{} MERGE (a)-[r:{rel_table}]->(b){prop_set}",
            unwind_par_cle("items", &[par_uuid("a", Some(from), "from_uuid"), par_uuid("b", Some(to), "to_uuid")], "MATCH")
        )
    }

    fn batch_update_fields(&self, table: &str, field_columns: &[&str]) -> String {
        let assigns: Vec<String> = field_columns.iter()
            .map(|c| format!("n.{c} = item.{c}"))
            .collect();
        format!(
            "{} SET {}",
            unwind_par_cle("items", &[par_uuid("n", Some(table), "_uuid")], "MATCH"),
            assigns.join(", ")
        )
    }

    fn select_by_uuids(&self, table: &str, fields: &[&str]) -> String {
        let return_cols = fields.iter()
            .map(|f| format!("n.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (n:{table} {{_uuid: uuid}}) \
             RETURN {return_cols}"
        )
    }

    fn batch_update_returning(
        &self,
        table: &str,
        set_columns: &[&str],
        return_exprs: &[(&str, &str)],
    ) -> String {
        let assigns: Vec<String> = set_columns.iter()
            .map(|c| format!("n.{c} = item.{c}"))
            .collect();
        let returns: Vec<String> = return_exprs.iter()
            .map(|(expr, alias)| {
                if expr == alias { expr.to_string() } else { format!("{expr} AS {alias}") }
            })
            .collect();
        format!(
            "{} SET {} RETURN {}",
            unwind_par_cle("items", &[par_uuid("n", Some(table), "_uuid")], "MATCH"),
            assigns.join(", "),
            returns.join(", "),
        )
    }

    fn batch_cascade_delete_returning_count(
        &self,
        table: &str,
        match_field: &str,
    ) -> String {
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (c:{table} {{{match_field}: uuid}}) \
             DETACH DELETE c RETURN uuid, count(c) AS cnt"
        )
    }

    fn batch_delete_by_field(&self, table: &str, field: &str) -> String {
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (n:{table} {{{field}: uuid}}) \
             DETACH DELETE n"
        )
    }

    fn batch_delete_relation(&self, rel_table: &str) -> String {
        format!(
            "{} {} MATCH (a)-[r:{rel_table}]->(b {{_uuid: __cle_1}}) DELETE r",
            unwind_par_cle("items", &[par_uuid("a", None, "from")], "MATCH"),
            cle_de_ligne(&["a"], "to", "__cle_1")
        )
    }

    fn batch_select(
        &self,
        table: &str,
        match_field: &str,
        table_match_col: &str,
        return_fields: &[&str],
    ) -> String {
        let returns = return_fields.iter()
            .map(|f| format!("n.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{} RETURN {returns}",
            unwind_par_cle("items", &[ParCle { var: "n", label: Some(table), prop: table_match_col, champ: match_field }], "MATCH")
        )
    }

    fn batch_set_null(&self, table: &str, field: &str) -> String {
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (n:{table} {{_uuid: uuid}}) \
             SET n.{field} = NULL"
        )
    }

    fn join_select(
        &self,
        from_table: &str,
        rel_table: &str,
        to_table: &str,
        direction_forward: bool,
        _match_col: &str,
        return_fields: &[&str],
    ) -> String {
        let returns = return_fields.iter()
            .map(|f| {
                if f.contains('.') { f.to_string() } else { format!("n.{f}") }
            })
            .collect::<Vec<_>>()
            .join(", ");
        if direction_forward {
            format!(
                "UNWIND $uuids AS uuid \
                 MATCH (n:{from_table} {{_uuid: uuid}})-[:{rel_table}]->(m:{to_table}) \
                 RETURN {returns}"
            )
        } else {
            format!(
                "UNWIND $uuids AS uuid \
                 MATCH (n:{from_table} {{_uuid: uuid}})<-[:{rel_table}]-(m:{to_table}) \
                 RETURN {returns}"
            )
        }
    }

    fn select_entity_all_by_uuids(&self, table: &str) -> String {
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (n:{table} {{_uuid: uuid}}) \
             RETURN n"
        )
    }

    fn join_delete_returning_count(
        &self,
        source_table: &str,
        rel_table: &str,
        target_table: &str,
    ) -> String {
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (e:{source_table} {{_uuid: uuid}})-[:{rel_table}]->(c:{target_table}) \
             DETACH DELETE c RETURN uuid, count(c) AS cnt"
        )
    }

    fn select_all(&self, table: &str, fields: &[&str], order_by: Option<&str>) -> String {
        let returns = fields.iter()
            .map(|f| format!("n.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        match order_by {
            Some(col) => format!("MATCH (n:{table}) RETURN {returns} ORDER BY n.{col}"),
            None => format!("MATCH (n:{table}) RETURN {returns}"),
        }
    }

    fn select_by_field(&self, table: &str, field: &str, fields: &[&str]) -> String {
        let returns = fields.iter()
            .map(|f| format!("n.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("MATCH (n:{table}) WHERE n.{field} = $value RETURN {returns}")
    }

    fn exists_by_uuid(&self, table: &str) -> String {
        format!("MATCH (n:{table} {{_uuid: $uuid}}) RETURN count(n) AS cnt")
    }

    fn count_rows(&self, table: &str) -> String {
        format!("MATCH (n:{table}) RETURN count(n) AS cnt")
    }

    fn count_marqueur_manquant(&self, table: &str, marqueur: &str) -> String {
        format!(
            "MATCH (n:{table}) WHERE n.{marqueur} IS NULL OR n.{marqueur} = '' OR n.{marqueur} <> n._text_hash \
             RETURN count(n) AS cnt"
        )
    }

    fn select_chunks_sans_marqueur(&self, table: &str, marqueur: &str, limite: usize) -> String {
        format!(
            "MATCH (n:{table}) WHERE n.{marqueur} IS NULL OR n.{marqueur} = '' OR n.{marqueur} <> n._text_hash \
             RETURN n._uuid, n._text, n._text_hash LIMIT {limite}"
        )
    }

    fn copier_colonne(&self, table: &str, de: &str, vers: &str) -> String {
        format!("MATCH (n:{table}) SET n.{vers} = n.{de}")
    }

    fn select_entites_a_redecouper(&self, table: &str, limite: usize) -> String {
        format!(
            "MATCH (n:{table}) \
             WHERE n._chunked_hash IS NULL OR n._chunked_hash = '' \
                OR n._chunked_hash <> n._content_hash \
             RETURN n._uuid LIMIT {limite}"
        )
    }

    fn reclamer_chunks_sans_marqueur(&self, table: &str, marqueur: &str, limite: usize) -> String {
        format!(
            "MATCH (n:{table}) \
             WHERE (n.{marqueur} IS NULL OR n.{marqueur} = '' OR n.{marqueur} <> n._text_hash) \
               AND (n._embed_claim IS NULL OR n._embed_claim = '' \
                    OR n._embed_claim < $perime OR n._embed_claim ENDS WITH $mien) \
             WITH n LIMIT {limite} \
             SET n._embed_claim = $reclamation \
             RETURN n._uuid, n._text, n._text_hash"
        )
    }

    fn resolve_chunks_with_parent(
        &self,
        chunk_table: &str,
        parent_table: &str,
        rel_table: &str,
        rel_forward: bool,
        has_source_refs: bool,
        parent_fields: &[&str],
    ) -> String {
        let mut return_cols = vec![
            "c._uuid AS chunk_uuid".to_string(),
            "p._uuid AS parent_uuid".to_string(),
            "c._text AS c_text".to_string(),
            "c._index AS c_idx".to_string(),
            "c._start_line AS c_sline".to_string(),
            "c._end_line AS c_eline".to_string(),
            "c._start_char AS c_start".to_string(),
            "c._end_char AS c_end".to_string(),
        ];
        if has_source_refs {
            return_cols.push("c._source_entity AS c_source_entity".to_string());
            return_cols.push("c._source_uuid AS c_source_uuid".to_string());
            // Le champ d'origine d'un chunk est son `_parent_field` : les chunks
            // d'une dérivée ne sont plus attribués par contributrice.
            return_cols.push("c._parent_field AS c_source_field".to_string());
        }
        for f in parent_fields {
            return_cols.push(format!("p.{f} AS {f}"));
        }

        let rel_match = if rel_forward {
            // rel_forward = true means parent→chunk (e.g. HAS_CHUNK)
            format!("MATCH (p:{parent_table})-[:{rel_table}]->(c)")
        } else {
            // rel_forward = false means chunk→parent (e.g. CHUNKED_FROM)
            format!("MATCH (c)-[:{rel_table}]->(p:{parent_table})")
        };

        format!(
            "MATCH (c:{chunk_table}) WHERE c._uuid IN $uuids \
             {rel_match} \
             RETURN {}",
            return_cols.join(", ")
        )
    }

    fn embed_check_hashes(&self, table: &str, marker: &str) -> String {
        format!(
            "{} RETURN n._uuid, n.{marker}, n._sparse_hash",
            unwind_par_cle("items", &[par_uuid("n", Some(table), "uuid")], "MATCH")
        )
    }

    fn embed_set(&self, table: &str, embedding_col: &str, marker: &str) -> String {
        format!(
            "{} SET n.{embedding_col} = item.emb, n.{marker} = item.hash",
            unwind_par_cle("items", &[par_uuid("n", Some(table), "uuid")], "MATCH")
        )
    }

    fn embed_carrying_vector(&self, table: &str, embedding_col: &str) -> Option<String> {
        Some(format!(
            "{} WHERE n.{embedding_col} IS NOT NULL RETURN n._uuid",
            unwind_par_cle("items", &[par_uuid("n", Some(table), "uuid")], "MATCH")
        ))
    }

    fn embed_clear(&self, table: &str, embedding_col: &str, marker: &str) -> String {
        format!(
            "{} SET n.{embedding_col} = NULL, n.{marker} = ''",
            unwind_par_cle("items", &[par_uuid("n", Some(table), "uuid")], "MATCH")
        )
    }

    fn embed_set_hash_returning_offset(&self, table: &str, marker: &str) -> String {
        let offset = self.node_offset_expr("n");
        format!(
            "{} SET n.{marker} = item.hash RETURN item.uuid, {offset} AS offset",
            unwind_par_cle("items", &[par_uuid("n", Some(table), "uuid")], "MATCH")
        )
    }

    fn embed_get_offset(&self, table: &str) -> String {
        let offset = self.node_offset_expr("n");
        format!(
            "{} RETURN item.uuid, {offset} AS offset",
            unwind_par_cle("items", &[par_uuid("n", Some(table), "uuid")], "MATCH")
        )
    }

    fn select_chunk_offsets(&self, chunk_table: &str) -> String {
        let offset = self.node_offset_expr("c");
        format!("MATCH (c:{chunk_table}) WHERE c._parent_uuid IN $uuids RETURN {offset}")
    }

    fn kb_gather_fields(&self, table: &str, return_entity_fields: &[&str]) -> String {
        let returns = std::iter::once("item.uuid AS _source_uuid".to_string())
            .chain(return_entity_fields.iter().map(|f| format!("e.{f} AS {f}")))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{} RETURN {returns}", unwind_par_cle("items", &[par_uuid("e", Some(table), "uuid")], "MATCH"))
    }

    fn kb_gather_content(
        &self,
        title_entity: &str,
        rel: &str,
        content_entity: &str,
        direction_forward: bool,
        return_fields: &[&str],
    ) -> String {
        let returns = std::iter::once("item.uuid AS _source_uuid".to_string())
            .chain(return_fields.iter().map(|f| format!("c.{f} AS {f}")))
            .collect::<Vec<_>>()
            .join(", ");
        // Le titre par sa clé, puis le contenu par l'arête depuis le titre lié.
        let titre = unwind_par_cle("items", &[par_uuid("t", Some(title_entity), "uuid")], "MATCH");
        if direction_forward {
            format!("{titre} WITH t, item MATCH (t)-[:{rel}]->(c:{content_entity}) RETURN {returns}")
        } else {
            format!("{titre} WITH t, item MATCH (t)<-[:{rel}]-(c:{content_entity}) RETURN {returns}")
        }
    }

    fn kb_upsert_index(
        &self,
        index_table: &str,
        all_fields: &[&str],
        update_fields: &[&str],
    ) -> String {
        let create_set = all_fields.iter()
            .map(|f| format!("idx.{f} = item.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        let match_set = update_fields.iter()
            .map(|f| format!("idx.{f} = item.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{} ON CREATE SET {create_set} ON MATCH SET {match_set}",
            unwind_par_cle("items", &[par_uuid("idx", Some(index_table), "uuid")], "MERGE")
        )
    }

    fn filter_join_clause(
        &self,
        result_alias: &str,
        rel_name: &str,
        target_alias: &str,
        target_entity: &str,
        forward: bool,
    ) -> String {
        if forward {
            format!("MATCH ({result_alias})-[:{rel_name}]->({target_alias}:{target_entity})")
        } else {
            format!("MATCH ({result_alias})<-[:{rel_name}]-({target_alias}:{target_entity})")
        }
    }

    fn filter_size_expr(&self, prop: &str) -> String {
        format!("size({prop})")
    }

    fn filter_starts_with(&self, prop: &str, param: &str) -> String {
        format!("starts_with({prop}, ${param})")
    }

    fn filter_contains(&self, prop: &str, param: &str) -> String {
        format!("contains({prop}, ${param})")
    }

    fn filter_list_scalar_contains(&self, prop: &str, param: &str) -> Option<String> {
        Some(format!("list_contains({prop}, ${param})"))
    }

    fn filter_list_any_match(&self, prop: &str, param: &str) -> String {
        format!("any(v IN {prop} WHERE list_contains(${param}, v))")
    }

    fn filter_list_all(&self, prop: &str, param: &str) -> String {
        format!("all(v IN ${param} WHERE list_contains({prop}, v))")
    }

    fn filter_list_none(&self, prop: &str, param: &str) -> String {
        format!("NOT any(v IN {prop} WHERE list_contains(${param}, v))")
    }

    fn filter_resolve_offsets(
        &self,
        table: &str,
        alias: &str,
        join_clauses: &[String],
        where_clause: &str,
        join_from: Option<(&str, &str, &str)>,
    ) -> String {
        let offset_expr = self.node_offset_expr(alias);
        match join_from {
            Some((from_alias, from_table, rel)) => {
                let joins = if join_clauses.is_empty() {
                    String::new()
                } else {
                    format!(" {}", join_clauses.join(" "))
                };
                format!(
                    "MATCH ({from_alias}:{from_table})-[:{rel}]->({alias}:{table}){joins} \
                     WHERE {where_clause} RETURN {offset_expr}"
                )
            }
            None => {
                let joins = if join_clauses.is_empty() {
                    String::new()
                } else {
                    format!(" {}", join_clauses.join(" "))
                };
                format!(
                    "MATCH ({alias}:{table}){joins} \
                     WHERE {where_clause} RETURN {offset_expr}"
                )
            }
        }
    }
}

// ─── PostgreSQL Dialect ──────────────────────────────────────────────────────

/// SQL DDL/DML for PostgreSQL with pgvector (Supabase-compatible).
pub struct PostgresDialect;

impl SchemaDialect for PostgresDialect {
    fn name(&self) -> &'static str {
        "postgresql"
    }

    fn node_offset_expr(&self, alias: &str) -> String {
        format!("{alias}._row_id")
    }

    fn node_id_expr(&self, alias: &str) -> String {
        format!("{alias}._row_id")
    }

    fn internal_schema(&self) -> Option<&'static str> {
        Some("rag3weaver")
    }

    fn setup_statements(&self) -> Vec<String> {
        vec![
            // **Le schéma d'abord.** Une extension créée sans `SCHEMA` atterrit
            // dans le premier schéma du `search_path`, lequel commence par
            // `"$user"` : l'emplacement dépendrait donc du **nom du rôle**.
            // Sur ce poste elles tombaient dans `rag3weaver` par coïncidence,
            // et une fonction qui les cherchait dans `public` échouait. On le
            // décide au lieu de le subir.
            "CREATE SCHEMA IF NOT EXISTS rag3weaver".into(),
            "CREATE EXTENSION IF NOT EXISTS vector SCHEMA rag3weaver".into(),
            // Trigrammes : le plein texte servi par la base elle-même, sans
            // second corpus à stocker à côté.
            "CREATE EXTENSION IF NOT EXISTS pg_trgm SCHEMA rag3weaver".into(),
            // Accents : « cafe » doit trouver « café ». Sur un corpus français
            // ce n'est pas un raffinement, c'est la moitié des requêtes.
            "CREATE EXTENSION IF NOT EXISTS unaccent SCHEMA rag3weaver".into(),
            // **`unaccent()` est STABLE, pas IMMUTABLE** — donc inutilisable
            // dans une expression d'index. Le contournement est un enrobage
            // déclaré immuable, avec le dictionnaire **nommé explicitement** :
            // c'est la forme à un argument qui est instable, parce qu'elle
            // dépend du `search_path` pour trouver son dictionnaire.
            //
            // La même expression doit servir des **deux** côtés — à l'index et
            // à la requête — sinon le planificateur ne les apparie pas et
            // l'index n'est jamais utilisé.
            "CREATE OR REPLACE FUNCTION rag3weaver.sans_accents(text) \
             RETURNS text LANGUAGE sql IMMUTABLE PARALLEL SAFE STRICT \
             AS $$ SELECT rag3weaver.unaccent('rag3weaver.unaccent'::regdictionary, $1) $$"
                .into(),
        ]
    }

    fn speaks_cypher(&self) -> bool { false }

    // ── Les lots passent par JSON ────────────────────────────────────────
    //
    // Un lot arrive en un seul paramètre `$items` portant une liste de lignes.
    // Le code d'origine écrivait `unnest($items) AS v(colonnes)` : `unnest` ne
    // fait pas ça. Il déplie **un** tableau en **une** colonne ; il ne
    // reconstruit pas des lignes à plusieurs champs. Aucune des treize
    // requêtes en lot n'a jamais pu s'exécuter.
    //
    // Deux formes le font, selon ce qu'on sait des types :
    //
    // - `jsonb_to_recordset($items::text::jsonb) AS v(a TEXT, b TEXT)` quand la
    //   requête déclare déjà ses colonnes et leurs types ;
    // - `jsonb_populate_recordset(NULL::{table}, ...)` quand elle ne les
    //   déclare pas — **le type de la table les fournit**, ce qui évite au
    //   dialecte de tenir un catalogue de types en double.
    //
    // Le `::text::jsonb` n'est pas une coquetterie. Écrit `$1::jsonb`,
    // PostgreSQL infère le paramètre comme `jsonb`, et le pilote refuse d'y
    // sérialiser une `String` Rust (`error serializing parameter 0`). En
    // passant par `text`, le paramètre est du texte — ce qu'on envoie
    // réellement — et la base fait la conversion.

    fn upsert_scope_node(&self, table: &str, id_param: &str) -> String {
        format!(
            "INSERT INTO {table} (_uuid, name) VALUES (${id_param}, ${id_param}) \
             ON CONFLICT (_uuid) DO NOTHING"
        )
    }

    fn type_name(&self, ct: &ColumnType) -> String {
        match ct {
            ColumnType::Text => "TEXT".into(),
            ColumnType::Int64 => "BIGINT".into(),
            ColumnType::Double => "DOUBLE PRECISION".into(),
            ColumnType::Boolean => "BOOLEAN".into(),
            ColumnType::Timestamp => "TIMESTAMPTZ".into(),
            ColumnType::Blob => "BYTEA".into(),
            ColumnType::Vector(dim) => format!("vector({dim})"),
            ColumnType::List(_) | ColumnType::Struct(_) => "JSONB".into(),
        }
    }

    fn default_value(&self, ct: &ColumnType) -> String {
        match ct {
            ColumnType::Text => "''".into(),
            ColumnType::Int64 => "0".into(),
            ColumnType::Double => "0.0".into(),
            ColumnType::Boolean => "false".into(),
            ColumnType::Timestamp => "'1970-01-01T00:00:00Z'".into(),
            ColumnType::Blob => "''::bytea".into(),
            ColumnType::Vector(dim) => format!("(ARRAY_FILL(0, ARRAY[{dim}]))::vector({dim})"),
            ColumnType::List(_) | ColumnType::Struct(_) => "NULL".into(),
        }
    }

    fn create_table(&self, name: &str, columns: &[ColumnDef]) -> String {
        // _row_id BIGSERIAL first — stable row offset for sparse index + NodeIdCache
        let mut col_defs = vec!["_row_id BIGSERIAL".to_string()];
        col_defs.extend(columns.iter()
            .map(|c| format!("{} {}", c.name, self.type_name(&c.col_type))));
        format!(
            "CREATE TABLE IF NOT EXISTS {name} (\n    {},\n    PRIMARY KEY (_uuid)\n)",
            col_defs.join(",\n    ")
        )
    }

    fn create_rel_table(
        &self,
        name: &str,
        _from_table: &str,
        _to_table: &str,
        props: &[ColumnDef],
    ) -> String {
        let mut col_defs = vec![
            "from_uuid TEXT NOT NULL".to_string(),
            "to_uuid TEXT NOT NULL".to_string(),
        ];
        for c in props {
            col_defs.push(format!("{} {}", c.name, self.type_name(&c.col_type)));
        }
        format!(
            "CREATE TABLE IF NOT EXISTS {name} (\n    {},\n    PRIMARY KEY (from_uuid, to_uuid)\n)",
            col_defs.join(",\n    ")
        )
    }

    fn alter_add_column_default(&self, table: &str, col: &ColumnDef, default_literal: &str) -> String {
        let typ = self.type_name(&col.col_type);
        format!("ALTER TABLE {table} ADD COLUMN IF NOT EXISTS {} {typ} DEFAULT {default_literal}", col.name)
    }

    fn secondary_indexes(&self, table: &str) -> Vec<String> {
        let court = table.replace('.', "_");
        vec![
            // Le chemin chaud : `resolve_offsets` fait
            // `WHERE _row_id = ANY(...)` à chaque résolution de recherche.
            format!("CREATE INDEX IF NOT EXISTS {court}_row_id_idx ON {table} (_row_id)"),
            // Chunk → parent, et tout `batch_delete_by_field` sur ce champ.
            // `IF NOT EXISTS` ne suffit pas ici : la colonne n'existe que sur
            // les tables de chunks, donc l'échec est attendu et absorbé par
            // l'appelant.
            format!("CREATE INDEX IF NOT EXISTS {court}_parent_idx ON {table} (_parent_uuid)"),
            // La cellule : présente sur toute table de données, filtrée dès
            // qu'un scope est posé.
            format!("CREATE INDEX IF NOT EXISTS {court}_cellule_idx ON {table} (_org, _project)"),
        ]
    }

    fn text_search_indexes(&self, table: &str, fields: &[String]) -> Vec<String> {
        let court = table.replace('.', "_");
        fields
            .iter()
            .map(|f| {
                format!(
                    "CREATE INDEX IF NOT EXISTS {court}_{f}_trgm_idx \
                     ON {table} USING gin (rag3weaver.sans_accents({f}) gin_trgm_ops)"
                )
            })
            .collect()
    }

    fn blob_store_indexes(&self, table: &str) -> Vec<String> {
        let court = table.replace('.', "_");
        vec![format!(
            "CREATE INDEX IF NOT EXISTS {court}_prefixe_idx ON {table} (_key text_pattern_ops)"
        )]
    }

    fn relation_indexes(&self, rel_table: &str) -> Vec<String> {
        let court = rel_table.replace('.', "_");
        // `from_uuid` est déjà servi par le préfixe de la clé primaire ;
        // `to_uuid` seul ne l'est pas.
        vec![format!(
            "CREATE INDEX IF NOT EXISTS {court}_to_idx ON {rel_table} (to_uuid)"
        )]
    }

    fn create_vector_index(&self, table: &str, column: &str, index_name: &str) -> String {
        format!(
            "CREATE INDEX IF NOT EXISTS {index_name} ON {table} USING hnsw ({column} vector_cosine_ops)"
        )
    }

    fn drop_vector_index(&self, _table: &str, index_name: &str) -> String {
        format!("DROP INDEX IF EXISTS {index_name}")
    }

    fn create_meta_table(&self) -> String {
        let t = self.internal_table("_catalog_meta");
        format!(
            "CREATE TABLE IF NOT EXISTS {t} (\n    \
             _key TEXT PRIMARY KEY,\n    \
             _value TEXT\n)"
        )
    }

    fn create_blob_table(&self) -> String {
        let t = self.internal_table("_index_blobs");
        format!(
            "CREATE TABLE IF NOT EXISTS {t} (\n    \
             _key TEXT PRIMARY KEY,\n    \
             _data BYTEA\n)"
        )
    }

    fn upsert_meta(&self, key_param: &str, value_param: &str) -> String {
        let t = self.internal_table("_catalog_meta");
        format!(
            "INSERT INTO {t} (_key, _value) VALUES (${key_param}, ${value_param}) \
             ON CONFLICT (_key) DO UPDATE SET _value = EXCLUDED._value"
        )
    }

    fn load_meta_by_prefix(&self, prefix_param: &str) -> String {
        let t = self.internal_table("_catalog_meta");
        format!("SELECT _key, _value FROM {t} WHERE _key LIKE ${prefix_param} || '%'")
    }

    fn batch_upsert(&self, table: &str, columns: &[&str]) -> String {
        let col_list = columns.join(", ");
        let val_refs = columns.iter().map(|c| format!("v.{c}")).collect::<Vec<_>>().join(", ");
        let update_set: Vec<String> = columns
            .iter()
            .filter(|c| **c != "_uuid")
            .map(|c| format!("{c} = EXCLUDED.{c}"))
            .collect();
        let id_expr = self.node_id_expr(table);
        format!(
            "INSERT INTO {table} ({col_list}) \
             SELECT {val_refs} FROM jsonb_populate_recordset(NULL::{table}, $items::text::jsonb) AS v \
             ON CONFLICT (_uuid) DO UPDATE SET {} \
             RETURNING {id_expr}, _uuid",
            update_set.join(", ")
        )
    }

    fn batch_delete(&self, table: &str) -> String {
        format!("DELETE FROM {table} WHERE _uuid = ANY($uuids)")
    }

    fn batch_cascade_delete(&self, table: &str) -> String {
        // PostgreSQL: delete from all relation tables that reference this entity, then delete entity.
        // For simplicity, rely on ON DELETE CASCADE if FKs are set up.
        // Otherwise, the caller must delete relations first.
        format!("DELETE FROM {table} WHERE _uuid = ANY($uuids)")
    }

    fn batch_link(&self, rel_table: &str, prop_columns: &[&str]) -> String {
        let mut cols = vec!["from_uuid", "to_uuid"];
        cols.extend(prop_columns.iter().copied());
        let col_list = cols.join(", ");
        let val_refs = cols.iter().map(|c| format!("v.{c}")).collect::<Vec<_>>().join(", ");
        let conflict_update = if prop_columns.is_empty() {
            "DO NOTHING".to_string()
        } else {
            let assigns: Vec<String> = prop_columns.iter()
                .map(|c| format!("{c} = EXCLUDED.{c}"))
                .collect();
            format!("DO UPDATE SET {}", assigns.join(", "))
        };
        format!(
            "INSERT INTO {rel_table} ({col_list}) \
             SELECT {val_refs} FROM jsonb_populate_recordset(NULL::{rel_table}, $items::text::jsonb) AS v \
             ON CONFLICT (from_uuid, to_uuid) {conflict_update}"
        )
    }

    fn batch_update_fields(&self, table: &str, field_columns: &[&str]) -> String {
        let assigns: Vec<String> = field_columns.iter()
            .map(|c| format!("{c} = v.{c}"))
            .collect();
        // Plus de liste de colonnes à construire : le type de la table les
        // fournit toutes, avec leurs types. Ce que le JSON ne porte pas devient
        // NULL, et on ne référence que ce qu'on assigne.
        format!(
            "UPDATE {table} SET {} \
             FROM jsonb_populate_recordset(NULL::{table}, $items::text::jsonb) AS v \
             WHERE {table}._uuid = v._uuid",
            assigns.join(", ")
        )
    }

    fn select_by_uuids(&self, table: &str, fields: &[&str]) -> String {
        let col_list = fields.join(", ");
        format!("SELECT {col_list} FROM {table} WHERE _uuid = ANY($uuids)")
    }

    fn select_node_ids(&self, table: &str) -> String {
        format!("SELECT _uuid, _row_id FROM {table} WHERE _uuid = ANY($uuids)")
    }

    fn select_all_uuids(&self, table: &str) -> String {
        format!("SELECT _uuid FROM {table}")
    }

    fn drop_table(&self, table: &str) -> String {
        format!("DROP TABLE IF EXISTS {table} CASCADE")
    }

    fn marquer_derivees_a_rendre(&self, derived_table: &str) -> String {
        format!("UPDATE {derived_table} SET _render_hash = '' WHERE _source_uuid = ANY($uuids)")
    }

    fn mark_snapshot_session(&self, table: &str) -> String {
        format!("UPDATE {table} SET _snapshot = $session, _absent_since = NULL WHERE _uuid = ANY($uuids)")
    }

    fn mark_absent_since(&self, table: &str) -> String {
        format!("UPDATE {table} SET _absent_since = $since WHERE _uuid = ANY($uuids) AND (_absent_since IS NULL OR _absent_since = 0)")
    }

    fn mark_written_session(&self, table: &str) -> String {
        format!(
            "UPDATE {table} SET _snapshot = $mark, _absent_since = NULL \
             WHERE _uuid = ANY($uuids) AND (_snapshot IS NULL OR _snapshot <> $session)"
        )
    }

    fn set_column_null(&self, table: &str, field: &str) -> String {
        format!("UPDATE {table} SET {field} = NULL")
    }

    fn select_snapshot_scope(&self, table: &str, scope: &[&str], extra: &[&str]) -> String {
        let filtre = scope.iter().enumerate()
            .map(|(i, f)| format!("{f} = $p{i}"))
            .collect::<Vec<_>>()
            .join(" AND ");
        let ou = if filtre.is_empty() { String::new() } else { format!(" WHERE {filtre}") };
        let mut rend = vec!["_uuid".to_string(), "_snapshot".to_string()];
        rend.extend(extra.iter().map(|f| f.to_string()));
        format!("SELECT {} FROM {table}{ou}", rend.join(", "))
    }

    fn count_relations_of(&self, _table: &str, _rel: &str) -> Option<String> {
        // Les relations sont des tables : il faudrait les énumérer toutes.
        None
    }

    fn create_aside_table(&self) -> String {
        let t = self.internal_table("_snapshot_aside");
        format!(
            "CREATE TABLE IF NOT EXISTS {t} (\n    \
             _key TEXT PRIMARY KEY,\n    \
             _entity TEXT,\n    \
             _uuid TEXT,\n    \
             _content_hash TEXT,\n    \
             _row TEXT,\n    \
             _chunks TEXT,\n    \
             _session TEXT,\n    \
             _removed_at BIGINT\n)"
        )
    }

    fn upsert_aside(&self) -> String {
        let t = self.internal_table("_snapshot_aside");
        format!(
            "INSERT INTO {t} (_key, _entity, _uuid, _content_hash, _row, _chunks, _session, _removed_at) \
             SELECT v.key, v.entity, v.uuid, v.hash, v.row, v.chunks, v.session, v.at \
             FROM jsonb_to_recordset($items::text::jsonb) AS v(key TEXT, entity TEXT, uuid TEXT, hash TEXT, row TEXT, chunks TEXT, session TEXT, at BIGINT) \
             ON CONFLICT (_key) DO UPDATE SET _entity = EXCLUDED._entity, _uuid = EXCLUDED._uuid, \
             _content_hash = EXCLUDED._content_hash, _row = EXCLUDED._row, _chunks = EXCLUDED._chunks, \
             _session = EXCLUDED._session, _removed_at = EXCLUDED._removed_at"
        )
    }

    fn select_aside(&self) -> String {
        let t = self.internal_table("_snapshot_aside");
        format!("SELECT _uuid, _content_hash, _row, _chunks, _session FROM {t} WHERE _key = ANY($keys) AND _row <> ''")
    }

    fn select_aside_by_session(&self) -> String {
        let t = self.internal_table("_snapshot_aside");
        format!(
            "SELECT _uuid, _content_hash, _row, _chunks, _session FROM {t} \
             WHERE _entity = $entity AND _session = $session AND _row <> ''"
        )
    }

    fn clear_aside(&self) -> String {
        let t = self.internal_table("_snapshot_aside");
        format!("UPDATE {t} SET _row = '', _chunks = '' WHERE _key = ANY($keys)")
    }

    fn purge_aside_before(&self, limit: usize) -> String {
        let t = self.internal_table("_snapshot_aside");
        format!(
            "WITH p AS (UPDATE {t} SET _row = '', _chunks = '' WHERE _key IN \
             (SELECT _key FROM {t} WHERE _entity = $entity AND _row <> '' AND _removed_at < $before LIMIT {limit}) \
             RETURNING 1) SELECT count(*) FROM p"
        )
    }

    fn select_chunk_vectors(&self, _chunk_table: &str, _column: &str, _marker: &str) -> Option<String> {
        // Relire une colonne `vector` en liste de flottants n'est pas encore
        // éprouvé sur ce dialecte : la copie part sans vecteurs, la ligne qui
        // revient est réembarquée. Moins bien, jamais faux.
        None
    }

    fn revert_lifecycle_state(&self, table: &str, field: &str) -> String {
        format!(
            "UPDATE {table} SET {field} = v.state, _absent_since = NULL \
             FROM jsonb_to_recordset($items::text::jsonb) AS v(uuid TEXT, state TEXT) \
             WHERE {table}._uuid = v.uuid"
        )
    }

    fn select_derivees_a_rendre(&self, derived_table: &str, limite: usize) -> String {
        format!(
            "SELECT _source_uuid FROM {derived_table} \
             WHERE _render_hash IS NULL OR _render_hash = '' LIMIT {limite}"
        )
    }

    fn select_racines_sans_derivee(&self, root_table: &str, derived_table: &str, _rel_table: &str, limite: usize) -> String {
        format!(
            "SELECT r._uuid FROM {root_table} r \
             LEFT JOIN {derived_table} d ON d._source_uuid = r._uuid \
             WHERE d._uuid IS NULL LIMIT {limite}"
        )
    }

    fn batch_update_returning(
        &self,
        table: &str,
        set_columns: &[&str],
        return_exprs: &[(&str, &str)],
    ) -> String {
        let assigns: Vec<String> = set_columns.iter()
            .map(|c| format!("{c} = v.{c}"))
            .collect();
        // Map rag3db expressions to PostgreSQL equivalents
        let returns: Vec<String> = return_exprs.iter()
            .map(|(expr, alias)| {
                // `OFFSET(id(n))` devient l'expression de décalage du dialecte,
                // et pas `{table}.id` écrit en dur : la colonne s'appelle
                // `_row_id` ici, et le nom en dur désignait une colonne qui
                // n'existe dans aucune table qu'on crée.
                let pg_expr = expr
                    .replace("OFFSET(id(n))", &self.node_offset_expr(table))
                    .replace("item.", "v.");
                if pg_expr == *alias { pg_expr } else { format!("{pg_expr} AS {alias}") }
            })
            .collect();
        format!(
            "UPDATE {table} SET {} \
             FROM jsonb_populate_recordset(NULL::{table}, $items::text::jsonb) AS v \
             WHERE {table}._uuid = v._uuid \
             RETURNING {}",
            assigns.join(", "),
            returns.join(", "),
        )
    }

    fn batch_cascade_delete_returning_count(
        &self,
        table: &str,
        match_field: &str,
    ) -> String {
        format!(
            "WITH deleted AS (\
             DELETE FROM {table} WHERE {match_field} = ANY($uuids) RETURNING {match_field}\
             ) SELECT {match_field} AS uuid, count(*) AS cnt FROM deleted GROUP BY {match_field}"
        )
    }

    fn batch_delete_by_field(&self, table: &str, field: &str) -> String {
        format!("DELETE FROM {table} WHERE {field} = ANY($uuids)")
    }

    fn batch_delete_relation(&self, rel_table: &str) -> String {
        format!(
            "DELETE FROM {rel_table} \
             USING jsonb_to_recordset($items::text::jsonb) AS v(from_uuid TEXT, to_uuid TEXT) \
             WHERE {rel_table}.from_uuid = v.from_uuid AND {rel_table}.to_uuid = v.to_uuid"
        )
    }

    fn batch_select(
        &self,
        table: &str,
        match_field: &str,
        table_match_col: &str,
        return_fields: &[&str],
    ) -> String {
        let cols = return_fields.join(", ");
        format!(
            "SELECT {cols} FROM {table} \
             INNER JOIN jsonb_to_recordset($items::text::jsonb) AS v({match_field} TEXT) \
             ON {table}.{table_match_col} = v.{match_field}"
        )
    }

    fn batch_set_null(&self, table: &str, field: &str) -> String {
        format!("UPDATE {table} SET {field} = NULL WHERE _uuid = ANY($uuids)")
    }

    fn join_select(
        &self,
        from_table: &str,
        rel_table: &str,
        to_table: &str,
        _direction_forward: bool,
        _match_col: &str,
        return_fields: &[&str],
    ) -> String {
        let cols = return_fields.join(", ");
        format!(
            "SELECT {cols} FROM {from_table} \
             INNER JOIN {rel_table} ON {rel_table}.from_uuid = {from_table}._uuid \
             INNER JOIN {to_table} ON {rel_table}.to_uuid = {to_table}._uuid \
             WHERE {from_table}._uuid = ANY($uuids)"
        )
    }

    fn select_entity_all_by_uuids(&self, table: &str) -> String {
        format!("SELECT * FROM {table} WHERE _uuid = ANY($uuids)")
    }

    fn join_delete_returning_count(
        &self,
        _source_table: &str,
        rel_table: &str,
        target_table: &str,
    ) -> String {
        format!(
            "WITH deleted AS (\
             DELETE FROM {target_table} \
             WHERE _uuid IN (SELECT to_uuid FROM {rel_table} WHERE from_uuid = ANY($uuids)) \
             RETURNING (SELECT from_uuid FROM {rel_table} WHERE to_uuid = {target_table}._uuid LIMIT 1) AS uuid\
             ) SELECT uuid, count(*) AS cnt FROM deleted GROUP BY uuid"
        )
    }

    fn select_all(&self, table: &str, fields: &[&str], order_by: Option<&str>) -> String {
        let cols = fields.join(", ");
        match order_by {
            Some(col) => format!("SELECT {cols} FROM {table} ORDER BY {col}"),
            None => format!("SELECT {cols} FROM {table}"),
        }
    }

    fn select_by_field(&self, table: &str, field: &str, fields: &[&str]) -> String {
        let cols = fields.join(", ");
        format!("SELECT {cols} FROM {table} WHERE {field} = $value")
    }

    fn exists_by_uuid(&self, table: &str) -> String {
        format!("SELECT count(*) AS cnt FROM {table} WHERE _uuid = $uuid")
    }

    fn count_rows(&self, table: &str) -> String {
        format!("SELECT count(*) AS cnt FROM {table}")
    }

    fn count_marqueur_manquant(&self, table: &str, marqueur: &str) -> String {
        format!(
            "SELECT count(*) AS cnt FROM {table} \
             WHERE {marqueur} IS NULL OR {marqueur} = '' OR {marqueur} <> _text_hash"
        )
    }

    fn select_chunks_sans_marqueur(&self, table: &str, marqueur: &str, limite: usize) -> String {
        format!(
            "SELECT _uuid, _text, _text_hash FROM {table} \
             WHERE {marqueur} IS NULL OR {marqueur} = '' OR {marqueur} <> _text_hash LIMIT {limite}"
        )
    }

    fn copier_colonne(&self, table: &str, de: &str, vers: &str) -> String {
        format!("UPDATE {table} SET {vers} = {de}")
    }

    fn select_entites_a_redecouper(&self, table: &str, limite: usize) -> String {
        format!(
            "SELECT _uuid FROM {table} \
             WHERE _chunked_hash IS NULL OR _chunked_hash = '' \
                OR _chunked_hash <> _content_hash \
             LIMIT {limite}"
        )
    }

    fn reclamer_chunks_sans_marqueur(&self, table: &str, marqueur: &str, limite: usize) -> String {
        format!(
            "UPDATE {table} SET _embed_claim = $reclamation \
             WHERE _uuid IN (SELECT _uuid FROM {table} \
                WHERE ({marqueur} IS NULL OR {marqueur} = '' OR {marqueur} <> _text_hash) \
                  AND (_embed_claim IS NULL OR _embed_claim = '' \
                       OR _embed_claim < $perime OR _embed_claim LIKE '%' || $mien) \
                LIMIT {limite}) \
             RETURNING _uuid, _text, _text_hash"
        )
    }

    fn resolve_chunks_with_parent(
        &self,
        chunk_table: &str,
        parent_table: &str,
        rel_table: &str,
        _rel_forward: bool,
        has_source_refs: bool,
        parent_fields: &[&str],
    ) -> String {
        let mut select_cols = vec![
            format!("{chunk_table}._uuid AS chunk_uuid"),
            format!("{parent_table}._uuid AS parent_uuid"),
            format!("{chunk_table}._text AS c_text"),
            format!("{chunk_table}._index AS c_idx"),
            format!("{chunk_table}._start_line AS c_sline"),
            format!("{chunk_table}._end_line AS c_eline"),
            format!("{chunk_table}._start_char AS c_start"),
            format!("{chunk_table}._end_char AS c_end"),
        ];
        if has_source_refs {
            select_cols.push(format!("{chunk_table}._source_entity AS c_source_entity"));
            select_cols.push(format!("{chunk_table}._source_uuid AS c_source_uuid"));
            select_cols.push(format!("{chunk_table}._parent_field AS c_source_field"));
        }
        for f in parent_fields {
            select_cols.push(format!("{parent_table}.{f}"));
        }

        format!(
            "SELECT {} FROM {chunk_table} \
             INNER JOIN {rel_table} ON {rel_table}.from_uuid = {chunk_table}._uuid \
             INNER JOIN {parent_table} ON {rel_table}.to_uuid = {parent_table}._uuid \
             WHERE {chunk_table}._uuid = ANY($uuids)",
            select_cols.join(", ")
        )
    }

    fn embed_check_hashes(&self, table: &str, marker: &str) -> String {
        format!(
            "SELECT _uuid, {marker}, _sparse_hash FROM {table} \
             INNER JOIN jsonb_to_recordset($items::text::jsonb) AS v(uuid TEXT) ON {table}._uuid = v.uuid"
        )
    }

    fn embed_set(&self, table: &str, embedding_col: &str, marker: &str) -> String {
        format!(
            "UPDATE {table} SET {embedding_col} = v.emb, {marker} = v.hash \
             FROM jsonb_to_recordset($items::text::jsonb) AS v(uuid TEXT, emb vector, hash TEXT) \
             WHERE {table}._uuid = v.uuid"
        )
    }

    fn embed_clear(&self, table: &str, embedding_col: &str, marker: &str) -> String {
        format!(
            "UPDATE {table} SET {embedding_col} = NULL, {marker} = '' \
             FROM jsonb_to_recordset($items::text::jsonb) AS v(uuid TEXT) \
             WHERE {table}._uuid = v.uuid"
        )
    }

    fn embed_set_hash_returning_offset(&self, table: &str, marker: &str) -> String {
        let offset = self.node_offset_expr(table);
        format!(
            "UPDATE {table} SET {marker} = v.hash \
             FROM jsonb_to_recordset($items::text::jsonb) AS v(uuid TEXT, hash TEXT) \
             WHERE {table}._uuid = v.uuid \
             RETURNING v.uuid, {offset} AS offset"
        )
    }

    fn select_chunk_offsets(&self, chunk_table: &str) -> String {
        let offset = self.node_offset_expr(chunk_table);
        format!("SELECT {offset} FROM {chunk_table} WHERE _parent_uuid = ANY($uuids)")
    }

    fn embed_get_offset(&self, table: &str) -> String {
        let offset = self.node_offset_expr(table);
        format!(
            "SELECT v.uuid, {offset} AS offset FROM {table} \
             INNER JOIN jsonb_to_recordset($items::text::jsonb) AS v(uuid TEXT) ON {table}._uuid = v.uuid"
        )
    }

    fn kb_gather_fields(&self, table: &str, return_entity_fields: &[&str]) -> String {
        let returns = std::iter::once("v.uuid AS _source_uuid".to_string())
            .chain(return_entity_fields.iter().map(|f| format!("{table}.{f}")))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "SELECT {returns} FROM {table} \
             INNER JOIN jsonb_to_recordset($items::text::jsonb) AS v(uuid TEXT) ON {table}._uuid = v.uuid"
        )
    }

    fn kb_gather_content(
        &self,
        _title_entity: &str,
        rel: &str,
        content_entity: &str,
        _direction_forward: bool,
        return_fields: &[&str],
    ) -> String {
        let returns = std::iter::once("v.uuid AS _source_uuid".to_string())
            .chain(return_fields.iter().map(|f| format!("{content_entity}.{f}")))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "SELECT {returns} FROM {content_entity} \
             INNER JOIN {rel} ON {rel}.to_uuid = {content_entity}._uuid \
             INNER JOIN jsonb_to_recordset($items::text::jsonb) AS v(uuid TEXT) ON {rel}.from_uuid = v.uuid"
        )
    }

    fn kb_upsert_index(
        &self,
        index_table: &str,
        all_fields: &[&str],
        update_fields: &[&str],
    ) -> String {
        let col_list = std::iter::once("_uuid")
            .chain(all_fields.iter().copied())
            .collect::<Vec<_>>()
            .join(", ");
        // `e->>'uuid'` et non `v.uuid` : la clé de l'élément s'appelle `uuid`
        // alors que la colonne s'appelle `_uuid`. Le type de la table fournit
        // les types de tous les champs, mais il ne connaît pas `uuid` — on le
        // lit donc dans l'objet JSON lui-même, à côté.
        let val_refs = std::iter::once("e->>'uuid'".to_string())
            .chain(all_fields.iter().map(|f| format!("v.{f}")))
            .collect::<Vec<_>>()
            .join(", ");
        let update_set = update_fields.iter()
            .map(|f| format!("{f} = EXCLUDED.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "INSERT INTO {index_table} ({col_list}) \
             SELECT {val_refs} \
             FROM jsonb_array_elements($items::text::jsonb) AS e, \
                  LATERAL jsonb_populate_record(NULL::{index_table}, e) AS v \
             ON CONFLICT (_uuid) DO UPDATE SET {update_set}"
        )
    }

    fn filter_join_clause(
        &self,
        result_alias: &str,
        rel_name: &str,
        target_alias: &str,
        target_entity: &str,
        forward: bool,
    ) -> String {
        // PostgreSQL: JOIN via relation join table + entity table
        // rel_table has (from_uuid, to_uuid)
        let rel_table = &self.internal_table(rel_name);
        if forward {
            format!(
                "JOIN {rel_table} AS _r_{target_alias} ON _r_{target_alias}.from_uuid = {result_alias}._uuid \
                 JOIN {target_entity} AS {target_alias} ON {target_alias}._uuid = _r_{target_alias}.to_uuid"
            )
        } else {
            format!(
                "JOIN {rel_table} AS _r_{target_alias} ON _r_{target_alias}.to_uuid = {result_alias}._uuid \
                 JOIN {target_entity} AS {target_alias} ON {target_alias}._uuid = _r_{target_alias}.from_uuid"
            )
        }
    }

    fn nouveau_magasin_de_checkpoints(
        &self,
        conn: std::sync::Arc<dyn crate::connection::DbConnection>,
        dossier: std::path::PathBuf,
    ) -> Option<std::sync::Arc<dyn crate::dataflow::checkpoint::CheckpointStore>> {
        Some(std::sync::Arc::new(
            crate::dataflow::checkpoint_store::PostgresCheckpointStore::with_directory(conn, dossier),
        ))
    }

    /// Le chunk porte `_parent_uuid` : une jointure gauche suffit, et le
    /// décalage est `_row_id`. Même forme de lignes que le Cypher — c'est elle
    /// que la lecture par position exige.
    fn resolve_parents_with_chunks(
        &self,
        entity: &str,
        chunk_table: &str,
        _chunk_rel: &str,
        _chunk_rel_fwd: bool,
        offsets: &[u64],
        return_fields: &[String],
        has_source_refs: bool,
    ) -> String {
        let offset_list = offsets
            .iter()
            .map(|o| o.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let mut cols: Vec<String> = vec![
            "n._row_id AS _offset".to_string(),
            "n._uuid AS _uuid".to_string(),
        ];
        for f in return_fields {
            cols.push(format!("n.{f} AS {f}"));
        }
        cols.extend(colonnes_de_chunk("c", has_source_refs));
        format!(
            "SELECT {} FROM {entity} AS n \
             LEFT JOIN {chunk_table} AS c ON c._parent_uuid = n._uuid \
             WHERE n._row_id = ANY(ARRAY[{offset_list}]::bigint[])",
            cols.join(", ")
        )
    }

    /// Le chunk porte `_parent_uuid` : une seule jointure, sur une colonne
    /// indexée (`secondary_indexes`). Passer par la table de relation
    /// `{parent}_CHUNKED_FROM` en coûterait deux pour le même résultat.
    fn chunk_parent_join(
        &self,
        chunk_alias: &str,
        parent_alias: &str,
        parent_entity: &str,
    ) -> String {
        // Pas de `internal_table` ici : les tables d'entités vivent dans le
        // schéma courant, contrairement aux tables de relations.
        format!(
            "JOIN {parent_entity} AS {parent_alias} \
             ON {parent_alias}._uuid = {chunk_alias}._parent_uuid"
        )
    }

    fn filter_size_expr(&self, prop: &str) -> String {
        format!("cardinality({prop})")
    }

    fn filter_starts_with(&self, prop: &str, param: &str) -> String {
        format!("{prop} LIKE ${param} || '%'")
    }

    fn filter_contains(&self, prop: &str, param: &str) -> String {
        format!("{prop} LIKE '%' || ${param} || '%'")
    }

    fn filter_list_any_match(&self, prop: &str, param: &str) -> String {
        format!("{prop} && ${param}")
    }

    fn filter_list_all(&self, prop: &str, param: &str) -> String {
        format!("{prop} @> ${param}")
    }

    fn filter_list_none(&self, prop: &str, param: &str) -> String {
        format!("NOT ({prop} && ${param})")
    }

    fn filter_resolve_offsets(
        &self,
        table: &str,
        alias: &str,
        join_clauses: &[String],
        where_clause: &str,
        join_from: Option<(&str, &str, &str)>,
    ) -> String {
        let offset_expr = self.node_offset_expr(alias);
        match join_from {
            Some((from_alias, from_table, rel)) => {
                let rel_table = self.internal_table(rel);
                let joins = if join_clauses.is_empty() {
                    String::new()
                } else {
                    format!(" {}", join_clauses.join(" "))
                };
                format!(
                    "SELECT {offset_expr} FROM {from_table} AS {from_alias} \
                     JOIN {rel_table} AS _r ON _r.from_uuid = {from_alias}._uuid \
                     JOIN {table} AS {alias} ON {alias}._uuid = _r.to_uuid\
                     {joins} WHERE {where_clause}"
                )
            }
            None => {
                let joins = if join_clauses.is_empty() {
                    String::new()
                } else {
                    format!(" {}", join_clauses.join(" "))
                };
                format!(
                    "SELECT {offset_expr} FROM {table} AS {alias}{joins} \
                     WHERE {where_clause}"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef { name: "_uuid".into(), col_type: ColumnType::Text },
            ColumnDef { name: "_content_hash".into(), col_type: ColumnType::Text },
            ColumnDef { name: "title".into(), col_type: ColumnType::Text },
            ColumnDef { name: "body".into(), col_type: ColumnType::Text },
            ColumnDef { name: "year".into(), col_type: ColumnType::Int64 },
        ]
    }

    /// La dette de rendu des dérivées (pas B) : la marque, sa lecture, et les
    /// racines orphelines — dans les deux dialectes.
    #[test]
    fn la_dette_de_rendu_se_pose_et_se_lit() {
        let d = Rag3dbDialect;
        assert_eq!(
            d.marquer_derivees_a_rendre("TicketView"),
            "UNWIND $uuids AS u MATCH (d:TicketView {_source_uuid: u}) SET d._render_hash = ''"
        );
        assert_eq!(
            d.select_derivees_a_rendre("TicketView", 5),
            "MATCH (d:TicketView) WHERE d._render_hash IS NULL OR d._render_hash = '' RETURN d._source_uuid LIMIT 5"
        );
        assert_eq!(
            d.select_racines_sans_derivee("Ticket", "TicketView", "TicketView_DERIVED_FROM", 5),
            "MATCH (r:Ticket) OPTIONAL MATCH (d:TicketView)-[:TicketView_DERIVED_FROM]->(r) WITH r, d WHERE d._uuid IS NULL RETURN r._uuid LIMIT 5"
        );
        let p = PostgresDialect;
        assert_eq!(
            p.marquer_derivees_a_rendre("TicketView"),
            "UPDATE TicketView SET _render_hash = '' WHERE _source_uuid = ANY($uuids)"
        );
        assert!(p.select_derivees_a_rendre("TicketView", 5).starts_with("SELECT _source_uuid FROM TicketView"));
        assert!(p.select_racines_sans_derivee("Ticket", "TicketView", "x", 5).contains("LEFT JOIN TicketView d ON d._source_uuid = r._uuid"));
    }

    #[test]
    fn rag3db_create_table() {
        let d = Rag3dbDialect;
        let ddl = d.create_table("Document", &sample_columns());
        assert!(ddl.contains("CREATE NODE TABLE IF NOT EXISTS Document"));
        assert!(ddl.contains("_uuid STRING"));
        assert!(ddl.contains("year INT64"));
        assert!(ddl.contains("PRIMARY KEY(_uuid)"));
    }

    #[test]
    fn postgres_create_table() {
        let d = PostgresDialect;
        let ddl = d.create_table("Document", &sample_columns());
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS Document"));
        assert!(ddl.contains("_uuid TEXT"));
        assert!(ddl.contains("year BIGINT"));
        assert!(ddl.contains("PRIMARY KEY (_uuid)"));
    }

    #[test]
    fn rag3db_rel_table() {
        let d = Rag3dbDialect;
        let ddl = d.create_rel_table("Doc_CHUNKED_FROM", "Doc_Chunk", "Doc", &[]);
        assert!(ddl.contains("CREATE REL TABLE IF NOT EXISTS"));
        assert!(ddl.contains("FROM Doc_Chunk TO Doc"));
    }

    /// **La jointure chunk→parent, dans les deux langues.**
    ///
    /// Elle était écrite en Cypher en dur dans le catalogue : sur PostgreSQL le
    /// `WHERE` compilé référençait un `p` que la requête ne déclarait jamais,
    /// et toute recherche sous filtre utilisateur mourait sur « missing
    /// FROM-clause entry for table "p" ». Ce test tient les deux formes pour
    /// qu'une seule ne reparte pas sans l'autre.
    #[test]
    fn la_jointure_chunk_parent_parle_les_deux_langues() {
        assert_eq!(
            Rag3dbDialect.chunk_parent_join("n", "p", "Product"),
            "MATCH (n)-[:Product_CHUNKED_FROM]->(p:Product)"
        );
        assert_eq!(
            PostgresDialect.chunk_parent_join("n", "p", "Product"),
            "JOIN Product AS p ON p._uuid = n._parent_uuid"
        );
    }

    #[test]
    fn postgres_rel_table() {
        let d = PostgresDialect;
        let ddl = d.create_rel_table("doc_chunked_from", "doc_chunk", "doc", &[]);
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS doc_chunked_from"));
        assert!(ddl.contains("from_uuid TEXT NOT NULL"));
        assert!(ddl.contains("to_uuid TEXT NOT NULL"));
        assert!(ddl.contains("PRIMARY KEY (from_uuid, to_uuid)"));
    }

    #[test]
    fn postgres_rel_table_with_props() {
        let d = PostgresDialect;
        let props = vec![
            ColumnDef { name: "weight".into(), col_type: ColumnType::Double },
        ];
        let ddl = d.create_rel_table("authored_by", "doc", "author", &props);
        assert!(ddl.contains("weight DOUBLE PRECISION"));
    }

    #[test]
    fn rag3db_vector_index() {
        let d = Rag3dbDialect;
        let ddl = d.create_vector_index("Doc_Chunk", "embedding", "Doc_Chunk_vec");
        assert!(ddl.contains("CALL CREATE_VECTOR_INDEX"));
        assert!(ddl.contains("cosine"));
    }

    #[test]
    fn postgres_vector_index() {
        let d = PostgresDialect;
        let ddl = d.create_vector_index("doc_chunk", "embedding", "doc_chunk_vec");
        assert!(ddl.contains("CREATE INDEX IF NOT EXISTS doc_chunk_vec"));
        assert!(ddl.contains("USING hnsw"));
        assert!(ddl.contains("vector_cosine_ops"));
    }

    #[test]
    fn rag3db_upsert_meta() {
        let d = Rag3dbDialect;
        let stmt = d.upsert_meta("key", "value");
        assert!(stmt.contains("MERGE"));
        assert!(stmt.contains("_catalog_meta"));
    }

    #[test]
    fn postgres_upsert_meta() {
        let d = PostgresDialect;
        let stmt = d.upsert_meta("key", "value");
        assert!(stmt.contains("INSERT INTO rag3weaver._catalog_meta"));
        assert!(stmt.contains("ON CONFLICT"));
    }

    #[test]
    fn rag3db_batch_upsert() {
        let d = Rag3dbDialect;
        let stmt = d.batch_upsert("Document", &["_uuid", "title", "body"]);
        assert!(stmt.contains("UNWIND $items"));
        assert!(stmt.contains("MERGE (n:Document"));
    }

    #[test]
    fn postgres_batch_upsert() {
        let d = PostgresDialect;
        let stmt = d.batch_upsert("document", &["_uuid", "title", "body"]);
        assert!(stmt.contains("INSERT INTO document"));
        // Le type de la table fournit les types des colonnes : c'est ce qui
        // permet de déplier des lignes sans que le dialecte les connaisse.
        assert!(stmt.contains("jsonb_populate_recordset(NULL::document, $items::text::jsonb)"));
        assert!(!stmt.contains("unnest("), "unnest ne déplie pas un tableau en colonnes");
        assert!(stmt.contains("ON CONFLICT (_uuid)"));
    }

    #[test]
    fn postgres_batch_delete() {
        let d = PostgresDialect;
        let stmt = d.batch_delete("document");
        assert!(stmt.contains("DELETE FROM document"));
        assert!(stmt.contains("ANY($uuids)"));
    }

    #[test]
    fn rag3db_batch_delete() {
        let d = Rag3dbDialect;
        let stmt = d.batch_delete("Document");
        assert!(stmt.contains("UNWIND $uuids"));
        assert!(stmt.contains("DELETE n"));
        assert!(!stmt.contains("DETACH"));
    }

    #[test]
    fn type_mapping_vector() {
        assert_eq!(Rag3dbDialect.type_name(&ColumnType::Vector(1024)), "FLOAT[1024]");
        assert_eq!(PostgresDialect.type_name(&ColumnType::Vector(1024)), "vector(1024)");
    }

    #[test]
    fn postgres_internal_schema() {
        let d = PostgresDialect;
        assert_eq!(d.internal_schema(), Some("rag3weaver"));
        assert_eq!(d.internal_table("_catalog_meta"), "rag3weaver._catalog_meta");
        assert!(d.create_meta_table().contains("rag3weaver._catalog_meta"));
        assert!(d.create_blob_table().contains("rag3weaver._index_blobs"));
        assert!(d.upsert_meta("k", "v").contains("rag3weaver._catalog_meta"));
        assert!(d.load_meta_by_prefix("p").contains("rag3weaver._catalog_meta"));
    }

    #[test]
    fn postgres_setup_statements() {
        let d = PostgresDialect;
        let stmts = d.setup_statements();
        assert!(stmts.iter().any(|s| s.contains("CREATE EXTENSION IF NOT EXISTS vector")));
        assert!(stmts.iter().any(|s| s.contains("CREATE SCHEMA IF NOT EXISTS rag3weaver")));
    }

    #[test]
    fn rag3db_no_internal_schema() {
        let d = Rag3dbDialect;
        assert_eq!(d.internal_schema(), None);
        assert_eq!(d.internal_table("_catalog_meta"), "_catalog_meta");
        assert!(d.setup_statements().is_empty());
    }

    #[test]
    fn rag3db_batch_link() {
        let d = Rag3dbDialect;
        let stmt = d.batch_link("AUTHORED_BY", &["weight"]);
        assert!(stmt.contains("UNWIND $items"));
        assert!(stmt.contains("MERGE (a)-[r:AUTHORED_BY]->(b)"));
        assert!(stmt.contains("r.weight = item.weight"));
    }

    #[test]
    fn rag3db_batch_link_no_props() {
        let d = Rag3dbDialect;
        let stmt = d.batch_link("CHUNKED_FROM", &[]);
        assert!(stmt.contains("MERGE (a)-[r:CHUNKED_FROM]->(b)"));
        assert!(!stmt.contains("SET"));
    }

    #[test]
    fn postgres_batch_link() {
        let d = PostgresDialect;
        let stmt = d.batch_link("authored_by", &["weight"]);
        assert!(stmt.contains("INSERT INTO authored_by"));
        assert!(stmt.contains("from_uuid, to_uuid, weight"));
        assert!(stmt.contains("ON CONFLICT (from_uuid, to_uuid) DO UPDATE"));
    }

    #[test]
    fn postgres_batch_link_no_props() {
        let d = PostgresDialect;
        let stmt = d.batch_link("chunked_from", &[]);
        assert!(stmt.contains("DO NOTHING"));
    }

    #[test]
    fn rag3db_batch_update_fields() {
        let d = Rag3dbDialect;
        let stmt = d.batch_update_fields("Document", &["title", "_embed_hash"]);
        assert!(stmt.contains("UNWIND $items"));
        assert!(stmt.contains("MATCH (n:Document"));
        assert!(stmt.contains("n.title = item.title"));
        assert!(stmt.contains("n._embed_hash = item._embed_hash"));
    }

    #[test]
    fn postgres_batch_update_fields() {
        let d = PostgresDialect;
        let stmt = d.batch_update_fields("document", &["title", "_embed_hash"]);
        assert!(stmt.contains("UPDATE document SET"));
        assert!(stmt.contains("title = v.title"));
        assert!(stmt.contains("FROM jsonb_populate_recordset(NULL::document, $items::text::jsonb)"));
        assert!(!stmt.contains("unnest("), "unnest ne déplie pas un tableau en colonnes");
        assert!(stmt.contains("WHERE document._uuid = v._uuid"));
    }

    #[test]
    fn rag3db_select_by_uuids() {
        let d = Rag3dbDialect;
        let stmt = d.select_by_uuids("Document", &["title", "body"]);
        assert!(stmt.contains("UNWIND $uuids"));
        assert!(stmt.contains("MATCH (n:Document"));
        assert!(stmt.contains("n.title, n.body"));
    }

    #[test]
    fn postgres_select_by_uuids() {
        let d = PostgresDialect;
        let stmt = d.select_by_uuids("document", &["title", "body"]);
        assert!(stmt.contains("SELECT title, body FROM document"));
        assert!(stmt.contains("ANY($uuids)"));
    }

    #[test]
    fn rag3db_cascade_delete() {
        let d = Rag3dbDialect;
        let stmt = d.batch_cascade_delete("Document");
        assert!(stmt.contains("DETACH DELETE"));
    }

    #[test]
    fn postgres_cascade_delete() {
        let d = PostgresDialect;
        let stmt = d.batch_cascade_delete("document");
        assert!(stmt.contains("DELETE FROM document"));
        assert!(!stmt.contains("DETACH"));
    }

    #[test]
    fn rag3db_batch_update_returning() {
        let d = Rag3dbDialect;
        let stmt = d.batch_update_returning(
            "Document",
            &["_embed_hash"],
            &[("item.uuid", "uuid"), ("OFFSET(id(n))", "offset")],
        );
        assert!(stmt.contains("UNWIND $items"));
        assert!(stmt.contains("SET n._embed_hash = item._embed_hash"));
        assert!(stmt.contains("RETURN item.uuid AS uuid, OFFSET(id(n)) AS offset"));
    }

    #[test]
    fn postgres_batch_update_returning() {
        let d = PostgresDialect;
        let stmt = d.batch_update_returning(
            "document",
            &["_embed_hash"],
            &[("item.uuid", "uuid"), ("OFFSET(id(n))", "offset")],
        );
        assert!(stmt.contains("UPDATE document SET"));
        assert!(stmt.contains("_embed_hash = v._embed_hash"));
        assert!(stmt.contains("RETURNING"));
        // `_row_id` et pas `id` : c'est la colonne que `create_table` pose
        // réellement. L'assertion précédente épinglait un nom de colonne qui
        // n'existait dans aucune table — elle garantissait la faute.
        assert!(stmt.contains("document._row_id AS offset"));
        assert!(stmt.contains("v.uuid AS uuid"));
    }

    #[test]
    fn rag3db_cascade_delete_returning_count() {
        let d = Rag3dbDialect;
        let stmt = d.batch_cascade_delete_returning_count("Doc_Chunk", "_parent_uuid");
        assert!(stmt.contains("UNWIND $uuids"));
        assert!(stmt.contains("MATCH (c:Doc_Chunk {_parent_uuid: uuid})"));
        assert!(stmt.contains("DETACH DELETE c RETURN uuid, count(c) AS cnt"));
    }

    #[test]
    fn postgres_cascade_delete_returning_count() {
        let d = PostgresDialect;
        let stmt = d.batch_cascade_delete_returning_count("doc_chunk", "_parent_uuid");
        assert!(stmt.contains("DELETE FROM doc_chunk WHERE _parent_uuid = ANY($uuids)"));
        assert!(stmt.contains("count(*) AS cnt"));
    }

    #[test]
    fn rag3db_batch_delete_relation() {
        let d = Rag3dbDialect;
        let stmt = d.batch_delete_relation("AUTHORED_BY");
        assert!(stmt.contains("UNWIND $items"));
        assert!(stmt.contains("WITH item, item.from AS __cle_0 MATCH (a {_uuid: __cle_0})"));
        assert!(stmt.contains("WITH a, item, item.to AS __cle_1 MATCH (a)-[r:AUTHORED_BY]->(b {_uuid: __cle_1})"));
        assert!(stmt.contains("DELETE r"));
    }

    #[test]
    fn postgres_batch_delete_relation() {
        let d = PostgresDialect;
        let stmt = d.batch_delete_relation("authored_by");
        assert!(stmt.contains("DELETE FROM authored_by"));
        // Types déclarés sur place : `jsonb_to_recordset` suffit.
        assert!(stmt.contains("jsonb_to_recordset($items::text::jsonb) AS v(from_uuid TEXT, to_uuid TEXT)"));
        assert!(!stmt.contains("unnest("), "unnest ne déplie pas un tableau en colonnes");
    }

    #[test]
    fn rag3db_batch_select() {
        let d = Rag3dbDialect;
        let stmt = d.batch_select("kb_Index", "uuid", "_uuid", &["_uuid", "_title", "_content"]);
        assert!(stmt.contains("UNWIND $items"));
        assert!(stmt.contains("WITH item, item.uuid AS __cle_0 MATCH (n:kb_Index {_uuid: __cle_0})"));
        assert!(stmt.contains("RETURN n._uuid, n._title, n._content"));
    }

    #[test]
    fn postgres_batch_select() {
        let d = PostgresDialect;
        let stmt = d.batch_select("kb_index", "uuid", "_uuid", &["_uuid", "_title", "_content"]);
        assert!(stmt.contains("SELECT _uuid, _title, _content FROM kb_index"));
        assert!(stmt.contains("JOIN jsonb_to_recordset($items::text::jsonb) AS v(uuid TEXT)"));
        assert!(!stmt.contains("unnest("), "unnest ne déplie pas un tableau en colonnes");
    }

    #[test]
    fn rag3db_batch_set_null() {
        let d = Rag3dbDialect;
        let stmt = d.batch_set_null("Document", "_embed_hash");
        assert!(stmt.contains("UNWIND $uuids"));
        assert!(stmt.contains("SET n._embed_hash = NULL"));
    }

    #[test]
    fn postgres_batch_set_null() {
        let d = PostgresDialect;
        let stmt = d.batch_set_null("document", "_embed_hash");
        assert!(stmt.contains("UPDATE document SET _embed_hash = NULL"));
        assert!(stmt.contains("ANY($uuids)"));
    }

    #[test]
    fn rag3db_join_select_forward() {
        let d = Rag3dbDialect;
        let stmt = d.join_select("Document", "IN_KB", "kb_Index", true, "_uuid", &["m._title"]);
        assert!(stmt.contains("MATCH (n:Document {_uuid: uuid})-[:IN_KB]->(m:kb_Index)"));
    }

    #[test]
    fn rag3db_join_select_reverse() {
        let d = Rag3dbDialect;
        let stmt = d.join_select("Document", "IN_KB", "kb_Index", false, "_uuid", &["m._title"]);
        assert!(stmt.contains("MATCH (n:Document {_uuid: uuid})<-[:IN_KB]-(m:kb_Index)"));
    }

    #[test]
    fn postgres_join_select() {
        let d = PostgresDialect;
        let stmt = d.join_select("document", "in_kb", "kb_index", true, "_uuid", &["kb_index._title"]);
        assert!(stmt.contains("INNER JOIN in_kb"));
        assert!(stmt.contains("INNER JOIN kb_index"));
    }

    #[test]
    fn rag3db_node_offset_expr() {
        let d = Rag3dbDialect;
        assert_eq!(d.node_offset_expr("n"), "OFFSET(id(n))");
        assert_eq!(d.node_id_expr("n"), "ID(n)");
    }

    #[test]
    fn postgres_node_offset_expr() {
        let d = PostgresDialect;
        assert_eq!(d.node_offset_expr("n"), "n._row_id");
        assert_eq!(d.node_id_expr("n"), "n._row_id");
    }

    #[test]
    fn postgres_create_table_has_row_id() {
        let d = PostgresDialect;
        let ddl = d.create_table("Document", &sample_columns());
        assert!(ddl.contains("_row_id BIGSERIAL"));
        // _row_id should be before _uuid
        let row_id_pos = ddl.find("_row_id").unwrap();
        let uuid_pos = ddl.find("_uuid").unwrap();
        assert!(row_id_pos < uuid_pos, "_row_id should come before _uuid");
    }

    #[test]
    fn rag3db_create_table_no_row_id() {
        let d = Rag3dbDialect;
        let ddl = d.create_table("Document", &sample_columns());
        assert!(!ddl.contains("_row_id"));
    }

    #[test]
    fn column_type_from_field_type() {
        assert!(matches!(ColumnType::from_field_type(&FieldType::Text), ColumnType::Text));
        assert!(matches!(ColumnType::from_field_type(&FieldType::Int64), ColumnType::Int64));
        assert!(matches!(ColumnType::from_field_type(&FieldType::Boolean), ColumnType::Boolean));
    }
}
