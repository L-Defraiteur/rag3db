//! Les valeurs qui traversent les dialectes : une valeur, un paramètre nommé,
//! le type déclaré d'un champ, et la validation d'un identifiant.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

/// Une valeur qui traverse les dialectes. Elle reprend les types que rag3db
/// (Kuzu) sait porter ; chaque dialecte la traduit vers les siens.
///
/// Variant order matters for `#[serde(untagged)]`: `Int` before `Float`
/// ensures that `42` deserializes as `Int(42)`, not `Float(42.0)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
    #[serde(skip)]
    Blob(Vec<u8>),
    /// Parameter-only type annotation; stored records/checkpoints retain plain values.
    #[serde(skip)]
    Typed { value: Box<Value>, field_type: FieldType },
}

impl Value {
    /// Validate explicit parameter types before crossing the native FFI boundary.
    pub fn validate_parameter_types(&self) -> Result<(), String> {
        match self {
            Self::Typed { value, field_type } => {
                validate_payload_type(field_type, 0)?;
                let json = serde_json::to_value(value).map_err(|e| e.to_string())?;
                normalize(field_type, &json).map(|_| ())
            }
            Self::List(values) => values.iter().try_for_each(Self::validate_parameter_types),
            Self::Map(values) => values.values().try_for_each(Self::validate_parameter_types),
            _ => Ok(()),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_blob(&self) -> Option<&[u8]> {
        match self {
            Self::Blob(b) => Some(b.as_slice()),
            _ => None,
        }
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::String(s.to_owned())
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Self::Int(n)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Self::Blob(v)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Self::List(v.into_iter().map(Into::into).collect())
    }
}

/// A named query parameter.
#[derive(Debug, Clone)]
pub struct QueryParam {
    pub name: String,
    pub value: Value,
}

impl QueryParam {
    pub fn new(name: impl Into<String>, value: impl Into<Value>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// Type of a field in an entity definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    #[serde(alias = "String")]
    String,
    #[serde(alias = "Text")]
    Text,
    #[serde(alias = "Int64")]
    Int64,
    #[serde(alias = "integer", alias = "Integer")]
    Integer,
    #[serde(alias = "Double")]
    Double,
    #[serde(alias = "number", alias = "Number")]
    Number,
    #[serde(alias = "Boolean")]
    Boolean,
    #[serde(alias = "Timestamp")]
    Timestamp,
    #[serde(alias = "Json", alias = "JSON")]
    Json,
    #[serde(alias = "Tags")]
    Tags,
    #[serde(alias = "Choice")]
    Choice,
    /// Native typed array. Legacy `Tags` remains text for compatibility.
    List(Box<FieldType>),
    /// Fixed, named object fields; deterministic ordering is part of its type.
    Struct(std::collections::BTreeMap<String, FieldType>),
}

impl Default for FieldType {
    fn default() -> Self {
        Self::String
    }
}

pub fn validate_payload_type(ty: &FieldType, depth: usize) -> Result<(), String> {
    if depth > 32 { return Err("payload type nesting exceeds 32".into()); }
    match ty {
        FieldType::List(item) => validate_payload_type(item, depth + 1),
        FieldType::Struct(fields) => {
            if fields.is_empty() { return Err("empty struct: use explicit Json for open objects".into()); }
            for (name, ty) in fields {
                if !is_valid_identifier(name) {
                    return Err(format!("invalid struct field name: \"{name}\" — must match [a-zA-Z_][a-zA-Z0-9_]*"));
                }
                validate_payload_type(ty, depth + 1)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub fn normalize(ty: &FieldType, v: &Json) -> Result<Value, String> {
    if v.is_null() { return Ok(Value::Null); }
    Ok(match ty {
        FieldType::Struct(fields) => {
            let obj = v.as_object().ok_or("expected object")?;
            if let Some(k) = obj.keys().find(|k| !fields.contains_key(*k)) { return Err(format!("undeclared field {k}")); }
            Value::Map(fields.iter().map(|(k,t)| Ok((k.clone(), normalize(t, obj.get(k).unwrap_or(&Json::Null)).map_err(|e| format!("{k}: {e}"))?))).collect::<Result<_,String>>()?)
        }
        FieldType::List(item) => Value::List(v.as_array().ok_or("expected array")?.iter().map(|v| normalize(item,v)).collect::<Result<_,_>>()?),
        FieldType::Json => Value::String(serde_json::to_string(v).map_err(|e| e.to_string())?),
        FieldType::Int64 | FieldType::Integer => Value::Int(v.as_i64().ok_or("expected signed integer")?),
        FieldType::Double | FieldType::Number => Value::Float(v.as_f64().ok_or("expected number")?),
        FieldType::Boolean => Value::Bool(v.as_bool().ok_or("expected boolean")?),
        _ => Value::String(v.as_str().ok_or("expected string")?.into()),
    })
}

/// Check whether a string is a valid identifier (`[a-zA-Z_][a-zA-Z0-9_]*`).
pub fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

