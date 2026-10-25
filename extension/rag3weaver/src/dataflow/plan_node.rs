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

/// **Un paramètre que le Plan ne lit pas est une faute de nom**, refusée en le nommant avec ceux
/// que le Plan lit. Perdu en silence, `sourc=$source` faisait disparaître `source`, et sous
/// `optional` le filtre tombait sans erreur.
fn refuser_les_inconnus<'a>(plan: &DeclaredPlan, cles: impl Iterator<Item = &'a String>) -> Result<(), String> {
    let lus = plan.param_names();
    let inconnus: Vec<&String> = cles.filter(|k| !lus.contains(*k)).collect();
    if inconnus.is_empty() {
        return Ok(());
    }
    let noms = |v: Vec<&String>| v.iter().map(|k| format!("« {k} »")).collect::<Vec<_>>().join(", ");
    Err(format!(
        "{NODE_TYPE}: le Plan ne lit pas {} — ses paramètres : {}",
        noms(inconnus),
        if lus.is_empty() { "aucun".to_string() } else { noms(lus.iter().collect()) }
    ))
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
        let mut params = match config.get("params") {
            None | Some(Value::Null) => serde_json::Map::new(),
            Some(Value::Object(m)) => m.clone(),
            Some(autre) => return Err(format!("{NODE_TYPE}: `params` n'est pas un objet ({autre})")),
        };
        // **Un paramètre par clé** : `PlanNode(plan=@plans/x.json, path=$path)`. Le Mermaid
        // n'écrit pas d'objet, mais il écrit une clé par paramètre de l'outil ; toute clé autre
        // que `plan` et `params` est un paramètre du Plan.
        if let Value::Object(cles) = config {
            for (k, v) in cles.iter().filter(|(k, _)| k.as_str() != "plan" && k.as_str() != "params") {
                params.insert(k.clone(), v.clone());
            }
        }
        refuser_les_inconnus(&plan, params.keys())?;
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
                    description: "les paramètres du Plan (un objet) ; toute autre clé de la config en est un aussi (`path=$path`) ; l'entrée params les remplace".into(),
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
            Some(Value::Object(m)) => {
                refuser_les_inconnus(&self.plan, m.keys())?;
                m.clone()
            }
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
        if let Some(f) = &lie.rows {
            let rows: Vec<Value> = lignes(f)?.iter().map(ligne_en_json).collect::<Result<_, _>>()?;
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

/// **Une ligne en objet JSON**, une clé par colonne. Une valeur qui ne se rend pas en JSON (un
/// `Blob`, une valeur typée de paramètre) refuse en nommant sa colonne : rendue `null` en entier,
/// la ligne disparaissait de `rows` sans erreur.
fn ligne_en_json(l: &BTreeMap<String, CypherValue>) -> Result<Value, String> {
    l.iter()
        .map(|(colonne, v)| {
            serde_json::to_value(v)
                .map(|v| (colonne.clone(), v))
                .map_err(|e| format!("{NODE_TYPE}: la colonne « {colonne} » ne se rend pas en JSON ({e}) — un Plan déclaré ne lit pas de Blob"))
        })
        .collect::<Result<serde_json::Map<_, _>, _>>()
        .map(Value::Object)
}

/// **Les Plans cités par un outil, lus et vérifiés**, à la place de leur citation : une seule
/// boucle, que le chargeur d'un manifeste (`backend.rs`) et le test des outils livrés
/// (`chaque_gabarit_d_outil_livre_se_construit`) appellent tous deux — un test qui recopierait la
/// boucle jugerait une autre résolution que celle du chargeur.
pub fn resoudre_les_plans(
    outil: &mut super::graph_tool::GraphTool,
    lire: impl Fn(&str) -> Result<String, String>,
    schema: &rag3weaver_ir::PlanSchema,
) -> Result<(), String> {
    let plans: Vec<(String, String)> = outil
        .template()
        .nodes
        .iter()
        .filter(|n| n.node_type == NODE_TYPE)
        .filter_map(|n| n.config.get("plan").and_then(Value::as_str).map(|c| (n.name.clone(), c.to_string())))
        .collect();
    for (noeud, cite) in plans {
        let lu = lire_le_plan(&cite, &lire, schema)?;
        outil.remplacer_config(&noeud, "plan", lu);
    }
    Ok(())
}

/// **Le Plan d'un `PlanNode`, lu au chargement** : `@chemin` relatif au
/// manifeste, lu, vérifié contre sa forme et contre `schema` ; le Plan lu
/// remplace la citation dans la configuration du nœud. Une erreur nomme le
/// fichier.
pub fn lire_le_plan(cite: &str, lire: impl Fn(&str) -> Result<String, String>, schema: &rag3weaver_ir::PlanSchema) -> Result<Value, String> {
    let Some(chemin) = cite.strip_prefix('@') else {
        return Err(format!("{NODE_TYPE}: `plan` se cite par @plans/<fichier>.json, pas « {cite} »"));
    };
    // **Un Plan livré** se cite `@builtin:<chemin relatif à templates/>`, comme un graphe livré :
    // il suit le binaire, et un manifeste relié ou installé ailleurs le trouve sans copier plans/.
    let texte = match chemin.strip_prefix(crate::builtin_templates::PREFIX) {
        Some(livre) => crate::builtin_templates::get(livre)
            .map(|o| String::from_utf8_lossy(o).into_owned())
            .ok_or_else(|| format!("{NODE_TYPE}: aucun Plan livré « {livre} »"))?,
        None => lire(chemin)?,
    };
    let plan = DeclaredPlan::from_json(&texte).map_err(|e| format!("{chemin} : {e}"))?;
    plan.check().map_err(|e| format!("{chemin} : {e}"))?;
    plan.check_schema(schema).map_err(|e| format!("{chemin} : {e}"))?;
    serde_json::to_value(&plan).map_err(|e| format!("{chemin} : {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le Plan d'`outline`, réduit : un chemin obligatoire, une source optionnelle, un plafond.
    fn plan() -> Value {
        json!({
            "let": {"du_fichier": {"all_of": [
                {"equals": {"field": "file_path", "param": "path"}},
                {"optional": {"equals": {"field": "source", "param": "source"}}}
            ]}},
            "forms": {"scopes": {"select": {"table": "Scope", "filter": {"let": "du_fichier"}, "returns": ["_uuid"],
                "order_by": [{"field": "_uuid"}], "limit": {"param": "limit", "max": 10}}}},
            "outputs": {"rows": "scopes"}
        })
    }

    /// **Une clé que le Plan ne lit pas est refusée, nommée** (relecture de l'arbre principal, 25
    /// octobre 2026) : `sourc=$source` perdait `source` en silence, et sous `optional` le filtre
    /// tombait — l'outil répondait sur toutes les sources, sans erreur.
    #[test]
    fn une_cle_que_le_plan_ne_lit_pas_est_refusee_en_la_nommant() {
        let bonne = json!({"plan": plan(), "path": "a.rs", "source": "s", "limit": 5});
        assert!(PlanNodeFactory.create("p", &bonne).is_ok(), "les paramètres du Plan passent");
        let mal_tapee = json!({"plan": plan(), "path": "a.rs", "sourc": "s"});
        let e = PlanNodeFactory.create("p", &mal_tapee).err().expect("une clé mal tapée se refuse");
        assert!(e.contains("sourc") && e.contains("source") && e.contains("path") && e.contains("limit"), "elle est nommée, avec ceux du Plan : {e}");
        let dans_params = json!({"plan": plan(), "params": {"path": "a.rs", "srce": "s"}});
        let e = PlanNodeFactory.create("p", &dans_params).err().expect("la même garde vaut pour params");
        assert!(e.contains("srce"), "{e}");
    }

    /// **Une ligne qui ne se rend pas en JSON refuse en nommant sa colonne** (25 octobre 2026) :
    /// un `Blob` faisait de la ligne entière un `null` dans `rows`, sans erreur.
    #[test]
    fn une_colonne_qui_ne_se_rend_pas_en_json_est_nommee() {
        let mut l = BTreeMap::new();
        l.insert("_uuid".to_string(), CypherValue::String("a".into()));
        l.insert("n".to_string(), CypherValue::Int(5));
        l.insert("liste".to_string(), CypherValue::List(vec![CypherValue::Int(1)]));
        assert_eq!(ligne_en_json(&l).unwrap(), json!({"_uuid": "a", "n": 5, "liste": [1]}), "un JSON naturel, sans étiquette de type");
        l.insert("brut".to_string(), CypherValue::Blob(vec![1, 2]));
        let e = ligne_en_json(&l).unwrap_err();
        assert!(e.contains("brut"), "{e}");
    }
}
