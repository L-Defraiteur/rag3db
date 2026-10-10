//! L'arbre d'un filtre, sans sa compilation : la compilation parle au
//! dialecte et reste dans rag3weaver (`FilterParser`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Value;

/// A single filter operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", content = "value")]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Eq(Value),
    Neq(Value),
    Lt(Value),
    Lte(Value),
    Gt(Value),
    Gte(Value),
    In(Vec<Value>),
    HasAny(Vec<Value>),
    HasAll(Vec<Value>),
    HasNone(Vec<Value>),
    IsNull,
    IsNotNull,
    IsEmpty,
    IsNotEmpty,
    Between(Value, Value),
    NotIn(Vec<Value>),
    StartsWith(String),
    Contains(String),
    ValuesCount {
        min: Option<usize>,
        max: Option<usize>,
    },
}

/// A filter value: direct value, list (IN shorthand), or operator list.
///
/// **L'ordre des variantes est celui de la lecture** (`untagged` essaie dans
/// l'ordre). `Ops` d'abord : une liste d'objets `{op, value}` relue comme une
/// valeur directe — une `Value::List` de cartes — donnait un filtre
/// d'égalité sur des cartes, que rien ne compile, et une recherche **non
/// restreinte** annoncée au journal seulement. C'est ce qui arrivait dès
/// qu'un `SearchOptions` traversait du JSON (le lanceur, une fiche d'outil)
/// — trouvé le 6 septembre 2026 par le domaine de travail qui ne rétrécissait
/// plus rien.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    /// One or more operators (combined with AND).
    Ops(Vec<FilterOp>),
    /// Array shorthand for IN clause.
    List(Vec<Value>),
    /// Direct value: equality check, or IS NULL if `Value::Null`.
    Direct(Value),
}

/// Composable filter condition (Qdrant-like Must/Should/MustNot).
///
/// Désérialisé à la main (voir plus bas) : `{}` veut dire « aucun filtre »,
/// et une forme inconnue reçoit un message qui dit les formes attendues.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterCondition {
    /// Single field filter.
    Field { key: String, value: FilterValue },
    /// Explicit local object path; dots in `Field.key` still mean entity joins.
    Path { path: Vec<String>, value: FilterValue },
    /// At least one element must satisfy the WHOLE condition, recursively.
    Nested { path: Vec<String>, condition: Box<FilterCondition> },
    /// AND: all conditions must match.
    Must(Vec<FilterCondition>),
    /// OR: at least one condition must match.
    Should(Vec<FilterCondition>),
    /// NOT: none of the conditions must match.
    MustNot(Vec<FilterCondition>),
}

/// La forme dérivée de [`FilterCondition`], variante pour variante.
#[derive(Deserialize)]
#[serde(remote = "FilterCondition", rename_all = "snake_case")]
enum FilterConditionDef {
    Field { key: String, value: FilterValue },
    Path { path: Vec<String>, value: FilterValue },
    Nested { path: Vec<String>, condition: Box<FilterCondition> },
    Must(Vec<FilterCondition>),
    Should(Vec<FilterCondition>),
    MustNot(Vec<FilterCondition>),
}

/// **`{}` est « aucun filtre ».** Un modèle qui veut tout sélectionner écrit
/// `{"filter": {}}` ; le refuser avec « expected map with a single key » le
/// laissait boucler sur la même erreur (27 septembre 2026, session MTG). Le
/// reste suit la forme dérivée, avec les formes attendues dans l'erreur.
impl<'de> Deserialize<'de> for FilterCondition {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.as_object().is_some_and(|o| o.is_empty()) {
            return Ok(FilterCondition::Must(Vec::new()));
        }
        // **must / should / must_not ensemble**, la forme booléenne que les
        // modèles écrivent d'eux-mêmes (Qdrant, Elasticsearch) : tout `must`,
        // au moins un `should`, aucun `must_not`. Les listes vides ne
        // contraignent rien. Deux refus d'affilée pour un `"must_not": []`
        // ajouté par politesse, le 27 septembre 2026.
        if let Some(o) = value.as_object() {
            let booleennes = ["must", "should", "must_not"];
            if o.len() > 1 && o.keys().all(|k| booleennes.contains(&k.as_str())) {
                let liste = |k: &str| -> Result<Vec<FilterCondition>, D::Error> {
                    match o.get(k) {
                        None => Ok(Vec::new()),
                        Some(v) => serde_json::from_value(v.clone()).map_err(serde::de::Error::custom),
                    }
                };
                let (must, should, must_not) = (liste("must")?, liste("should")?, liste("must_not")?);
                let mut all = must;
                if !should.is_empty() {
                    all.push(FilterCondition::Should(should));
                }
                if !must_not.is_empty() {
                    all.push(FilterCondition::MustNot(must_not));
                }
                return Ok(FilterCondition::Must(all));
            }
        }
        FilterConditionDef::deserialize(value).map_err(|e| {
            serde::de::Error::custom(format!(
                "filtre invalide ({e}) : un objet à une seule clé parmi field, path, nested, must, should, must_not ; \
                 par exemple {{\"field\":{{\"key\":\"owned\",\"value\":[{{\"op\":\"gt\",\"value\":0}}]}}}} \
                 ou {{\"must\":[…]}} ; {{}} pour ne rien filtrer"
            ))
        })
    }
}

impl From<HashMap<String, FilterValue>> for FilterCondition {
    fn from(filters: HashMap<String, FilterValue>) -> Self {
        let fields: Vec<FilterCondition> = filters
            .into_iter()
            .map(|(key, value)| FilterCondition::Field { key, value })
            .collect();
        FilterCondition::Must(fields)
    }
}

// ─── From impls ─────────────────────────────────────────────────────────────

impl From<Value> for FilterValue {
    fn from(v: Value) -> Self {
        Self::Direct(v)
    }
}

impl From<&str> for FilterValue {
    fn from(s: &str) -> Self {
        Self::Direct(Value::from(s))
    }
}

impl From<i64> for FilterValue {
    fn from(n: i64) -> Self {
        Self::Direct(Value::from(n))
    }
}

impl From<f64> for FilterValue {
    fn from(f: f64) -> Self {
        Self::Direct(Value::from(f))
    }
}

impl From<bool> for FilterValue {
    fn from(b: bool) -> Self {
        Self::Direct(Value::from(b))
    }
}

impl From<Vec<Value>> for FilterValue {
    fn from(v: Vec<Value>) -> Self {
        Self::List(v)
    }
}
