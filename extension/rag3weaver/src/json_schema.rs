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
    /// **Le vocabulaire déclaré** (`enum` du schéma, ou de ses `items`) :
    /// ce qu'un agent peut écrire dans un filtre sans deviner `"Red"` ou
    /// `"R"`. Absent pour un vocabulaire ouvert.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<Value>>,
    /// Le premier des `examples` déclarés par le schéma : c'est lui que
    /// l'exemple de filtre montre, plutôt qu'une valeur devinée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<Value>,
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
        let props = resolve(root, root, 0).and_then(|r| r.get("properties")).and_then(Value::as_object);
        for f in filter_fields.iter_mut().filter(|f| f.path.len() == 1) {
            let schema = props.and_then(|p| p.get(&f.path[0]));
            f.values = schema.and_then(|s| declared_values(root, s));
            f.example = schema
                .and_then(|s| resolve(root, s, 0))
                .and_then(|s| s.get("examples"))
                .and_then(Value::as_array)
                .and_then(|e| e.first().cloned());
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

/// Suit les `$ref` locaux et les unions nullables jusqu'au schéma concret.
fn resolve<'a>(root: &'a Value, s: &'a Value, depth: usize) -> Option<&'a Value> {
    if depth > 32 {
        return None;
    }
    if let Some(reference) = s.get("$ref").and_then(Value::as_str) {
        return resolve(root, root.pointer(reference.strip_prefix('#')?)?, depth + 1);
    }
    for keyword in ["anyOf", "oneOf"] {
        if let Some(variants) = s.get(keyword).and_then(Value::as_array) {
            let nonnull: Vec<_> = variants.iter().filter(|v| v.get("type").and_then(Value::as_str) != Some("null")).collect();
            return match nonnull.as_slice() {
                [one] => resolve(root, one, depth + 1),
                _ => None,
            };
        }
    }
    Some(s)
}

/// L'`enum` d'un champ, ou celui de ses éléments pour une liste.
fn declared_values(root: &Value, s: &Value) -> Option<Vec<Value>> {
    let s = resolve(root, s, 0)?;
    let s = match s.get("items") {
        Some(items) => resolve(root, items, 0)?,
        None => s,
    };
    s.get("enum").and_then(Value::as_array).map(|v| v.iter().filter(|x| !x.is_null()).cloned().collect())
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
    out.push(FilterField { path, field_type: ty.clone(), nested_scopes: scopes, operators: ops.iter().map(|s| s.to_string()).collect(), values: None, example: None });
}

// ─── Filtres exposés aux agents ──────────────────────────────────────────────
//
// Un paramètre `filter` ou `options` d'un outil n'était annoncé que comme
// « json » : l'agent ne connaissait ni la grammaire, ni les champs, ni leur
// vocabulaire. Il écrivait `{}` (refusé), mettait « instant cost {1} » dans
// le texte de la requête, ou `"filter"` au lieu de `"filter_condition"` —
// ignoré sans bruit. Tout ce qu'il faut savoir est dans le schéma de
// l'entité : on le génère ici, pour n'importe quel backend (27 septembre 2026).

/// Les champs qu'un filtre peut nommer par `key` : ceux du premier niveau,
/// hors listes d'objets (celles-là passent par `nested`).
fn champs_simples(fields: &[FilterField]) -> Vec<&FilterField> {
    fields.iter().filter(|f| f.path.len() == 1 && f.nested_scopes.is_empty() && !f.path[0].starts_with('_')).collect()
}

fn nom_de_type(ty: &FieldType) -> &'static str {
    match ty {
        FieldType::Int64 | FieldType::Integer => "entiers",
        FieldType::Double | FieldType::Number => "nombres",
        FieldType::Boolean => "booléens",
        FieldType::List(_) => "listes",
        FieldType::Timestamp => "dates",
        _ => "textes",
    }
}

/// Le schéma JSON d'une condition de filtre : **sa forme seulement**. Les
/// clés, les opérateurs et les niveaux imbriqués sont vérifiés par le moteur,
/// avec ses propres messages ; les recopier ici dans chaque outil coûtait
/// ~16 000 jetons par tour au modèle (27 septembre 2026).
pub fn filter_condition_schema() -> Value {
    // Une clé, ou must / should / must_not ensemble (la forme booléenne de
    // Qdrant ou d'Elasticsearch) ; le moteur refuse les autres mélanges.
    serde_json::json!({"type":"object","additionalProperties":false,"properties":{
        "field":{"type":"object","additionalProperties":false,"required":["key","value"],"properties":{"key":{"type":"string"},"value":{}}},
        "must":{"type":"array","items":{"type":"object"}},
        "should":{"type":"array","items":{"type":"object"}},
        "must_not":{"type":"array","items":{"type":"object"}},
        "path":{},"nested":{}
    }})
}

