//! Explicit JSON Schema → native rag3db payload types. No inference from sample rows.
//! This is a storage mapping, not a full JSON Schema validator (format, pattern,
//! numeric bounds, etc. remain the source validator's responsibility).
use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::config::{FieldType, SimpleFieldDef};
use crate::connection::CypherValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterField {
    pub path: Vec<String>,
    pub field_type: FieldType,
    /// Array-object scopes that must be entered with a `nested` condition.
    pub nested_scopes: Vec<Vec<String>>,
    pub operators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaMapping {
    pub fields: HashMap<String, SimpleFieldDef>,
    pub filter_fields: Vec<FilterField>,
    /// Open objects remain JSON text. They are preserved, not advertised as typed filters.
    pub opaque_paths: Vec<Vec<String>>,
}

impl JsonSchemaMapping {
    pub fn from_schema(root: &Value) -> Result<Self, String> {
        let mut opaque_paths = vec![];
        let ty = map_type(root, root, &mut vec![], &mut opaque_paths, 0)?;
        let FieldType::Struct(properties) = ty else { return Err("root must be a fixed object".into()); };
        let mut filter_fields = vec![];
        for (name, ty) in &properties {
            describe(ty, vec![name.clone()], vec![], &mut filter_fields);
        }
        let fields = properties.into_iter().map(|(k, field_type)| (k, SimpleFieldDef { field_type, ..Default::default() })).collect();
        Ok(Self { fields, filter_fields, opaque_paths })
    }

    /// Preserve objects/lists as Cypher values; only explicitly opaque JSON becomes text.
    /// Missing fields become NULL, never fake zero/empty values. Required/enum checks
    /// are the source JSON Schema validator's responsibility.
    pub fn record(&self, value: &Value) -> Result<BTreeMap<String, CypherValue>, String> {
        let fields = self.fields.iter().map(|(k,v)| (k.clone(), v.field_type.clone())).collect();
        match normalize(&FieldType::Struct(fields), value)? {
            CypherValue::Map(m) => Ok(m),
            _ => Err("record must be an object".into()),
        }
    }
}

fn map_type(root: &Value, s: &Value, path: &mut Vec<String>, opaque: &mut Vec<Vec<String>>, depth: usize) -> Result<FieldType, String> {
    let err = |message: &str| format!("{}: {message}", path.join("."));
    if depth > 32 { return Err(err("recursive schema or nesting exceeds 32")); }
    if let Some(reference) = s.get("$ref").and_then(Value::as_str) {
        let pointer = reference.strip_prefix('#').ok_or_else(|| err("only local $ref supported"))?;
        let target = root.pointer(pointer).ok_or_else(|| err("unknown $ref"))?;
        return map_type(root, target, path, opaque, depth + 1);
    }
    for keyword in ["anyOf", "oneOf"] {
        if let Some(variants) = s.get(keyword).and_then(Value::as_array) {
            let nonnull: Vec<_> = variants.iter().filter(|v| v.get("type").and_then(Value::as_str) != Some("null")).collect();
            if nonnull.len() != 1 { return Err(err("heterogeneous unions need an explicit mapping")); }
            return map_type(root, nonnull[0], path, opaque, depth + 1);
        }
    }
    if s.get("allOf").is_some() { return Err(err("allOf must be resolved before mapping")); }
    let typename = match s.get("type") {
        Some(Value::String(t)) => t.as_str(),
        Some(Value::Array(types)) => {
            let types: Vec<_> = types.iter().filter_map(Value::as_str).filter(|t| *t != "null").collect();
            if types.len() != 1 { return Err(err("heterogeneous types need an explicit mapping")); }
            types[0]
        }
        _ if s.get("properties").is_some() => "object",
        _ => return Err(err("missing concrete type")),
    };
    Ok(match typename {
        "string" => FieldType::String,
        "integer" => FieldType::Int64,
        "number" => FieldType::Double,
        "boolean" => FieldType::Boolean,
        "array" => {
            let item = s.get("items").ok_or_else(|| err("array requires items"))?;
            FieldType::List(Box::new(map_type(root, item, path, opaque, depth + 1)?))
        }
        "object" => {
            let props = s.get("properties").and_then(Value::as_object);
            if props.is_none_or(|p| p.is_empty()) || s.get("additionalProperties").is_some_and(|v| v != &Value::Bool(false)) {
                opaque.push(path.clone());
                FieldType::Json
            } else {
                let mut fields = BTreeMap::new();
                for (k,v) in props.unwrap() {
                    crate::schema::validate_identifier(k, "JSON property").map_err(|e| e.to_string())?;
                    path.push(k.clone());
                    let ty = map_type(root, v, path, opaque, depth + 1)?;
                    path.pop();
                    fields.insert(k.clone(), ty);
                }
                FieldType::Struct(fields)
            }
        }
        _ => return Err(err("unsupported type")),
    })
}

fn describe(ty: &FieldType, path: Vec<String>, scopes: Vec<Vec<String>>, out: &mut Vec<FilterField>) {
    let ops: &[&str] = match ty {
        FieldType::Struct(fields) => {
            for (k,v) in fields { let mut p = path.clone(); p.push(k.clone()); describe(v,p,scopes.clone(),out); }
            return;
        }
        FieldType::List(item) if matches!(item.as_ref(), FieldType::Struct(_)) => {
            let mut scopes = scopes; scopes.push(path.clone());
            describe(item,path,scopes,out); return;
        }
        FieldType::Json | FieldType::Tags => return,
        FieldType::List(item) if matches!(item.as_ref(), FieldType::Json | FieldType::List(_)) => return,
        FieldType::List(_) => &["has_any", "has_all", "has_none", "is_null", "is_empty", "values_count"],
        FieldType::Int64 | FieldType::Integer | FieldType::Double | FieldType::Number => &["eq", "neq", "gt", "gte", "lt", "lte", "in", "is_null"],
        FieldType::Boolean => &["eq", "neq", "is_null"],
        _ => &["eq", "neq", "in", "starts_with", "contains", "is_null"],
    };
    out.push(FilterField { path, field_type: ty.clone(), nested_scopes: scopes, operators: ops.iter().map(|s| s.to_string()).collect() });
}

pub(crate) fn normalize(ty: &FieldType, v: &Value) -> Result<CypherValue, String> {
    if v.is_null() { return Ok(CypherValue::Null); }
    Ok(match ty {
        FieldType::Struct(fields) => {
            let obj = v.as_object().ok_or("expected object")?;
            if let Some(k) = obj.keys().find(|k| !fields.contains_key(*k)) { return Err(format!("undeclared field {k}")); }
            CypherValue::Map(fields.iter().map(|(k,t)| Ok((k.clone(), normalize(t, obj.get(k).unwrap_or(&Value::Null)).map_err(|e| format!("{k}: {e}"))?))).collect::<Result<_,String>>()?)
        }
        FieldType::List(item) => CypherValue::List(v.as_array().ok_or("expected array")?.iter().map(|v| normalize(item,v)).collect::<Result<_,_>>()?),
        FieldType::Json => CypherValue::String(serde_json::to_string(v).map_err(|e| e.to_string())?),
        FieldType::Int64 | FieldType::Integer => CypherValue::Int(v.as_i64().ok_or("expected signed integer")?),
        FieldType::Double | FieldType::Number => CypherValue::Float(v.as_f64().ok_or("expected number")?),
        FieldType::Boolean => CypherValue::Bool(v.as_bool().ok_or("expected boolean")?),
        _ => CypherValue::String(v.as_str().ok_or("expected string")?.into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn nullable_refs_and_nested_arrays_preserve_types() {
        let schema = json!({"type":"object", "properties": {
            "colors":{"type":"array","items":{"type":"string"}},
            "abilities":{"type":"array","items":{"$ref":"#/$defs/Ability"}},
            "attributes":{"type":"object","additionalProperties":{"type":"string"}}
        }, "$defs":{"Ability":{"type":"object","properties":{
            "name":{"type":"string"}, "level":{"anyOf":[{"type":"integer"},{"type":"null"}]}
        }}}});
        let mapping = JsonSchemaMapping::from_schema(&schema).unwrap();
        let row = mapping.record(&json!({"colors":[],"abilities":[{"name":"Flying","level":null}],"attributes":{"custom":"yes"}})).unwrap();
        assert_eq!(row["colors"], CypherValue::List(vec![]));
        assert!(matches!(&row["abilities"], CypherValue::List(v) if matches!(&v[0], CypherValue::Map(_))));
        assert_eq!(mapping.opaque_paths, vec![vec!["attributes"]]);
        assert!(mapping.filter_fields.iter().any(|f| f.path == ["abilities", "level"] && f.nested_scopes == [vec!["abilities"]]));
        assert!(mapping.record(&json!({"colors":[3]})).is_err());
        let serialized = serde_json::to_value(&mapping).unwrap();
        let _: JsonSchemaMapping = serde_json::from_value(serialized).unwrap();
    }
    #[test]
    fn rejects_ambiguous_or_recursive_schema() {
        for s in [json!({"type":"array"}),json!({"anyOf":[{"type":"integer"},{"type":"string"}]}),json!({"$ref":"#"})] {
            assert!(JsonSchemaMapping::from_schema(&s).is_err());
        }
    }
}

/// Validate the explicit structured branches against the declared payload types.
/// Legacy Field joins keep their existing semantics; inside Nested, Field is local.
pub fn validate_structured_filter(condition: &crate::filter::FilterCondition, fields: &HashMap<String, SimpleFieldDef>) -> Result<(), String> {
    let root = FieldType::Struct(fields.iter().map(|(k,v)| (k.clone(),v.field_type.clone())).collect());
    validate_condition(condition, &root, false, 0)
}
fn validate_condition(c: &crate::filter::FilterCondition, root: &FieldType, nested: bool, depth: usize) -> Result<(), String> {
    use crate::filter::FilterCondition as C;
    if depth > 32 { return Err("filter nesting exceeds 32".into()); }
    fn at<'a>(mut ty: &'a FieldType, path: &[String]) -> Result<&'a FieldType,String> {
        if path.is_empty() { return Err("empty filter path".into()); }
        for part in path {
            crate::schema::validate_identifier(part, "filter path").map_err(|e|e.to_string())?;
            let FieldType::Struct(fields) = ty else { return Err(format!("{part}: object expected; enter arrays with nested")); };
            ty = fields.get(part).ok_or_else(||format!("unknown filter path component {part}"))?;
        }
        Ok(ty)
    }
    match c {
        C::Path {path,value} => validate_filter_value(at(root,path)?,value),
        C::Nested {path,condition} => {
            let FieldType::List(item) = at(root,path)? else { return Err("nested requires an array".into()); };
            if !matches!(item.as_ref(), FieldType::Struct(_)) { return Err("nested requires object elements".into()); }
            validate_condition(condition,item,true,depth+1)
        }
        C::Field {key,value} if nested => validate_filter_value(at(root,std::slice::from_ref(key))?,value),
        C::Field {..} => Ok(()),
        C::Must(cs) | C::Should(cs) | C::MustNot(cs) => cs.iter().try_for_each(|c| validate_condition(c,root,nested,depth+1)),
    }
}
fn validate_filter_value(ty: &FieldType, value: &crate::filter::FilterValue) -> Result<(), String> {
    use crate::filter::{FilterOp as O, FilterValue as V};
    let check = |t: &FieldType, v: &CypherValue| normalize(t,&serde_json::to_value(v).map_err(|e|e.to_string())?).map(|_|());
    match value {
        V::Direct(v) => check(ty,v),
        V::List(vs) => vs.iter().try_for_each(|v|check(ty,v)),
        V::Ops(ops) => {
            if ops.is_empty() { return Err("empty filter predicate".into()); }
            for op in ops {
                match op {
                    O::HasAny(vs) | O::HasAll(vs) | O::HasNone(vs) => {
                        let FieldType::List(item) = ty else { return Err("list operator on non-array".into()); };
                        vs.iter().try_for_each(|v|check(item,v))?;
                    }
                    O::Gt(v) | O::Gte(v) | O::Lt(v) | O::Lte(v) => {
                        if !matches!(ty,FieldType::Int64|FieldType::Integer|FieldType::Double|FieldType::Number|FieldType::String|FieldType::Timestamp) { return Err("ordered scalar expected".into()); }
                        check(ty,v)?;
                    }
                    O::Eq(v) | O::Neq(v) => check(ty,v)?,
                    O::In(vs) | O::NotIn(vs) => vs.iter().try_for_each(|v|check(ty,v))?,
                    O::Between(a,b) => { check(ty,a)?; check(ty,b)?; }
                    O::Contains(_) | O::StartsWith(_) if !matches!(ty,FieldType::String|FieldType::Text) => return Err("text operator on non-text".into()),
                    O::ValuesCount {..} | O::IsEmpty | O::IsNotEmpty if !matches!(ty,FieldType::List(_) | FieldType::String | FieldType::Text) => return Err("size operator on scalar".into()),
                    _ => (),
                }
            }
            Ok(())
        }
    }
}
