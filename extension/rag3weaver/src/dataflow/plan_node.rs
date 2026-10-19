//! **Le `PlanNode`** (19 octobre 2026) : un Plan déclaré
//! ([`rag3weaver_ir::DeclaredPlan`]), lu dans `plans/` au chargement du
//! gabarit, lié aux paramètres de l'appel, et lu par la vue de lecture — les
//! lignes et le total d'un même Plan dans le même instantané (L6).
//!
//! `PlanNode(plan=@plans/x.json, params=$params)` : `PreparedBackend::load`
//! remplace `@plans/x.json` par le Plan lu, vérifié (sa forme, et le schéma du
//! manifeste) ; un Plan faux refuse le gabarit. Les paramètres viennent de
//! `params` (la configuration), ou de l'entrée `params` quand un nœud en amont
//! les calcule (le `RhaiNode` de `cited_by`).
//!
//! Sorties : `rows` (une liste d'objets, un par ligne, nommés par `returns` ;
//! une liste liée, `collect`, sous son nom), `total` (le compte de
//! `outputs.total`), `results` (les lignes de `outputs.results` en
//! résultats, pour `RenderResultsNode` et `IntersectResultsNode`).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use rag3weaver_ir::{Column, DeclaredPlan, Form, Value as IrValue};
use serde_json::{json, Value};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use super::resultat::UnifiedResult;
use crate::catalog::Catalog;
use crate::connection::CypherValue;
use crate::search::SearchResult;

pub const NODE_TYPE: &str = "PlanNode";

/// Un `plan` encore cité (`@plans/…`) : le chargement ne l'a pas lu (un graphe
/// hors des outils d'un manifeste).
fn non_resolu(cite: &str) -> String {
    format!("{NODE_TYPE}: le Plan « {cite} » n'a pas été lu au chargement — un PlanNode se déclare dans un outil du manifeste")
}

pub struct PlanNodeFactory;

