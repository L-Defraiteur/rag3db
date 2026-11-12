//! Fusion à trois sans entrée-sortie. Une absence reste distincte d'une valeur nulle.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Divergence;

/// Les objets structurés peuvent descendre ensuite ; les autres contenus sont atomiques.
pub fn atomic<T: Eq + Clone>(
    base: Option<&T>,
    local: Option<&T>,
    target: Option<&T>,
) -> Result<Option<T>, Divergence> {
    if local == target || target == base {
        Ok(local.cloned())
    } else if local == base {
        Ok(target.cloned())
    } else {
        Err(Divergence)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conflict {
    /// Composants du chemin, sans ambiguïté sur les clés contenant '/' ou '~'.
    pub path: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub base: Option<Value>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub local: Option<Value>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub target: Option<Value>,
}

// Un champ présent valant null ne doit pas être décodé comme un champ absent.
fn present<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

impl Conflict {
    pub fn pointer(&self) -> String {
        self.path
            .iter()
            .map(|x| format!("/{}", x.replace('~', "~0").replace('/', "~1")))
            .collect()
    }
}

/// Les tableaux sont atomiques. On ne descend que si les trois valeurs sont des objets :
/// une suppression ou un changement de forme divergent reste un conflit à cet endroit.
pub fn json(
    base: Option<&Value>,
    local: Option<&Value>,
    target: Option<&Value>,
) -> Result<Option<Value>, Vec<Conflict>> {
    fn walk(
        base: Option<&Value>,
        local: Option<&Value>,
        target: Option<&Value>,
        path: &mut Vec<String>,
        errors: &mut Vec<Conflict>,
    ) -> Option<Value> {
        if let Ok(value) = atomic(base, local, target) {
            return value;
        }
        if let (Some(Value::Object(b)), Some(Value::Object(l)), Some(Value::Object(t))) =
            (base, local, target)
        {
            let keys: BTreeSet<&String> = b.keys().chain(l.keys()).chain(t.keys()).collect();
            let mut out = Map::new();
            for key in keys {
                path.push(key.clone());
                if let Some(value) = walk(b.get(key), l.get(key), t.get(key), path, errors) {
                    out.insert(key.clone(), value);
                }
                path.pop();
            }
            Some(Value::Object(out))
        } else {
            errors.push(Conflict {
                path: path.clone(),
                base: base.cloned(),
                local: local.cloned(),
                target: target.cloned(),
            });
            None
        }
    }
    let mut errors = Vec::new();
    let value = walk(base, local, target, &mut Vec::new(), &mut errors);
    if errors.is_empty() {
        Ok(value)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn disjoint_changes_and_identical_edits_merge() {
        let b = json!({"a":1,"b":2,"same":0});
        let l = json!({"a":3,"b":2,"same":5});
        let t = json!({"a":1,"b":4,"same":5});
        assert_eq!(
            super::json(Some(&b), Some(&l), Some(&t)).unwrap(),
            Some(json!({"a":3,"b":4,"same":5}))
        );
    }

    #[test]
    fn absence_null_and_concurrent_addition_are_distinct() {
        let b = json!({});
        let l = json!({"a":null});
        let t = json!({"a":1});
        let errors = super::json(Some(&b), Some(&l), Some(&t)).unwrap_err();
        assert_eq!(errors[0].base, None);
        assert_eq!(errors[0].local, Some(Value::Null));
        assert_eq!(errors[0].pointer(), "/a");
        assert_eq!(super::json(Some(&b), Some(&b), Some(&l)).unwrap(), Some(l));
    }

    #[test]
    fn wire_conflicts_preserve_absence_and_explicit_null() {
        let c = Conflict {
            path: vec!["x".into()],
            base: None,
            local: Some(Value::Null),
            target: Some(Value::Bool(true)),
        };
        let wire = serde_json::to_value(&c).unwrap();
        assert!(wire.get("base").is_none());
        assert_eq!(wire.get("local"), Some(&Value::Null));
        assert_eq!(serde_json::from_value::<Conflict>(wire).unwrap(), c);
    }

    #[test]
    fn removal_against_edit_and_changed_shape_refuse_before_descent() {
        let b = json!({"x":{"a":1}});
        let l = json!({});
        let t = json!({"x":{"a":2}});
        assert_eq!(
            super::json(Some(&b), Some(&l), Some(&t)).unwrap_err()[0].pointer(),
            "/x"
        );
        let scalar = json!({"x":0});
        assert_eq!(
            super::json(Some(&b), Some(&scalar), Some(&t)).unwrap_err()[0].pointer(),
            "/x"
        );
        assert_eq!(
            super::json(Some(&b), Some(&scalar), Some(&b)).unwrap(),
            Some(scalar)
        );
    }

    #[test]
    fn arrays_are_atomic_and_conflicts_are_ordered() {
        let b = json!({"z":[1],"a/b~":0});
        let l = json!({"z":[1,2],"a/b~":1});
        let t = json!({"z":[1,3],"a/b~":2});
        let e = super::json(Some(&b), Some(&l), Some(&t)).unwrap_err();
        assert_eq!(
            e.iter().map(Conflict::pointer).collect::<Vec<_>>(),
            ["/a~1b~0", "/z"]
        );
        assert_eq!(
            atomic(Some(&vec![1]), None, Some(&vec![2])),
            Err(Divergence)
        );
    }
}
