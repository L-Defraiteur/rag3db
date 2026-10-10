//! Le nœud entièrement scripté : une **déclaration** (un nom, des ports
//! d'entrée et de sortie chacun avec son JSON Schema, une configuration, un
//! langage, un script) devient un type de nœud qu'on enregistre sous son nom
//! et qu'on pose dans n'importe quel graphe comme un nœud fourni.
//!
//! Le script n'écrit qu'une fonction : `run({ inputs, config })` en TypeScript
//! ou en JavaScript (en rhai, l'expression lit `input.inputs` et
//! `input.config`) ; elle rend un objet dont chaque clé est un port de sortie.
//!
//! **Les ports vérifient à la frontière** : chaque entrée est validée contre
//! son schéma avant l'appel, chaque sortie après ; la configuration l'est à la
//! création du nœud. C'est ce qui tient lieu de types : TypeScript n'est
//! jamais vérifié (voir `crate::script`).
//!
//! Le script est préparé une fois, à la déclaration ; tous les nœuds créés
//! sous ce nom, dans tous les graphes, le partagent.
use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use crate::script::{PreparedScript, ScriptLimits};
use serde_json::{Map, Value};
use std::sync::Arc;

/// Un port déclaré : son nom et le schéma de ce qui y passe.
#[derive(Debug, Clone)]
pub struct DeclaredPort {
    pub name: String,
    pub schema: Value,
    pub required: bool,
}

/// Un paramètre de configuration déclaré.
#[derive(Debug, Clone)]
pub struct DeclaredParam {
    pub name: String,
    pub schema: Value,
    pub required: bool,
    pub default: Option<Value>,
    pub description: String,
}

/// La déclaration d'un nœud scripté, quelle que soit la forme lue.
#[derive(Debug, Clone)]
pub struct ScriptedNodeDecl {
    pub name: String,
    pub description: String,
    pub language: String,
    pub source: String,
    pub inputs: Vec<DeclaredPort>,
    pub outputs: Vec<DeclaredPort>,
    pub config: Vec<DeclaredParam>,
}

struct Checked {
    name: String,
    required: bool,
    validator: jsonschema::Validator,
}

struct Declared {
    decl: ScriptedNodeDecl,
    script: Arc<dyn PreparedScript>,
    inputs: Vec<Checked>,
    outputs: Vec<Checked>,
    config: Vec<Checked>,
    limits: ScriptLimits,
}

/// La fabrique d'un nœud scripté : la déclaration vérifiée, le script préparé.
pub struct ScriptedNodeFactory {
    declared: Arc<Declared>,
}

impl ScriptedNodeFactory {
    /// Vérifie la déclaration comme on vérifie un nœud : noms, ports, schémas,
    /// et le script lui-même (préparé ici, refusé ici). Toute erreur nomme ce
    /// qu'il faut corriger.
    pub fn new(decl: ScriptedNodeDecl, limits: ScriptLimits) -> Result<Self, String> {
        let at = |what: String| format!("scripted node '{}': {what}", decl.name);
        if !is_identifier(&decl.name) {
            return Err(at(
                "the name must be an identifier (letters, digits, _)".into()
            ));
        }
        if decl.outputs.is_empty() {
            return Err(at("declares no output port".into()));
        }
        let inputs = checked_ports("input", &decl.inputs).map_err(at)?;
        let outputs = checked_ports("output", &decl.outputs).map_err(at)?;
        let mut config = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for param in &decl.config {
            if !is_identifier(&param.name) || !seen.insert(param.name.as_str()) {
                return Err(at(format!(
                    "config parameter '{}' is not an identifier, or is declared twice",
                    param.name
                )));
            }
            let validator = validator(&param.schema)
                .map_err(|e| at(format!("config parameter '{}': {e}", param.name)))?;
            if let Some(default) = &param.default {
                if let Err(e) = validator.validate(default) {
                    return Err(at(format!(
                        "config parameter '{}': the default does not match its schema: {e}",
                        param.name
                    )));
                }
            }
            config.push(Checked {
                name: param.name.clone(),
                required: param.required,
                validator,
            });
        }
        let script = crate::script::prepare(&decl.language, &decl.source, &limits)
            .map_err(|e| at(format!("script: {e}")))?;
        Ok(Self {
            declared: Arc::new(Declared {
                decl,
                script,
                inputs,
                outputs,
                config,
                limits,
            }),
        })
    }
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn validator(schema: &Value) -> Result<jsonschema::Validator, String> {
    if !schema.is_object() && !schema.is_boolean() {
        return Err("its schema must be a JSON Schema object".into());
    }
    jsonschema::validator_for(schema).map_err(|e| format!("invalid schema: {e}"))
}

fn checked_ports(side: &str, ports: &[DeclaredPort]) -> Result<Vec<Checked>, String> {
    let mut seen = std::collections::BTreeSet::new();
    ports
        .iter()
        .map(|port| {
            if !is_identifier(&port.name) || !seen.insert(port.name.as_str()) {
                return Err(format!(
                    "{side} port '{}' is not an identifier, or is declared twice",
                    port.name
                ));
            }
            Ok(Checked {
                name: port.name.clone(),
                required: port.required,
                validator: validator(&port.schema)
                    .map_err(|e| format!("{side} port '{}': {e}", port.name))?,
            })
        })
        .collect()
}

/// Un flottant entier (dans la plage exacte des flottants, ±2⁵³) devient un
/// entier, partout dans la valeur.
fn canonical_numbers(value: Value) -> Value {
    const EXACT: f64 = 9_007_199_254_740_992.0;
    match value {
        Value::Number(n) => match n.as_f64() {
            Some(f) if !n.is_i64() && !n.is_u64() && f.fract() == 0.0 && f.abs() <= EXACT => {
                Value::from(f as i64)
            }
            _ => Value::Number(n),
        },
        Value::Array(items) => Value::Array(items.into_iter().map(canonical_numbers).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, canonical_numbers(v)))
                .collect(),
        ),
        other => other,
    }
}