impl NodeFactory for PlanNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let plan = match config.get("plan") {
            Some(Value::String(cite)) => return Err(non_resolu(cite)),
            Some(v) => serde_json::from_value::<DeclaredPlan>(v.clone()).map_err(|e| format!("{NODE_TYPE}: Plan illisible : {e}"))?,
            None => return Err(format!("{NODE_TYPE}: il manque `plan`")),
        };
        plan.check().map_err(|e| format!("{NODE_TYPE}: {e}"))?;
        let params = match config.get("params") {
            None | Some(Value::Null) => serde_json::Map::new(),
            Some(Value::Object(m)) => m.clone(),
            Some(autre) => return Err(format!("{NODE_TYPE}: `params` n'est pas un objet ({autre})")),
        };
        Ok(Box::new(PlanNode { name: name.to_string(), plan, params }))
    }

    fn node_type(&self) -> &str {
        NODE_TYPE
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: NODE_TYPE.into(),
            description: "Un Plan déclaré (plans/…), lié aux paramètres de l'appel et lu par la vue : ses lignes, son total, ses résultats".into(),
            inputs: vec![PortDef::value("params", PortType::Map, false)],
            outputs: vec![
                PortDef::value("rows", PortType::Map, false),
                PortDef::value("total", PortType::Map, false),
                PortDef::value("results", PortType::Results, false),
            ],
            config_params: vec![
                ConfigParam {
                    name: "plan".into(),
                    param_type: ConfigParamType::Json,
                    required: true,
                    default: None,
                    description: "le Plan, cité par @plans/<fichier>.json, relatif au manifeste".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "params".into(),
                    param_type: ConfigParamType::Json,
                    required: false,
                    default: Some(json!({})),
                    description: "les paramètres du Plan (un objet) ; l'entrée params les remplace".into(),
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }

    // L6 : ne fait que lire (un Plan déclaré ne porte que select et count).
    fn snapshot_safe(&self, _config: &Value) -> bool {
        true
    }
}

pub struct PlanNode {
    name: String,
    plan: DeclaredPlan,
    params: serde_json::Map<String, Value>,
}

impl Node for PlanNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn node_type(&self) -> &'static str {
        NODE_TYPE
    }

    fn inputs(&self) -> Vec<PortDef> {
        PlanNodeFactory.schema().inputs
    }

    fn outputs(&self) -> Vec<PortDef> {
        PlanNodeFactory.schema().outputs
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let params = match ctx.input("params").and_then(|p| p.downcast::<Value>()) {
            Some(Value::Object(m)) => m.clone(),
            Some(autre) => return Err(format!("{NODE_TYPE}: l'entrée params n'est pas un objet ({autre})")),
            None => self.params.clone(),
        };
        let params: BTreeMap<String, IrValue> = params
            .into_iter()
            .map(|(k, v)| serde_json::from_value::<IrValue>(v).map(|v| (k, v)))
            .collect::<Result<_, _>>()
            .map_err(|e| format!("{NODE_TYPE}: un paramètre ne se lit pas : {e}"))?;
        let lie = self.plan.bind(&params).map_err(|e| format!("{NODE_TYPE}: {e}"))?;

        // **Par la vue**, sans le mutex (L4), dans l'instantané de l'appel (L6) :
        // toutes les formes du Plan lisent le même état.
        let catalog = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned().ok_or(format!("{NODE_TYPE}: 'catalog' service not found"))?;
        let vue = ctx.service::<Arc<crate::read_view::ReadViewCell>>(crate::read_view::READ_VIEW_SERVICE).cloned();
        let cell = ctx.service::<crate::scope::Scope>("scope").cloned();
        let entite = lie
            .results
            .as_ref()
            .map(|r| r.entity.clone())
            .or_else(|| lie.plan.nodes.iter().find_map(|n| if let Form::Select(s) = &n.form { Some(s.table.clone()) } else { None }))
            .unwrap_or_default();
        let lecture = crate::dataflow::catalog_read::read_for_tool(&catalog, vue.as_ref(), &entite, cell.as_ref());
        let (source, statut) = lecture.into_reader()?;
        if let Some(s) = statut {
            ctx.warn(&s);
        }
        let options = crate::plan::PlanOptions { no_transaction: true, ..Default::default() };
        let rapport = source.get().run_read_plan(&lie.plan, options).map_err(|e| format!("{NODE_TYPE}: {e:?}"))?;

        // Les lignes d'une sélection, en objets nommés par ses `returns`.
        let lignes = |form: &str| -> Result<Vec<BTreeMap<String, CypherValue>>, String> {
            let Some(Form::Select(s)) = lie.plan.get(form).map(|n| &n.form) else {
                return Err(format!("{NODE_TYPE}: « {form} » n'est pas une sélection"));
            };
            let noms: Vec<String> = s
                .returns
                .iter()
                .map(|c| match c {
                    Column::Node(f) => f.clone(),
                    Column::Collect(c) => c.alias.clone(),
                    _ => String::new(),
                })
                .collect();
            let lu = rapport.outputs.get(form).ok_or_else(|| format!("{NODE_TYPE}: la forme « {form} » n'a rien rendu"))?;
            Ok(lu
                .rows
                .iter()
                .map(|ligne| noms.iter().cloned().zip(ligne.iter().cloned()).filter(|(n, _)| !n.is_empty()).collect())
                .collect())
        };
        let en_json = |l: &BTreeMap<String, CypherValue>| serde_json::to_value(l).unwrap_or(Value::Null);

        if let Some(f) = &lie.rows {
            let rows: Vec<Value> = lignes(f)?.iter().map(en_json).collect();
            ctx.set_output("rows", PortValue::new(Value::Array(rows)));
        }
        if let Some(t) = &lie.total {
            let n = rapport
                .outputs
                .get(t)
                .and_then(|r| r.rows.first())
                .and_then(|l| l.first())
                .and_then(|v| if let CypherValue::Int(n) = v { Some(*n) } else { None })
                .ok_or_else(|| format!("{NODE_TYPE}: le compte « {t} » n'a pas rendu de nombre"))?;
            ctx.set_output("total", PortValue::new(json!(n)));
        }
        if let Some(r) = &lie.results {
            let mut resultats = Vec::new();
            for data in lignes(&r.form)? {
                let uuid = data
                    .get(&r.uuid)
                    .and_then(CypherValue::as_str)
                    .ok_or_else(|| format!("{NODE_TYPE}: une ligne de « {} » n'a pas de « {} »", r.form, r.uuid))?
                    .to_owned();
                let mut un = UnifiedResult::from(SearchResult {
                    uuid,
                    score: 1.0,
                    entity: Some(r.entity.clone()),
                    data: Some(data),
                    chunk: None,
                    chunks: None,
                });
                un.signal = Some(self.name.clone());
                resultats.push(un);
            }
            ctx.set_output("results", PortValue::new(resultats));
        }
        Ok(())
    }
}

/// **Le Plan d'un `PlanNode`, lu au chargement** : `@chemin` relatif au
/// manifeste, lu, vérifié contre sa forme et contre `schema` ; le Plan lu
/// remplace la citation dans la configuration du nœud. Une erreur nomme le
/// fichier.
pub fn lire_le_plan(cite: &str, lire: impl Fn(&str) -> Result<String, String>, schema: &rag3weaver_ir::PlanSchema) -> Result<Value, String> {
    let Some(chemin) = cite.strip_prefix('@') else {
        return Err(format!("{NODE_TYPE}: `plan` se cite par @plans/<fichier>.json, pas « {cite} »"));
    };
    let texte = lire(chemin)?;
    let plan = DeclaredPlan::from_json(&texte).map_err(|e| format!("{chemin} : {e}"))?;
    plan.check().map_err(|e| format!("{chemin} : {e}"))?;
    plan.check_schema(schema).map_err(|e| format!("{chemin} : {e}"))?;
    serde_json::to_value(&plan).map_err(|e| format!("{chemin} : {e}"))
}