/// La description d'un paramètre de filtre : la grammaire en une ligne, les
/// champs **groupés par type** avec leurs opérateurs et leur vocabulaire, et
/// un exemple construit depuis les champs réels. Compacte : elle part vers
/// le modèle à chaque tour.
pub fn filter_description(fields: &[FilterField]) -> String {
    let simples = champs_simples(fields);
    let mut groupes: BTreeMap<&str, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for f in &simples {
        let g = groupes.entry(nom_de_type(&f.field_type)).or_default();
        let nom = match &f.values {
            Some(v) if !v.is_empty() => format!(
                "{} [{}]",
                f.path[0],
                v.iter().map(|x| x.as_str().map(str::to_string).unwrap_or_else(|| x.to_string())).collect::<Vec<_>>().join(", ")
            ),
            _ => f.path[0].clone(),
        };
        g.0.push(nom);
        if g.1.is_empty() {
            g.1 = f.operators.clone();
        }
    }
    let champs = groupes
        .iter()
        .map(|(t, (noms, ops))| format!("{t} ({}) : {}", ops.join(", "), noms.join(", ")))
        .collect::<Vec<_>>()
        .join(" ; ");
    format!(
        "{{}} : aucun filtre. field.value : [{{\"op\":…,\"value\":…}}], une liste (in) ou une valeur (eq) ; must/should/must_not : liste de filtres. Champs — {champs}. Exemple : {}",
        filter_example(&simples)
    )
}

/// Un exemple réaliste. D'abord les `examples` déclarés par le schéma (deux
/// au plus, combinés par `must`) ; sinon une liste au vocabulaire déclaré et
/// un nombre, en évitant les champs techniques (`key`, `…_id`, `…_code`) et
/// les valeurs vides de sens (`unknown`, `none`, `other`).
fn filter_example(simples: &[&FilterField]) -> String {
    let condition = |f: &FilterField, v: &Value| match (&f.field_type, v) {
        (FieldType::List(_), Value::Array(_)) => serde_json::json!({"field":{"key":f.path[0],"value":[{"op":"has_any","value":v}]}}),
        (FieldType::List(_), _) => serde_json::json!({"field":{"key":f.path[0],"value":[{"op":"has_any","value":[v]}]}}),
        _ => serde_json::json!({"field":{"key":f.path[0],"value":[v]}}),
    };
    let mut conditions: Vec<Value> = simples
        .iter()
        .filter_map(|f| f.example.as_ref().map(|v| condition(f, v)))
        .take(2)
        .collect();
    if conditions.is_empty() {
        let technique = |f: &&&FilterField| {
            let n = f.path[0].as_str();
            n == "key" || n.ends_with("_id") || n.ends_with("_code") || n.ends_with("_ids")
        };
        let parlante = |v: &&Value| !matches!(v.as_str(), Some("unknown" | "none" | "other" | ""));
        let vocabulaire = simples
            .iter()
            .filter(|f| !technique(f))
            .filter_map(|f| f.values.as_ref().and_then(|v| v.iter().find(parlante)).map(|v| (f, v)))
            .min_by_key(|(f, _)| !matches!(f.field_type, FieldType::List(_)));
        if let Some((f, v)) = vocabulaire {
            conditions.push(condition(f, v));
        }
        if let Some(f) = simples.iter().filter(|f| !technique(f)).find(|f| {
            matches!(f.field_type, FieldType::Int64 | FieldType::Integer | FieldType::Double | FieldType::Number)
        }) {
            conditions.push(serde_json::json!({"field":{"key":f.path[0],"value":[{"op":"lte","value":2}]}}));
        }
    }
    match conditions.len() {
        0 => "{}".into(),
        1 => conditions.remove(0).to_string(),
        _ => serde_json::json!({"must":conditions}).to_string(),
    }
}

