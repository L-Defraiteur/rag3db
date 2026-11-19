//! The family `read` of the memory connection (owner: recherche). See [`crate::memory_dialect`].
//!
//! The IR read on the image, as the rag3db dialect translates it (`Rag3dbDialect::select` and `hop`): a selection
//! by uuids leads with the uuid (`u`), a hop with the uuid of its start; `Column::Null`, `Label` and `Whole` travel
//! in JSON under reserved names (`(vide)`, `(étiquette)`, `(ligne entière)`). One evaluator of predicates,
//! [`matches`], serves the selections, the hops and the counts of the family `count`. What it does not say is
//! refused by its name: an order by an aggregate, `Optional` or `Let` left unbound, a `Through` on an edge.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use rag3weaver_ir::{Column, Direction, Hop, Limit, Offset, OrderBy, Predicate, Select};
use serde_json::Value;

use super::{not_written, param, State};
use crate::connection::{CypherValue, DbError, QueryParam, QueryResult};
use crate::memory_image::NodeView;

const NULL_COLUMN: &str = "(vide)";
const LABEL_COLUMN: &str = "(étiquette)";
const WHOLE_COLUMN: &str = "(ligne entière)";

fn refused(what: &str) -> DbError {
    DbError::QueryError(format!("connexion mémoire : lecture non dite en mémoire : {what}"))
}

fn form<T: serde::de::DeserializeOwned>(stmt: &Value) -> Result<T, DbError> {
    serde_json::from_value(stmt["form"].clone()).map_err(|e| DbError::QueryError(format!("connexion mémoire : forme illisible ({e})")))
}

fn needed<'a>(params: &'a [QueryParam], name: &str) -> Result<&'a CypherValue, DbError> {
    param(params, name).ok_or_else(|| DbError::QueryError(format!("connexion mémoire : paramètre « {name} » absent")))
}

fn uuids(params: &[QueryParam], name: &str) -> Result<Vec<String>, DbError> {
    match needed(params, name)? {
        CypherValue::List(items) => Ok(items.iter().filter_map(|v| if let CypherValue::String(s) = v { Some(s.clone()) } else { None }).collect()),
        CypherValue::String(s) => Ok(vec![s.clone()]),
        other => Err(DbError::QueryError(format!("connexion mémoire : « {name} » n'est pas une liste d'uuids : {other:?}"))),
    }
}

fn field(node: &NodeView, f: &str) -> CypherValue {
    match f {
        "_uuid" => CypherValue::String(node.uuid.clone()),
        _ => node.props.get(f).cloned().unwrap_or(CypherValue::Null),
    }
}

fn whole(node: &NodeView) -> CypherValue {
    let mut m = node.props.clone();
    m.insert("_uuid".into(), CypherValue::String(node.uuid.clone()));
    CypherValue::Map(m)
}

fn number(v: &CypherValue) -> Option<f64> {
    match v {
        CypherValue::Int(i) => Some(*i as f64),
        CypherValue::Float(f) => Some(*f),
        _ => None,
    }
}

