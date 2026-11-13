//! Exhaustive values and schema, separated from lexical materialization bytes.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PhysicalSnapshot {
    pub schema: BTreeMap<String, String>,
    pub logical_rows: BTreeMap<String, Vec<String>>,
    pub metadata: BTreeMap<String, String>,
    pub lexical_blobs: Vec<String>,
    pub execution_rows: BTreeMap<String, Vec<String>>,
}
fn encoded_value(value: &CypherValue) -> Result<serde_json::Value> {
    use serde_json::json;
    Ok(match value {
        CypherValue::Null => json!(["null"]),
        CypherValue::Bool(v) => json!(["bool", v]),
        CypherValue::Int(v) => json!(["i64", v]),
        CypherValue::Float(v) => json!(["f64_bits", v.to_bits()]),
        CypherValue::String(v) => json!(["text", v]),
        CypherValue::Blob(v) => json!(["blob", v]),
        CypherValue::List(v) => json!([
            "list",
            v.iter().map(encoded_value).collect::<Result<Vec<_>>>()?
        ]),
        CypherValue::Map(v) => json!([
            "map",
            v.iter()
                .map(|(k, v)| Ok((k.clone(), encoded_value(v)?)))
                .collect::<Result<BTreeMap<_, _>>>()?
        ]),
        CypherValue::Typed { .. } => {
            return Err(invalid("parameter-only type in persisted values"))
        }
    })
}
fn rows(result: crate::connection::QueryResult) -> Result<Vec<String>> {
    let mut rows = result
        .rows
        .into_iter()
        .map(|row| {
            let values = row.iter().map(encoded_value).collect::<Result<Vec<_>>>()?;
            encode(&values)
        })
        .collect::<Result<Vec<_>>>()?;
    rows.sort(); // Preserve duplicate rows: multiplicity is part of the state.
    Ok(rows)
}
impl PhysicalSnapshot {
    pub(super) fn read(conn: &dyn DbConnection) -> Result<Self> {
        let tables = conn
            .execute("CALL show_tables() RETURN *")
            .map_err(storage)?;
        let mut schema = BTreeMap::new();
        let mut logical_rows = BTreeMap::new();
        let mut lexical_blobs = Vec::new();
        let mut execution_rows = BTreeMap::new();
        for row in tables.rows {
            let name = row
                .get(1)
                .and_then(CypherValue::as_str)
                .ok_or_else(|| invalid("malformed table name"))?;
            let kind = row
                .get(2)
                .and_then(CypherValue::as_str)
                .ok_or_else(|| invalid("malformed table kind"))?;
            crate::schema::validate_identifier(name, "physical table").map_err(storage)?;
            let columns = conn
                .execute(&format!("CALL table_info('{name}') RETURN *"))
                .map_err(storage)?;
            let names = columns
                .rows
                .iter()
                .map(|column| {
                    let name = column
                        .get(1)
                        .and_then(CypherValue::as_str)
                        .ok_or_else(|| invalid("malformed column name"))?;
                    crate::schema::validate_identifier(name, "physical column").map_err(storage)?;
                    Ok(name.to_string())
                })
                .collect::<Result<Vec<_>>>()?;
            if names.is_empty() {
                return Err(invalid("table without explicit properties"));
            }
            let connections = match kind {
                "NODE" => vec![],
                "REL" => rows(
                    conn.execute(&format!("CALL show_connection('{name}') RETURN *"))
                        .map_err(storage)?,
                )?,
                _ => return Err(invalid("unknown physical table kind")),
            };
            let definition = encode(&(kind, rows(columns)?, connections))?;
            if schema.insert(name.into(), definition).is_some() {
                return Err(invalid("duplicate table name"));
            }
            if matches!(name, "_DataflowMigration" | "_catalog_meta") {
                // These rows have their own closed protocol reader. They are
                // never mistaken for user data or for a published generation.
                continue;
            }
            let fields = names
                .iter()
                .map(|name| format!("n.{name}"))
                .collect::<Vec<_>>()
                .join(", ");
            let query = if kind == "REL" {
                format!("MATCH (src)-[n:{name}]->(dst) RETURN src._uuid, dst._uuid, {fields}")
            } else {
                format!("MATCH (n:{name}) RETURN {fields}")
            };
            let values = rows(conn.execute(&query).map_err(storage)?)?;
            if matches!(name, "_DataflowExecution" | "_DataflowNodeState") {
                execution_rows.insert(name.into(), values);
            } else if name == "_index_blobs" {
                lexical_blobs = values;
            } else {
                logical_rows.insert(name.into(), values);
            }
        }
        let metadata = read_metadata(conn)?
            .into_iter()
            .filter(|(key, _)| key != STATE_KEY && key != REGISTRY_KEY)
            .collect();
        Ok(Self {
            schema,
            logical_rows,
            metadata,
            lexical_blobs,
            execution_rows,
        })
    }
    pub(super) fn logical_fingerprint(&self) -> Result<String> {
        digest(
            "physical-index-logical:v1",
            &(&self.schema, &self.logical_rows, &self.metadata),
        )
    }
    pub(super) fn materialization_fingerprint(&self) -> Result<String> {
        digest("physical-index-lexical:v1", &self.lexical_blobs)
    }
}