/// Le schéma des `options` d'une recherche exposée : le filtre typé, la
/// pagination, et **rien d'autre** — une clé inconnue (`filter` pour
/// `filter_condition`) devient une erreur au lieu d'être ignorée.
pub fn search_options_schema(filter_description: &str) -> Value {
    let mut filtre = filter_condition_schema();
    filtre["description"] = Value::String(filter_description.to_string());
    serde_json::json!({
        "type":"object","additionalProperties":false,
        "description":"Options : filter_condition (critères structurés sur les champs de l'entité — jamais dans le texte de la requête), limit, offset.",
        "properties":{
            "filter_condition":filtre,
            "limit":{"type":"integer","minimum":1,"maximum":200},
            "offset":{"type":"integer","minimum":0},
            "signals":{},"consistency":{}
        }
    })
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

    /// Un champ inconnu est refusé avant la base, avec les champs qui existent ;
    /// jointures et colonnes internes passent.
    #[test]
    fn un_champ_inconnu_est_refuse_avec_la_liste() {
        let mapping = JsonSchemaMapping::from_schema(&json!({"type":"object","properties":{
            "kind":{"type":"string"},"effects":{"type":"array","items":{"type":"string"}}}})).unwrap();
        let f = |v| serde_json::from_value::<crate::filter::FilterCondition>(v).unwrap();
        let e = check_field_names(&f(json!({"must":[{"field":{"key":"card_types","value":["Artifact"]}}]})), &mapping.fields).unwrap_err();
        assert!(e.contains("card_types") && e.contains("effects, kind"), "{e}");
        for ok in [json!({"field":{"key":"kind","value":"activated"}}), json!({"field":{"key":"deck.name","value":"x"}}), json!({"field":{"key":"_uuid","value":"x"}})] {
            check_field_names(&f(ok.clone()), &mapping.fields).unwrap_or_else(|e| panic!("{ok} : {e}"));
        }
    }

    /// Le filtre d'un outil se décrit depuis le schéma de l'entité : les
    /// champs par type, le vocabulaire déclaré, un exemple tiré des
    /// `examples` ; la forme refuse deux clés, les options une clé inconnue.
    #[test]
    fn le_filtre_d_un_outil_se_decrit_depuis_le_schema() {
        let schema = json!({"type":"object","properties":{
            "key":{"type":"string"},
            "types":{"type":"array","items":{"type":"string","enum":["Creature","Instant"]},"examples":[["Instant"]]},
            "rarity":{"type":"string","enum":["unknown","common","rare"]},
            "cost":{"type":"integer","examples":[2]},
            "owned":{"type":"integer"}
        }});
        let mapping = JsonSchemaMapping::from_schema(&schema).unwrap();
        let d = filter_description(&mapping.filter_fields);
        assert!(d.contains("types [Creature, Instant]") && d.contains("rarity [unknown, common, rare]"), "{d}");
        assert!(d.contains(r#"{"must":[{"field":{"key":"cost","value":[2]}},{"field":{"key":"types","value":[{"op":"has_any","value":["Instant"]}]}}]}"#), "{d}");
        let sans_exemples = JsonSchemaMapping::from_schema(&json!({"type":"object","properties":{
            "rarity":{"type":"string","enum":["unknown","common"]},"owned":{"type":"integer"},"set_id":{"type":"integer"}}})).unwrap();
        let d = filter_description(&sans_exemples.filter_fields);
        assert!(d.contains(r#""key":"rarity","value":["common"]"#) && d.contains(r#""key":"owned""#) && !d.contains(r#""key":"set_id""#), "{d}");

        let filtre = jsonschema::validator_for(&filter_condition_schema()).unwrap();
        for ok in [json!({}), json!({"field":{"key":"cost","value":[{"op":"lte","value":2}]}}), json!({"must":[{"field":{"key":"a","value":1}}]})] {
            assert!(filtre.is_valid(&ok), "{ok}");
        }
        assert!(filtre.is_valid(&json!({"must":[{"field":{"key":"a","value":1}}],"must_not":[],"should":[]})), "forme booléenne combinée");
        for ko in [json!({"owned":1}), json!({"field":{"key":"a"}})] {
            assert!(!filtre.is_valid(&ko), "{ko}");
        }
        let options = jsonschema::validator_for(&search_options_schema("…")).unwrap();
        assert!(options.is_valid(&json!({"limit":20,"filter_condition":{"field":{"key":"cost","value":[1]}}})));
        assert!(!options.is_valid(&json!({"filter":{"field":{"key":"cost","value":[1]}}})), "une clé inconnue est refusée");
    }
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

/// **Un champ inconnu est refusé avant la base**, avec les champs qui existent.
/// Il descendait jusqu'au moteur de requêtes : « Binder exception: Cannot find
/// property card_types for p » — vrai, et inutilisable par un agent, qui
/// réessayait le même champ (27 septembre 2026). Les jointures (`a.b`) et les
/// colonnes internes (`_…`) gardent leur sens ; les `nested` sont vérifiés
/// plus bas, dans leur portée.
/// À n'appeler que là où `fields` fait foi : une entité **dérivée** (une base
/// de connaissances traduite) filtre aussi sur les champs de ses sources.
pub fn check_field_names(c: &crate::filter::FilterCondition, fields: &HashMap<String, SimpleFieldDef>) -> Result<(), String> {
    use crate::filter::FilterCondition as C;
    match c {
        C::Field { key, .. } if !key.contains('.') && !key.starts_with('_') && !fields.contains_key(key) => {
            let mut known: Vec<&str> = fields.keys().map(String::as_str).filter(|k| !k.starts_with('_')).collect();
            known.sort_unstable();
            Err(format!("champ inconnu « {key} » pour cette entité ; champs filtrables : {}", known.join(", ")))
        }
        C::Must(cs) | C::Should(cs) | C::MustNot(cs) => cs.iter().try_for_each(|c| check_field_names(c, fields)),
        _ => Ok(()),
    }
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