fn equal(a: &CypherValue, b: &CypherValue) -> bool {
    match (number(a), number(b)) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

/// The order of Cypher on a column: numbers and texts in their order, nulls last.
fn order(a: &CypherValue, b: &CypherValue) -> Ordering {
    match (a, b) {
        (CypherValue::Null, CypherValue::Null) => Ordering::Equal,
        (CypherValue::Null, _) => Ordering::Greater,
        (_, CypherValue::Null) => Ordering::Less,
        (CypherValue::String(x), CypherValue::String(y)) => x.cmp(y),
        (CypherValue::Bool(x), CypherValue::Bool(y)) => x.cmp(y),
        _ => match (number(a), number(b)) {
            (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
            _ => format!("{a:?}").cmp(&format!("{b:?}")),
        },
    }
}

/// **Does a row satisfy a predicate.** `uuid` is the row's (empty for an edge: `Through` is then refused); `props`
/// its fields. The only evaluator of the connection: the family `count` calls it too.
pub(super) fn matches(state: &State, pred: &Predicate, uuid: &str, props: &BTreeMap<String, CypherValue>, params: &[QueryParam]) -> Result<bool, DbError> {
    let value = |f: &str| if f == "_uuid" { CypherValue::String(uuid.to_string()) } else { props.get(f).cloned().unwrap_or(CypherValue::Null) };
    let text = |v: &CypherValue| if let CypherValue::String(s) = v { Some(s.clone()) } else { None };
    Ok(match pred {
        Predicate::Equals { field, param } => equal(&value(field), needed(params, param)?),
        Predicate::Contains { field, param } => match (text(&value(field)), text(needed(params, param)?)) {
            (Some(a), Some(b)) => a.contains(&b),
            _ => false,
        },
        Predicate::StartsWith { field, param } => match (text(&value(field)), text(needed(params, param)?)) {
            (Some(a), Some(b)) => a.starts_with(&b),
            _ => false,
        },
        Predicate::AtLeast { field, param } => {
            let (a, b) = (value(field), needed(params, param)?.clone());
            !matches!(a, CypherValue::Null) && order(&a, &b) != Ordering::Less
        }
        Predicate::In { field, param } => match needed(params, param)? {
            CypherValue::List(items) => {
                let v = value(field);
                items.iter().any(|x| equal(&v, x))
            }
            other => return Err(DbError::QueryError(format!("connexion mémoire : « {param} » n'est pas une liste : {other:?}"))),
        },
        Predicate::AnyOf(all) => {
            for p in all {
                if matches(state, p, uuid, props, params)? {
                    return Ok(true);
                }
            }
            false
        }
        Predicate::AllOf(all) => {
            for p in all {
                if !matches(state, p, uuid, props, params)? {
                    return Ok(false);
                }
            }
            true
        }
        Predicate::Through { relation, direction, end, filter } => {
            if uuid.is_empty() {
                return Err(refused("Predicate::Through sur une arête"));
            }
            let mut wanted = Vec::new();
            if let Some(f) = filter {
                predicate_fields(f, &mut wanted);
            }
            for (other, _) in state.image.neighbours_fields(relation, uuid, matches!(direction, Direction::Outgoing), &[]) {
                let Some(row) = projected(state, &other, &wanted) else { continue };
                if row.table != *end {
                    continue;
                }
                if match filter {
                    Some(f) => matches(state, f, &row.uuid, &row.fields, params)?,
                    None => true,
                } {
                    return Ok(true);
                }
            }
            false
        }
        Predicate::Optional(_) => return Err(refused("Predicate::Optional non lié")),
        Predicate::Let(_) => return Err(refused("Predicate::Let")),
        Predicate::Compiled(_) | Predicate::Visible { .. } => return Err(refused("Predicate::Compiled ou Visible")),
    })
}

fn bound(limit: &Option<Limit>, params: &[QueryParam]) -> Result<Option<usize>, DbError> {
    Ok(match limit {
        None => None,
        Some(Limit::Literal(n)) => Some(*n),
        Some(Limit::Param { param, max }) => {
            let n = match needed(params, param)? {
                CypherValue::Int(i) => (*i).max(0) as usize,
                other => return Err(DbError::QueryError(format!("connexion mémoire : borne « {param} » : {other:?}"))),
            };
            Some(max.map_or(n, |m| n.min(m)))
        }
    })
}

fn skip(offset: &Option<Offset>, params: &[QueryParam]) -> Result<usize, DbError> {
    Ok(match offset {
        None => 0,
        Some(Offset::Literal(n)) => *n,
        Some(Offset::Param { param }) => match needed(params, param)? {
            CypherValue::Int(i) => (*i).max(0) as usize,
            other => return Err(DbError::QueryError(format!("connexion mémoire : saut « {param} » : {other:?}"))),
        },
    })
}

fn collect(state: &State, c: &rag3weaver_ir::Collect, uuid: &str, params: &[QueryParam]) -> Result<CypherValue, DbError> {
    let mut wanted = c.fields.clone();
    if let Some(f) = &c.filter {
        predicate_fields(f, &mut wanted);
    }
    let mut out = Vec::new();
    for (other, _) in state.image.neighbours_fields(&c.relation, uuid, matches!(c.direction, Direction::Outgoing), &[]) {
        let Some(row) = projected(state, &other, &wanted) else { continue };
        if row.table != c.end {
            continue;
        }
        if let Some(f) = &c.filter {
            if !matches(state, f, &row.uuid, &row.fields, params)? {
                continue;
            }
        }
        out.push(CypherValue::Map(c.fields.iter().map(|f| (f.clone(), row.fields.get(f).cloned().unwrap_or(CypherValue::Null))).collect()));
    }
    Ok(CypherValue::List(out))
}

/// The fields of a node a read needs: the returned ones, the filter's and the order's, each once (`_uuid` by its
/// name). Only these are unfolded from the image (`MemoryImage::node_fields`); a whole row is read whole.
fn needed_fields(returns: &[Column], filter: Option<&Predicate>, order: &[String]) -> Vec<String> {
    let mut fields: Vec<String> = Vec::new();
    for c in returns {
        if let Column::Node(f) = c {
            if f != NULL_COLUMN && f != LABEL_COLUMN && f != WHOLE_COLUMN {
                fields.push(f.clone());
            }
        }
    }
    if let Some(p) = filter {
        predicate_fields(p, &mut fields);
    }
    fields.extend(order.iter().cloned());
    let mut seen = std::collections::HashSet::new();
    fields.retain(|f| seen.insert(f.clone()));
    fields
}

/// The fields of the row a predicate reads (`Through` reads its neighbours itself).
fn predicate_fields(p: &Predicate, out: &mut Vec<String>) {
    match p {
        Predicate::Equals { field, .. }
        | Predicate::Contains { field, .. }
        | Predicate::AtLeast { field, .. }
        | Predicate::In { field, .. }
        | Predicate::StartsWith { field, .. } => out.push(field.clone()),
        Predicate::AnyOf(all) | Predicate::AllOf(all) => all.iter().for_each(|q| predicate_fields(q, out)),
        Predicate::Optional(q) => predicate_fields(q, out),
        Predicate::Through { .. } | Predicate::Let(_) | Predicate::Compiled(_) | Predicate::Visible { .. } => {}
    }
}

fn wants_whole(returns: &[Column]) -> bool {
    returns.iter().any(|c| matches!(c, Column::Whole) || matches!(c, Column::Node(f) if f == WHOLE_COLUMN))
}

/// A projected row: its uuid, its table, and its needed fields by name.
struct Row {
    uuid: String,
    table: String,
    fields: BTreeMap<String, CypherValue>,
}

fn projected(state: &State, uuid: &str, fields: &[String]) -> Option<Row> {
    let names: Vec<&str> = fields.iter().map(String::as_str).collect();
    let (table, values) = state.image.node_fields(uuid, &names)?;
    Some(Row { uuid: uuid.to_string(), table, fields: fields.iter().cloned().zip(values).collect() })
}

fn ordered(rows: &mut [(Row, Vec<CypherValue>)], keys: &[(String, bool)]) {
    if keys.is_empty() {
        return;
    }
    rows.sort_by(|(a, _), (b, _)| {
        for (f, desc) in keys {
            let null = CypherValue::Null;
            let o = order(a.fields.get(f).unwrap_or(&null), b.fields.get(f).unwrap_or(&null));
            let o = if *desc { o.reverse() } else { o };
            if o != Ordering::Equal {
                return o;
            }
        }
        Ordering::Equal
    });
}

fn whole_of(state: &State, uuid: &str) -> CypherValue {
    state.image.node(uuid).map(|n| whole(&n)).unwrap_or(CypherValue::Null)
}

fn select(state: &State, s: &Select, params: &[QueryParam]) -> Result<QueryResult, DbError> {
    let mut keys = Vec::new();
    for k in &s.order_by {
        match &k.by {
            OrderBy::Field(f) => keys.push((f.clone(), k.desc)),
            other => return Err(refused(&format!("un ordre par {other:?}"))),
        }
    }
    let order_fields: Vec<String> = keys.iter().map(|(f, _)| f.clone()).collect();
    let fields = needed_fields(&s.returns, s.filter.as_ref(), &order_fields);
    let candidates: Vec<String> = if s.by_uuids { uuids(params, "uuids")? } else { state.image.nodes_of_table(&s.table) };
    let mut kept: Vec<(Row, Vec<CypherValue>)> = Vec::new();
    for u in candidates {
        let Some(row) = projected(state, &u, &fields) else { continue };
        if row.table != s.table {
            continue;
        }
        if let Some(f) = &s.filter {
            if !matches(state, f, &row.uuid, &row.fields, params)? {
                continue;
            }
        }
        kept.push((row, Vec::new()));
    }
    ordered(&mut kept, &keys);
    let from = skip(&s.offset, params)?;
    let take = bound(&s.limit, params)?.unwrap_or(usize::MAX);
    let mut columns: Vec<String> = s.by_uuids.then(|| "u".to_string()).into_iter().collect();
    for c in &s.returns {
        columns.push(match c {
            Column::Node(f) => f.clone(),
            Column::Collect(c) => c.alias.clone(),
            other => format!("{other:?}"),
        });
    }
    let whole_wanted = wants_whole(&s.returns);
    let mut rows = Vec::new();
    for (row, _) in kept.into_iter().skip(from).take(take) {
        let mut out: Vec<CypherValue> = s.by_uuids.then(|| CypherValue::String(row.uuid.clone())).into_iter().collect();
        let entire = if whole_wanted { whole_of(state, &row.uuid) } else { CypherValue::Null };
        for c in &s.returns {
            out.push(match c {
                Column::Node(f) if f == WHOLE_COLUMN => entire.clone(),
                Column::Node(f) if f == NULL_COLUMN || f == LABEL_COLUMN => CypherValue::Null,
                Column::Node(f) => row.fields.get(f).cloned().unwrap_or(CypherValue::Null),
                Column::Whole => entire.clone(),
                Column::Collect(c) => collect(state, c, &row.uuid, params)?,
                // As the rag3db dialect: an edge, a label or a null column of a selection is NULL.
                Column::Edge(_) | Column::Label | Column::Null => CypherValue::Null,
            });
        }
        rows.push(out);
    }
    Ok(QueryResult { columns, rows })
}

fn hop(state: &State, h: &Hop, params: &[QueryParam]) -> Result<QueryResult, DbError> {
    let fields = needed_fields(&h.returns, h.filter.as_ref(), &h.order_by);
    let mut edge_fields: Vec<String> = h.returns.iter().filter_map(|c| if let Column::Edge(f) = c { Some(f.clone()) } else { None }).collect();
    if let Some(x) = &h.exclude {
        edge_fields.push(x.field.clone());
    }
    let edge_names: Vec<&str> = edge_fields.iter().map(String::as_str).collect();
    let edge_at = |f: &str| edge_fields.iter().position(|e| e == f);
    let whole_wanted = wants_whole(&h.returns);
    let mut rows: Vec<(Row, Vec<CypherValue>)> = Vec::new();
    for start in uuids(params, "uuids")? {
        let Some((start_table, _)) = state.image.node_fields(&start, &[]) else { continue };
        if h.start.as_ref().is_some_and(|t| *t != start_table) {
            continue;
        }
        let outgoing = matches!(h.direction, Direction::Outgoing);
        for (other, edge) in state.image.neighbours_fields(&h.relation, &start, outgoing, &edge_names) {
            let Some(row) = projected(state, &other, &fields) else { continue };
            if h.end.as_ref().is_some_and(|t| *t != row.table) {
                continue;
            }
            if let Some(x) = &h.exclude {
                if let Some(CypherValue::String(v)) = edge_at(&x.field).map(|i| &edge[i]) {
                    if x.values.iter().any(|w| w == v) {
                        continue;
                    }
                }
            }
            if let Some(f) = &h.filter {
                if !matches(state, f, &row.uuid, &row.fields, params)? {
                    continue;
                }
            }
            let entire = if whole_wanted { whole_of(state, &row.uuid) } else { CypherValue::Null };
            let mut out = vec![CypherValue::String(start.clone())];
            for c in &h.returns {
                out.push(match c {
                    Column::Node(f) if f == WHOLE_COLUMN => entire.clone(),
                    Column::Node(f) if f == LABEL_COLUMN => CypherValue::String(row.table.clone()),
                    Column::Node(f) if f == NULL_COLUMN => CypherValue::Null,
                    Column::Node(f) => row.fields.get(f).cloned().unwrap_or(CypherValue::Null),
                    Column::Edge(f) => edge_at(f).map(|i| edge[i].clone()).unwrap_or(CypherValue::Null),
                    Column::Label => CypherValue::String(row.table.clone()),
                    Column::Whole => entire.clone(),
                    Column::Null => CypherValue::Null,
                    Column::Collect(_) => return Err(refused("une liste liée dans un saut")),
                });
            }
            rows.push((row, out));
        }
    }
    let keys: Vec<(String, bool)> = h.order_by.iter().map(|f| (f.clone(), false)).collect();
    ordered(&mut rows, &keys);
    let take = h.limit.unwrap_or(usize::MAX);
    let mut columns = vec!["u".to_string()];
    columns.extend(h.returns.iter().map(|c| format!("{c:?}")));
    Ok(QueryResult { columns, rows: rows.into_iter().take(take).map(|(_, r)| r).collect() })
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().into_iter().flatten().filter_map(|s| s.as_str().map(str::to_string)).collect()
}

fn fields_of(node: &NodeView, fields: &[String]) -> Vec<CypherValue> {
    fields.iter().map(|f| field(node, f)).collect()
}

pub(super) fn execute(state: &mut State, op: &str, stmt: &Value, params: &[QueryParam]) -> Result<QueryResult, DbError> {
    let table = stmt["table"].as_str().unwrap_or_default();
    let of_table = |state: &State| -> Vec<NodeView> { state.image.nodes_of_table(table).into_iter().filter_map(|u| state.image.node(&u)).collect() };
    let rows = |rows: Vec<Vec<CypherValue>>, columns: Vec<String>| -> Result<QueryResult, DbError> { Ok(QueryResult { columns, rows }) };
    match op {
        "select" => select(state, &form::<Select>(stmt)?, params),
        "hop" => hop(state, &form::<Hop>(stmt)?, params),
        "select_by_uuids" => {
            let fields = strings(&stmt["fields"]);
            let found = uuids(params, "uuids")?.into_iter().filter_map(|u| state.image.node(&u)).filter(|n| n.table == table);
            rows(found.map(|n| fields_of(&n, &fields)).collect(), fields.clone())
        }
        "select_all" => {
            let fields = strings(&stmt["fields"]);
            let mut nodes = of_table(state);
            if let Some(by) = stmt["order_by"].as_str() {
                nodes.sort_by(|a, b| order(&field(a, by), &field(b, by)));
            }
            rows(nodes.iter().map(|n| fields_of(n, &fields)).collect(), fields.clone())
        }
        "select_by_field" => {
            let (by, fields) = (stmt["field"].as_str().unwrap_or_default(), strings(&stmt["fields"]));
            let wanted = needed(params, "value")?.clone();
            rows(of_table(state).iter().filter(|n| equal(&field(n, by), &wanted)).map(|n| fields_of(n, &fields)).collect(), fields.clone())
        }
        "exists_by_uuid" => {
            let CypherValue::String(u) = needed(params, "uuid")? else { return Err(refused("exists_by_uuid sans uuid texte")) };
            let n = state.image.node(u).filter(|n| n.table == table).is_some() as i64;
            rows(vec![vec![CypherValue::Int(n)]], vec!["cnt".into()])
        }
        "select_snapshot_scope" => {
            let (scope, extra) = (strings(&stmt["scope"]), strings(&stmt["extra"]));
            let mut fields = vec!["_uuid".to_string(), "_snapshot".to_string()];
            fields.extend(extra);
            let mut out = Vec::new();
            for n in of_table(state) {
                let mut keep = true;
                for (i, f) in scope.iter().enumerate() {
                    if !equal(&field(&n, f), needed(params, &format!("p{i}"))?) {
                        keep = false;
                        break;
                    }
                }
                if keep {
                    out.push(fields_of(&n, &fields));
                }
            }
            rows(out, fields)
        }
        "select_all_uuids" => rows(of_table(state).iter().map(|n| vec![CypherValue::String(n.uuid.clone())]).collect(), vec!["_uuid".into()]),
        other => Err(not_written("read", other)),
    }
}