fn port_defs(ports: &[Checked]) -> Vec<PortDef> {
    ports
        .iter()
        .map(|p| PortDef {
            name: p.name.clone().into(),
            port_type: PortType::Map,
            required: p.required,
        })
        .collect()
}

impl NodeFactory for ScriptedNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let declared = &self.declared;
        let at = |what: String| format!("{} '{name}': {what}", declared.decl.name);
        let given = match config {
            Value::Null => Map::new(),
            Value::Object(map) => map.clone(),
            _ => return Err(at("the configuration must be an object".into())),
        };
        if let Some(unknown) = given
            .keys()
            .find(|key| !declared.config.iter().any(|p| &p.name == *key))
        {
            return Err(at(format!("unknown config parameter '{unknown}'")));
        }
        let mut resolved = Map::new();
        for (param, decl) in declared.config.iter().zip(&declared.decl.config) {
            let value = match given.get(&param.name).or(decl.default.as_ref()) {
                Some(value) => value.clone(),
                None if param.required => {
                    return Err(at(format!("missing config parameter '{}'", param.name)))
                }
                None => continue,
            };
            if let Err(e) = param.validator.validate(&value) {
                return Err(at(format!("config parameter '{}': {e}", param.name)));
            }
            resolved.insert(param.name.clone(), value);
        }
        Ok(Box::new(ScriptedNode {
            name: name.to_string(),
            declared: declared.clone(),
            config: Value::Object(resolved),
        }))
    }

    fn node_type(&self) -> &str {
        &self.declared.decl.name
    }

    fn schema(&self) -> NodeSchema {
        let declared = &self.declared;
        NodeSchema {
            node_type: declared.decl.name.clone().into(),
            description: declared.decl.description.clone().into(),
            inputs: port_defs(&declared.inputs),
            outputs: port_defs(&declared.outputs),
            config_params: declared
                .decl
                .config
                .iter()
                .map(|p| ConfigParam {
                    name: p.name.clone().into(),
                    param_type: ConfigParamType::Json,
                    required: p.required,
                    default: p.default.clone(),
                    description: p.description.clone().into(),
                    choices: None,
                    json_schema: Some(p.schema.clone()),
                })
                .collect(),
        }
    }
}

struct ScriptedNode {
    name: String,
    declared: Arc<Declared>,
    config: Value,
}

impl Node for ScriptedNode {
    fn node_type(&self) -> &'static str {
        "ScriptedNode"
    }

    fn inputs(&self) -> Vec<PortDef> {
        port_defs(&self.declared.inputs)
    }

    fn outputs(&self) -> Vec<PortDef> {
        port_defs(&self.declared.outputs)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(self.config.clone()))
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let declared = &self.declared;
        let kind = &declared.decl.name;
        let mut inputs = Map::new();
        for port in &declared.inputs {
            let Some(value) = ctx.input(&port.name) else {
                if port.required {
                    return Err(format!(
                        "{kind}: input port '{}' received nothing",
                        port.name
                    ));
                }
                continue;
            };
            let Some(value) = value.downcast::<Value>() else {
                return Err(format!(
                    "{kind}: input port '{}' carries a value that is not JSON",
                    port.name
                ));
            };
            if let Err(e) = port.validator.validate(&value) {
                return Err(format!(
                    "{kind}: input port '{}' does not match its schema at '{}': {e}",
                    port.name,
                    e.instance_path()
                ));
            }
            inputs.insert(port.name.clone(), (*value).clone());
        }
        let argument = serde_json::json!({ "inputs": inputs, "config": self.config });
        let output = declared
            .script
            .call(&argument, &declared.limits)
            .map_err(|e| format!("{kind}: {e}"))?;
        // Un nombre JSON a une seule forme : rhai rend `125.0` là où
        // JavaScript rend `125` ; la sortie d'un nœud ne dépend pas du langage.
        let Value::Object(output) = canonical_numbers(output) else {
            return Err(format!(
                "{kind}: run must return an object keyed by output port"
            ));
        };
        if let Some(unknown) = output
            .keys()
            .find(|key| !declared.outputs.iter().any(|p| &p.name == *key))
        {
            return Err(format!(
                "{kind}: run returned '{unknown}', which is not an output port"
            ));
        }
        for port in &declared.outputs {
            match output.get(&port.name) {
                Some(value) => {
                    if let Err(e) = port.validator.validate(value) {
                        return Err(format!(
                            "{kind}: output port '{}' does not match its schema at '{}': {e}",
                            port.name,
                            e.instance_path()
                        ));
                    }
                    ctx.set_output(&port.name, PortValue::new(value.clone()));
                }
                None if port.required => {
                    return Err(format!(
                        "{kind}: run did not return output port '{}'",
                        port.name
                    ))
                }
                None => {}
            }
        }
        Ok(())
    }
}

// ─── La forme lue : nodes/<nom>.node.json à côté de son script ─────────────

/// Le fichier de déclaration, tel qu'écrit. Les clés inconnues sont refusées
/// en les nommant ; `script` est relatif au fichier, jamais absolu ni hors de
/// son dossier, pour qu'un dossier de déclarations se copie tel quel.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclFile {
    name: String,
    #[serde(default)]
    description: String,
    /// Absent : déduit de l'extension du script (`.ts`, `.js`, `.rhai`).
    language: Option<String>,
    script: String,
    #[serde(default)]
    inputs: Map<String, Value>,
    outputs: Map<String, Value>,
    #[serde(default)]
    config: Map<String, Value>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PortFile {
    schema: Value,
    #[serde(default = "yes")]
    required: bool,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ParamFile {
    schema: Value,
    #[serde(default)]
    required: bool,
    default: Option<Value>,
    #[serde(default)]
    description: String,
}

fn yes() -> bool {
    true
}

/// L'extension d'un fichier de déclaration de nœud.
pub const DECLARATION_SUFFIX: &str = ".node.json";

/// Lit une déclaration et son script.
pub fn read_declaration(path: &std::path::Path) -> Result<ScriptedNodeDecl, String> {
    let shown = path.display();
    let text = std::fs::read_to_string(path).map_err(|e| format!("{shown}: {e}"))?;
    let file: DeclFile = serde_json::from_str(&text).map_err(|e| format!("{shown}: {e}"))?;
    let relative = std::path::Path::new(&file.script);
    if relative.is_absolute()
        || relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "{shown}: script '{}' must be a path inside the declaration's folder",
            file.script
        ));
    }
    let language = match file.language {
        Some(language) => language,
        None => match relative.extension().and_then(|e| e.to_str()) {
            Some("ts") => "typescript".into(),
            Some("js") => "javascript".into(),
            Some("rhai") => "rhai".into(),
            _ => {
                return Err(format!(
                    "{shown}: no language declared, and none follows from '{}'",
                    file.script
                ))
            }
        },
    };
    let script_path = path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join(relative);
    let source = std::fs::read_to_string(&script_path)
        .map_err(|e| format!("{shown}: script '{}': {e}", file.script))?;
    let ports = |side: &str, map: Map<String, Value>| -> Result<Vec<DeclaredPort>, String> {
        map.into_iter()
            .map(|(name, value)| {
                let port: PortFile = serde_json::from_value(value)
                    .map_err(|e| format!("{shown}: {side} port '{name}': {e}"))?;
                Ok(DeclaredPort {
                    name,
                    schema: port.schema,
                    required: port.required,
                })
            })
            .collect()
    };
    let config = file
        .config
        .into_iter()
        .map(|(name, value)| {
            let param: ParamFile = serde_json::from_value(value)
                .map_err(|e| format!("{shown}: config parameter '{name}': {e}"))?;
            Ok(DeclaredParam {
                name,
                schema: param.schema,
                required: param.required,
                default: param.default,
                description: param.description,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(ScriptedNodeDecl {
        name: file.name,
        description: file.description,
        language,
        source,
        inputs: ports("input", file.inputs)?,
        outputs: ports("output", file.outputs)?,
        config,
    })
}

/// Toutes les déclarations d'un dossier (`*.node.json`, sans descendre), en
/// fabriques prêtes à enregistrer. Un dossier absent n'en déclare aucune ;
/// une seule déclaration fautive refuse le dossier, et chaque erreur est
/// rendue.
pub fn discover(
    dir: &std::path::Path,
    limits: &ScriptLimits,
) -> Result<Vec<ScriptedNodeFactory>, Vec<String>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(vec![format!("{}: {e}", dir.display())]),
    };
    let mut paths: Vec<_> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(DECLARATION_SUFFIX))
        })
        .collect();
    paths.sort();
    let mut factories = Vec::new();
    let mut errors = Vec::new();
    let mut names = std::collections::BTreeSet::new();
    for path in paths {
        match read_declaration(&path)
            .and_then(|decl| ScriptedNodeFactory::new(decl, limits.clone()))
        {
            Ok(factory) => {
                if names.insert(factory.node_type().to_string()) {
                    factories.push(factory);
                } else {
                    errors.push(format!(
                        "{}: node type '{}' is declared twice",
                        path.display(),
                        factory.node_type()
                    ));
                }
            }
            Err(e) => errors.push(e),
        }
    }
    if errors.is_empty() {
        Ok(factories)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests;
