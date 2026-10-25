//! **Un sous-graphe cité, appelé une fois ou une fois par élément** (25 octobre
//! 2026 : l'archive de la mémoire longue, une écriture et une recherche par
//! section).
//!
//! Avant : un graphe ne pouvait en contenir un autre que si du code l'avait
//! enregistré comme type de nœud (`GraphTool::as_node_factory`) ; aucun
//! manifeste ne pouvait le déclarer, et un nœud d'écriture branché derrière un
//! flux d'éléments n'en voyait qu'un pli. Ici, deux nœuds déclarables :
//!
//! - `GraphNode(graph='@graphs/x.mmd', …)` : le graphe cité, appelé une fois ;
//! - `ForEachNode(graph='@graphs/x.mmd', …)` : appelé **une fois par élément**
//!   de son entrée `elements`, l'élément passé au paramètre `element` du
//!   graphe cité (`%% param: element json!`), les résultats rendus en flux dans
//!   l'ordre des éléments.
//!
//! Le graphe se cite comme un Plan : `@chemin` relatif au manifeste, ou
//! `@builtin:<chemin relatif à templates/>`. Le chargeur du manifeste le lit, le
//! lie au registre du backend et le vérifie ([`resoudre_les_sous_graphes`]) ; un
//! graphe cité absent ou faux refuse le gabarit en nommant le fichier, et ses
//! types de nœuds passent sous la politique de l'outil qui le contient (la liste
//! blanche ne s'élargit pas en silence). Les autres clés de la configuration
//! sont les paramètres du graphe cité, substitués comme ceux d'un outil ;
//! `$element.<champ>` atteint un champ de l'élément. Un sous-graphe tourne sous
//! le run de son parent : il voit ses services (catalogue, modèle, embarqueur,
//! cellule), et ses écritures entrent dans la même transaction.

use std::sync::{Arc, OnceLock, Weak};

use serde_json::{json, Map, Value};

use super::graph_tool::{build_definition, resolve_params, substitute_definition, GraphTool, NodeTypePolicy};
use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeRegistry, NodeSchema};
use super::port::{PortDef, PortKind, PortType, PortValue, Posture};
use super::services::ServiceRegistry;

pub const GRAPH_NODE: &str = "GraphNode";
pub const FOREACH_NODE: &str = "ForEachNode";

/// Au-delà, une citation qui en cite une autre est refusée : un cycle de
/// fichiers ne se déroule pas sans fin.
const PROFONDEUR_MAX: usize = 8;

/// **Le registre que voient les sous-graphes cités** : celui du backend, posé
/// une fois bâti. Faible, pour que le registre (qui contient les fabriques qui le
/// tiennent) ne se retienne pas lui-même.
#[derive(Clone, Default)]
pub struct RegistreDesSousGraphes(Arc<OnceLock<Weak<NodeRegistry>>>);

impl RegistreDesSousGraphes {
    pub fn poser(&self, registre: &Arc<NodeRegistry>) {
        let _ = self.0.set(Arc::downgrade(registre));
    }

    fn get(&self, qui: &str) -> Result<Arc<NodeRegistry>, String> {
        self.0
            .get()
            .and_then(Weak::upgrade)
            .ok_or_else(|| format!("{qui} : le registre des nœuds n'est pas posé — un sous-graphe cité se déclare dans un outil du manifeste"))
    }
}

/// Le nom du paramètre qui reçoit l'élément (`element=`), `element` par défaut.
fn parametre_de_l_element(config: &Value) -> &str {
    config.get("element").and_then(Value::as_str).unwrap_or("element")
}

/// **Les sous-graphes cités d'un outil, lus et vérifiés** à la place de leur
/// citation, comme les Plans ([`super::plan_node::resoudre_les_plans`]). La
/// configuration `graph` d'un `GraphNode` ou d'un `ForEachNode` devient
/// `{cite, mermaid, sous}` : le texte du graphe cité, et pour chacun de ses
/// propres nœuds cités, le même arbre. Rend les types de nœuds de tous les
/// sous-graphes, pour la politique de l'outil.
pub fn resoudre_les_sous_graphes(
    outil: &mut GraphTool,
    lire: &dyn Fn(&str) -> Result<String, String>,
    nodes: &NodeRegistry,
) -> Result<Vec<String>, String> {
    let mut types = Vec::new();
    let cites: Vec<(String, String, Value)> = outil
        .template()
        .nodes
        .iter()
        .filter(|n| n.node_type == GRAPH_NODE || n.node_type == FOREACH_NODE)
        .filter_map(|n| n.config.get("graph").and_then(Value::as_str).map(|c| (n.name.clone(), c.to_string(), n.config.clone())))
        .collect();
    for (noeud, cite, config) in cites {
        let par_element = outil.template().nodes.iter().any(|n| n.name == noeud && n.node_type == FOREACH_NODE);
        let arbre = lire_un_sous_graphe(&cite, &config, par_element, lire, nodes, &mut types, 1).map_err(|e| format!("le nœud « {noeud} » : {e}"))?;
        outil.remplacer_config(&noeud, "graph", arbre);
    }
    Ok(types)
}

fn lire_un_sous_graphe(
    cite: &str,
    config: &Value,
    par_element: bool,
    lire: &dyn Fn(&str) -> Result<String, String>,
    nodes: &NodeRegistry,
    types: &mut Vec<String>,
    profondeur: usize,
) -> Result<Value, String> {
    if profondeur > PROFONDEUR_MAX {
        return Err(format!("« {cite} » : plus de {PROFONDEUR_MAX} sous-graphes cités les uns dans les autres — un cycle de citations ?"));
    }
    let Some(chemin) = cite.strip_prefix('@') else {
        return Err(format!("`graph` se cite par @graphs/<fichier>.mmd ou @builtin:<chemin>, pas « {cite} »"));
    };
    let texte = match chemin.strip_prefix(crate::builtin_templates::PREFIX) {
        Some(livre) => crate::builtin_templates::get(livre)
            .map(|o| String::from_utf8_lossy(o).into_owned())
            .ok_or_else(|| format!("aucun graphe livré « {livre} »"))?,
        None => lire(chemin).map_err(|e| format!("{chemin} : {e}"))?,
    };
    let outil = GraphTool::from_mermaid(&texte).and_then(|t| t.bind(nodes)).map_err(|e| format!("{chemin} : {e}"))?;
    let element = parametre_de_l_element(config);
    if par_element && !outil.params().iter().any(|p| p.name == element) {
        return Err(format!("{chemin} : le graphe ne déclare pas le paramètre « {element} » qui reçoit l'élément (`%% param: {element} json!`)"));
    }
    // **Jamais l'élément dans un script** : substituer le texte d'un document
    // dans le code d'un nœud scripté, ce serait l'exécuter.
    for n in &outil.template().nodes {
        if let Some(cle) = script_qui_lit(&n.config, element) {
            return Err(format!(
                "{chemin} : le nœud « {} » substitue ${element} dans sa clé « {cle} » — un script lit l'élément par son entrée, jamais par substitution",
                n.name
            ));
        }
    }
    let mut sous = Map::new();
    for n in &outil.template().nodes {
        types.push(n.node_type.clone());
        if n.node_type == GRAPH_NODE || n.node_type == FOREACH_NODE {
            if let Some(c) = n.config.get("graph").and_then(Value::as_str) {
                let arbre = lire_un_sous_graphe(c, &n.config, n.node_type == FOREACH_NODE, lire, nodes, types, profondeur + 1).map_err(|e| format!("{chemin}, le nœud « {} » : {e}", n.name))?;
                sous.insert(n.name.clone(), arbre);
            }
        }
    }
    Ok(json!({ "cite": cite, "mermaid": texte, "sous": sous }))
}

/// La clé d'un `script` qui contient `$<element>`, s'il y en a une.
fn script_qui_lit(config: &Value, element: &str) -> Option<String> {
    let marque = format!("${element}");
    match config {
        Value::Object(o) => o.iter().find_map(|(k, v)| match v {
            Value::String(s) if k.contains("script") && s.contains(&marque) => Some(k.clone()),
            Value::Object(_) | Value::Array(_) => script_qui_lit(v, element),
            _ => None,
        }),
        Value::Array(a) => a.iter().find_map(|v| script_qui_lit(v, element)),
        _ => None,
    }
}

/// Le graphe cité d'une configuration résolue, rematérialisé et lié.
fn materialiser(config: &Value, registre: &NodeRegistry, qui: &str) -> Result<(String, GraphTool), String> {
    let arbre = match config.get("graph") {
        Some(Value::Object(o)) => o,
        Some(Value::String(c)) => return Err(format!("{qui} : le graphe « {c} » n'a pas été lu au chargement — ce nœud se déclare dans un outil du manifeste")),
        _ => return Err(format!("{qui} : il manque `graph`")),
    };
    let cite = arbre.get("cite").and_then(Value::as_str).unwrap_or("?").to_string();
    let texte = arbre.get("mermaid").and_then(Value::as_str).ok_or_else(|| format!("{qui} : `graph` sans texte"))?;
    let mut outil = GraphTool::from_mermaid(texte).map_err(|e| format!("{qui}, {cite} : {e}"))?;
    if let Some(Value::Object(sous)) = arbre.get("sous") {
        for (noeud, a) in sous {
            outil.remplacer_config(noeud, "graph", a.clone());
        }
    }
    let outil = outil.bind(registre).map_err(|e| format!("{qui}, {cite} : {e}"))?;
    Ok((cite, outil))
}

/// Les clés de la configuration propres au nœud, qui ne sont pas des paramètres du graphe cité.
const CLES_DU_NOEUD: [&str; 5] = ["graph", "element", "result", "on_error", "parallelism"];

/// Les paramètres du graphe cité, lus dans la configuration (hors l'élément).
fn parametres(outil: &GraphTool, config: &Value, sauf: Option<&str>, qui: &str) -> Result<Map<String, Value>, String> {
    let mut args = Map::new();
    if let Value::Object(o) = config {
        for (k, v) in o.iter().filter(|(k, _)| !CLES_DU_NOEUD.contains(&k.as_str())) {
            args.insert(k.clone(), v.clone());
        }
    }
    let params: Vec<ConfigParam> = outil.params().iter().filter(|p| Some(p.name.as_ref()) != sauf).cloned().collect();
    resolve_params(&params, &Value::Object(args)).map_err(|e| format!("{qui} : {e}"))
}

/// Le port du résultat : `result='nœud.port'`, sinon le `%% result:` du graphe cité.
fn resultat(outil: &GraphTool, config: &Value) -> (String, String) {
    match config.get("result").and_then(Value::as_str).and_then(|r| r.split_once('.')) {
        Some((n, p)) => (n.to_string(), p.to_string()),
        None => {
            let (n, p) = outil.result();
            (n.to_string(), p.to_string())
        }
    }
}

/// **Un appel du graphe cité** sous le run du parent : ses services, et
/// l'annulation du parent qui l'arrête.
fn appeler(
    outil: &GraphTool,
    args: &Map<String, Value>,
    registre: &NodeRegistry,
    services: Arc<ServiceRegistry>,
    annulation: &super::node::CancelToken,
    (rn, rp): (&str, &str),
) -> Result<PortValue, String> {
    let def = substitute_definition(outil.template(), args);
    let mut graphe = build_definition(&def, registre, &NodeTypePolicy::All).map_err(|e| e.to_string())?;
    let runtime = super::runtime::DataflowRuntime::with_services_arc(def.nodes.len().max(1), services);
    let stop = runtime.cancel_handle();
    annulation.on_cancel(move || stop.cancel());
    let sortie = runtime.execute(&mut graphe)?;
    sortie.get(rn, rp).cloned().ok_or_else(|| match sortie.skip_reason(rn) {
        Some(r) => format!("le résultat {rn}.{rp} n'a pas été produit : {r}"),
        None => format!("le résultat {rn}.{rp} n'a pas été produit"),
    })
}

/// Les services d'un appel : ceux du parent, marqués de son run.
fn services_du_parent(ctx: &NodeContext) -> Arc<ServiceRegistry> {
    if ctx.run_id().is_empty() {
        ctx.services_arc()
    } else {
        let mut couche = ServiceRegistry::layered(ctx.services_arc());
        couche.register("parent_run", ctx.run_id().to_string());
        Arc::new(couche)
    }
}

/// Un élément en JSON : une valeur JSON telle quelle, sinon sa forme de point de reprise.
fn en_json(v: &PortValue) -> Result<Value, String> {
    if let Some(j) = v.downcast::<Value>() {
        return Ok(j.clone());
    }
    if let Some(s) = v.downcast::<String>() {
        return Ok(Value::String(s.clone()));
    }
    let cp = super::checkpoint::port_value_to_checkpoint(v)?;
    match cp.data_json {
        Some(t) => serde_json::from_str(&t).map_err(|e| e.to_string()),
        None => Err("un élément sans valeur lisible en JSON".into()),
    }
}

/// **Un champ nommé et absent refuse l'appel** : `$element.titre` sur un élément
/// sans `titre` ne devient pas une chaîne vide.
fn champs_manquants(outil: &GraphTool, element: &str, valeur: &Value) -> Option<String> {
    let marque = format!("${element}.");
    let mut manquants = Vec::new();
    for n in &outil.template().nodes {
        let mut chaines = Vec::new();
        collecter(&n.config, &mut chaines);
        for s in chaines {
            let mut reste = s.as_str();
            while let Some(i) = reste.find(&marque) {
                reste = &reste[i + marque.len()..];
                let fin = reste.find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.').unwrap_or(reste.len());
                let chemin = reste[..fin].trim_end_matches('.');
                let mut v = valeur;
                let mut trouve = true;
                for part in chemin.split('.').filter(|p| !p.is_empty()) {
                    match v.get(part) {
                        Some(x) => v = x,
                        None => {
                            trouve = false;
                            break;
                        }
                    }
                }
                if !trouve {
                    manquants.push(format!("{element}.{chemin} (nœud « {} »)", n.name));
                }
                reste = &reste[fin..];
            }
        }
    }
    (!manquants.is_empty()).then(|| manquants.join(", "))
}

fn collecter<'a>(v: &'a Value, out: &mut Vec<&'a String>) {
    match v {
        Value::String(s) => out.push(s),
        Value::Array(a) => a.iter().for_each(|x| collecter(x, out)),
        Value::Object(o) => o.values().for_each(|x| collecter(x, out)),
        _ => {}
    }
}

fn param(name: &str, t: ConfigParamType, required: bool, default: Option<Value>, description: &str, choices: Option<Choices>) -> ConfigParam {
    ConfigParam { name: name.to_string().into(), param_type: t, required, default, description: description.to_string().into(), choices, json_schema: None }
}

// ─── GraphNode cité ─────────────────────────────────────────────────────────

pub struct CitedGraphNodeFactory {
    pub registre: RegistreDesSousGraphes,
}

impl NodeFactory for CitedGraphNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let qui = format!("{GRAPH_NODE} « {name} »");
        let registre = self.registre.get(&qui)?;
        let (cite, outil) = materialiser(config, &registre, &qui)?;
        let args = parametres(&outil, config, None, &qui)?;
        let resultat = resultat(&outil, config);
        Ok(Box::new(CitedGraphNode { name: name.to_string(), cite, outil, args, resultat, registre }))
    }

    fn node_type(&self) -> &str {
        GRAPH_NODE
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: GRAPH_NODE.into(),
            description: "Un graphe cité (@graphs/x.mmd, @builtin:…), appelé une fois : ses paramètres sont les autres clés de la configuration".into(),
            inputs: vec![PortDef::value("trigger", PortType::Any, false)],
            outputs: vec![PortDef::value("result", PortType::Any, false)],
            config_params: vec![
                param("graph", ConfigParamType::Json, true, None, "le graphe, cité par @graphs/<fichier>.mmd (relatif au manifeste) ou @builtin:<chemin>", None),
                param("result", ConfigParamType::String, false, None, "le port du résultat, nœud.port (défaut : le %% result du graphe cité)", None),
            ],
        }
    }
}

struct CitedGraphNode {
    name: String,
    cite: String,
    outil: GraphTool,
    args: Map<String, Value>,
    resultat: (String, String),
    registre: Arc<NodeRegistry>,
}

impl Node for CitedGraphNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        GRAPH_NODE
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("trigger", PortType::Any, false)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("result", PortType::Any, false)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let services = services_du_parent(ctx);
        let v = appeler(&self.outil, &self.args, &self.registre, services, &ctx.cancellation(), (&self.resultat.0, &self.resultat.1))
            .map_err(|e| format!("{GRAPH_NODE} « {} » ({}) : {e}", self.name, self.cite))?;
        ctx.set_output("result", v);
        Ok(())
    }
}

// ─── ForEachNode ────────────────────────────────────────────────────────────

pub struct ForEachNodeFactory {
    pub registre: RegistreDesSousGraphes,
}

impl NodeFactory for ForEachNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let qui = format!("{FOREACH_NODE} « {name} »");
        let registre = self.registre.get(&qui)?;
        let (cite, outil) = materialiser(config, &registre, &qui)?;
        let element = parametre_de_l_element(config).to_string();
        if !outil.params().iter().any(|p| p.name == element.as_str()) {
            return Err(format!("{qui}, {cite} : le graphe ne déclare pas le paramètre « {element} » (`%% param: {element} json!`)"));
        }
        let args = parametres(&outil, config, Some(&element), &qui)?;
        let on_error = match config.get("on_error").and_then(Value::as_str).unwrap_or("stop") {
            "stop" => false,
            "report" => true,
            autre => return Err(format!("{qui} : on_error vaut stop ou report, pas « {autre} »")),
        };
        let parallelism = match config.get("parallelism") {
            None | Some(Value::Null) => 1,
            Some(v) => v.as_u64().filter(|n| (1..=64).contains(n)).ok_or_else(|| format!("{qui} : parallelism est un entier de 1 à 64, pas {v}"))? as usize,
        };
        let resultat = resultat(&outil, config);
        Ok(Box::new(ForEachNode { name: name.to_string(), cite, outil, element, args, resultat, on_error, parallelism, registre }))
    }

    fn node_type(&self) -> &str {
        FOREACH_NODE
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: FOREACH_NODE.into(),
            description: "Un graphe cité, appelé une fois par élément de `elements` (l'élément au paramètre `element`), ses résultats en flux dans l'ordre, et un rapport".into(),
            inputs: ports_d_entree(),
            outputs: ports_de_sortie(),
            config_params: vec![
                param("graph", ConfigParamType::Json, true, None, "le graphe, cité par @graphs/<fichier>.mmd (relatif au manifeste) ou @builtin:<chemin>", None),
                param("element", ConfigParamType::String, false, Some(json!("element")), "le paramètre du graphe cité qui reçoit l'élément ($element, $element.<champ>)", None),
                param("result", ConfigParamType::String, false, None, "le port du résultat, nœud.port (défaut : le %% result du graphe cité)", None),
                param("on_error", ConfigParamType::String, false, Some(json!("stop")), "stop : l'échec d'un élément arrête le run ; report : {error, element, index} dans le flux, et la suite continue", Some(Choices::fixed(["stop", "report"]))),
                param("parallelism", ConfigParamType::Int, false, Some(json!(1)), "appels en même temps, au plus (1 : l'un après l'autre) ; l'ordre des résultats est celui des éléments", None),
            ],
        }
    }
}

fn ports_d_entree() -> Vec<PortDef> {
    vec![PortDef::value("elements", PortType::Any, true).posture(Posture::PerElement)]
}

fn ports_de_sortie() -> Vec<PortDef> {
    vec![PortDef::value("results", PortType::Any, false).stream(PortKind::Finite), PortDef::value("report", PortType::Map, false)]
}

struct ForEachNode {
    name: String,
    cite: String,
    outil: GraphTool,
    element: String,
    args: Map<String, Value>,
    resultat: (String, String),
    on_error: bool,
    parallelism: usize,
    registre: Arc<NodeRegistry>,
}

impl Node for ForEachNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        FOREACH_NODE
    }
    fn inputs(&self) -> Vec<PortDef> {
        ports_d_entree()
    }
    fn outputs(&self) -> Vec<PortDef> {
        ports_de_sortie()
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let qui = format!("{FOREACH_NODE} « {} » ({})", self.name, self.cite);
        // Les éléments : un flux, ou une valeur — une liste se lit alors élément
        // par élément.
        let mut elements: Vec<Value> = Vec::new();
        while let Some(v) = ctx.next("elements")? {
            elements.push(en_json(&v).map_err(|e| format!("{qui} : l'élément {} : {e}", elements.len()))?);
        }
        if let [Value::Array(liste)] = elements.as_slice() {
            elements = liste.clone();
        }
        let services = services_du_parent(ctx);
        let annulation = ctx.cancellation();
        let (rn, rp) = (self.resultat.0.as_str(), self.resultat.1.as_str());
        let appel = |i: usize, e: &Value| -> Result<PortValue, String> {
            if let Some(m) = champs_manquants(&self.outil, &self.element, e) {
                return Err(format!("l'élément {i} n'a pas {m}"));
            }
            let mut args = self.args.clone();
            args.insert(self.element.clone(), e.clone());
            appeler(&self.outil, &args, &self.registre, services.clone(), &annulation, (rn, rp))
        };
        let mut sorties: Vec<Result<PortValue, String>> = Vec::with_capacity(elements.len());
        // Par fenêtres de `parallelism` appels ; en stop, un échec arrête les
        // fenêtres suivantes (aucun élément de plus n'est appelé, donc rien de
        // plus n'est écrit).
        for (debut, fenetre) in elements.chunks(self.parallelism).enumerate().map(|(k, f)| (k * self.parallelism, f)) {
            if fenetre.len() == 1 {
                sorties.push(appel(debut, &fenetre[0]));
            } else {
                let appel = &appel;
                let rendus: Vec<Result<PortValue, String>> = std::thread::scope(|s| {
                    let fils: Vec<_> = fenetre.iter().enumerate().map(|(j, e)| s.spawn(move || appel(debut + j, e))).collect();
                    fils.into_iter().map(|f| f.join().unwrap_or_else(|_| Err("l'appel a paniqué".into()))).collect()
                });
                sorties.extend(rendus);
            }
            if !self.on_error && sorties.iter().any(|r| r.is_err()) {
                break;
            }
        }
        let mut erreurs = Vec::new();
        for (i, r) in sorties.into_iter().enumerate() {
            match r {
                Ok(v) => match ctx.emit("results", v) {
                    Ok(()) | Err(super::node::Stopped::NoReader) => {}
                    Err(e) => return Err(e.into()),
                },
                Err(e) if !self.on_error => return Err(format!("{qui} : {e}")),
                Err(e) => {
                    erreurs.push(json!({"index": i, "error": e}));
                    let rendu = json!({"index": i, "error": e, "element": elements[i]});
                    match ctx.emit("results", PortValue::new(rendu)) {
                        Ok(()) | Err(super::node::Stopped::NoReader) => {}
                        Err(e) => return Err(e.into()),
                    }
                }
            }
        }
        let n = elements.len();
        ctx.set_output("report", PortValue::new(json!({"elements": n, "ok": n - erreurs.len(), "errors": erreurs})));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::graph::DataflowGraph;
    use crate::dataflow::node_factories::register_builtins;
    use crate::dataflow::runtime::DataflowRuntime;
    use std::sync::Mutex;

    type Journal = Arc<Mutex<Vec<String>>>;

    /// **Le faux sous-graphe qui écrit** : il note `valeur` dans le journal du
    /// run (le service `journal`), et échoue si `echoue` vaut vrai.
    struct Noter {
        name: String,
        valeur: String,
        echoue: bool,
    }

    impl Node for Noter {
        fn name(&self) -> &str {
            &self.name
        }
        fn node_type(&self) -> &str {
            "NoterNode"
        }
        fn inputs(&self) -> Vec<PortDef> {
            vec![]
        }
        fn outputs(&self) -> Vec<PortDef> {
            vec![PortDef::value("text", PortType::Text, false)]
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            if self.echoue {
                return Err(format!("refus voulu sur {}", self.valeur));
            }
            let journal = ctx.service::<Journal>("journal").cloned().ok_or("pas de journal")?;
            journal.lock().unwrap().push(format!("écrit {}", self.valeur));
            ctx.set_output("text", PortValue::new(format!("rendu {}", self.valeur)));
            Ok(())
        }
    }

    struct NoterFactory;

    impl NodeFactory for NoterFactory {
        fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
            Ok(Box::new(Noter {
                name: name.to_string(),
                valeur: config.get("valeur").and_then(Value::as_str).unwrap_or_default().to_string(),
                echoue: config.get("echoue").and_then(Value::as_bool).unwrap_or(false),
            }))
        }
        fn node_type(&self) -> &str {
            "NoterNode"
        }
        fn schema(&self) -> NodeSchema {
            NodeSchema {
                node_type: "NoterNode".into(),
                description: "note sa valeur".into(),
                inputs: vec![],
                outputs: vec![PortDef::value("text", PortType::Text, false)],
                config_params: vec![
                    param("valeur", ConfigParamType::String, true, None, "", None),
                    param("echoue", ConfigParamType::Bool, false, Some(json!(false)), "", None),
                ],
            }
        }
    }

    /// Une source : les éléments en flux fini.
    struct Sections {
        name: String,
        elements: Vec<Value>,
    }

    impl Node for Sections {
        fn name(&self) -> &str {
            &self.name
        }
        fn node_type(&self) -> &str {
            "Sections"
        }
        fn inputs(&self) -> Vec<PortDef> {
            vec![]
        }
        fn outputs(&self) -> Vec<PortDef> {
            vec![PortDef::value("out", PortType::Any, true).stream(PortKind::Finite)]
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            for e in &self.elements {
                ctx.emit("out", PortValue::new(e.clone()))?;
            }
            Ok(())
        }
    }

    /// Lit les résultats élément par élément, dans le journal (l'ordre du flux).
    struct Recueil {
        name: String,
    }

    impl Node for Recueil {
        fn name(&self) -> &str {
            &self.name
        }
        fn node_type(&self) -> &str {
            "Recueil"
        }
        fn inputs(&self) -> Vec<PortDef> {
            vec![PortDef::value("in", PortType::Any, true).posture(Posture::PerElement)]
        }
        fn outputs(&self) -> Vec<PortDef> {
            vec![]
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            let journal = ctx.service::<Journal>("journal").cloned().ok_or("pas de journal")?;
            while let Some(v) = ctx.next("in")? {
                let lu = match (v.downcast::<String>(), v.downcast::<Value>()) {
                    (Some(s), _) => s.clone(),
                    (None, Some(j)) => j.to_string(),
                    _ => "?".into(),
                };
                journal.lock().unwrap().push(format!("reçu {lu}"));
            }
            Ok(())
        }
    }

    const SECTION: &str = "%% tool: section\n%% description: une section\n%% param: element json! -- la section\n%% result: note.text\n\ngraph LR\n    note[\"NoterNode(valeur='$element.title', echoue=$element.echoue)\"]\n";

    fn registre() -> Arc<NodeRegistry> {
        let mut r = NodeRegistry::new();
        register_builtins(&mut r);
        r.register(Box::new(NoterFactory));
        Arc::new(r)
    }

    fn section(title: &str, echoue: bool) -> Value {
        json!({"title": title, "echoue": echoue})
    }

    /// Joue `ForEachNode` sur `elements` ; rend le journal, le rapport, ou l'échec.
    fn jouer(elements: Vec<Value>, config: Value) -> (Vec<String>, Result<Value, String>) {
        let reg = registre();
        let sous = RegistreDesSousGraphes::default();
        sous.poser(&reg);
        let mut config = config;
        config["graph"] = json!({"cite": "@graphs/section.mmd", "mermaid": SECTION, "sous": {}});
        let journal: Journal = Arc::default();
        let mut services = ServiceRegistry::new();
        services.register("journal", journal.clone());
        let mut g = DataflowGraph::new();
        g.add_node(Box::new(Sections { name: "sections".into(), elements })).unwrap();
        let chaque = ForEachNodeFactory { registre: sous }.create("chaque", &config);
        let chaque = match chaque {
            Ok(n) => n,
            Err(e) => return (Vec::new(), Err(e)),
        };
        g.add_node(chaque).unwrap();
        g.add_node(Box::new(Recueil { name: "recueil".into() })).unwrap();
        g.connect("sections", "out", "chaque", "elements").unwrap();
        g.connect("chaque", "results", "recueil", "in").unwrap();
        let r = DataflowRuntime::with_services(100, services).execute(&mut g);
        let rapport = r.map(|o| o.get("chaque", "report").and_then(|v| v.downcast::<Value>().cloned()).unwrap_or(Value::Null));
        let lignes = journal.lock().unwrap().clone();
        (lignes, rapport)
    }

    /// **N éléments, N appels et N écritures, dans l'ordre** ; le rapport les compte.
    #[test]
    fn chaque_element_appelle_le_sous_graphe_et_ecrit_dans_l_ordre() {
        let (journal, rapport) = jouer((1..=4).map(|i| section(&format!("s{i}"), false)).collect(), json!({}));
        let rapport = rapport.unwrap();
        assert_eq!(rapport, json!({"elements": 4, "ok": 4, "errors": []}));
        let ecrits: Vec<&String> = journal.iter().filter(|l| l.starts_with("écrit")).collect();
        assert_eq!(ecrits, ["écrit s1", "écrit s2", "écrit s3", "écrit s4"], "{journal:?}");
        let recus: Vec<&String> = journal.iter().filter(|l| l.starts_with("reçu")).collect();
        assert_eq!(recus, ["reçu rendu s1", "reçu rendu s2", "reçu rendu s3", "reçu rendu s4"], "les résultats en flux, dans l'ordre : {journal:?}");
    }

    /// **En stop, l'échec du k-ième arrête tout** : le run échoue en nommant
    /// l'élément, et aucun élément suivant n'est appelé ni écrit.
    #[test]
    fn en_stop_l_echec_d_un_element_arrete_le_run_sans_appeler_la_suite() {
        let elements = vec![section("s1", false), section("s2", true), section("s3", false)];
        let (journal, rapport) = jouer(elements, json!({}));
        let e = rapport.unwrap_err();
        assert!(e.contains("chaque") && e.contains("refus voulu sur s2"), "{e}");
        assert!(!journal.iter().any(|l| l == "écrit s3"), "rien après l'échec : {journal:?}");
    }

    /// **En report, l'échec se dit et la suite continue** : {index, error, element}
    /// dans le flux, et le rapport compte l'échec.
    #[test]
    fn en_report_l_echec_se_dit_dans_le_flux_et_le_rapport_et_la_suite_continue() {
        let elements = vec![section("s1", false), section("s2", true), section("s3", false)];
        let (journal, rapport) = jouer(elements, json!({"on_error": "report"}));
        let rapport = rapport.unwrap();
        assert_eq!(rapport["elements"], 3);
        assert_eq!(rapport["ok"], 2);
        assert_eq!(rapport["errors"][0]["index"], 1);
        assert!(journal.iter().any(|l| l == "écrit s3"), "la suite continue : {journal:?}");
        assert!(journal.iter().any(|l| l.starts_with("reçu {") && l.contains("refus voulu sur s2") && l.contains("\"index\":1")), "l'échec dans le flux : {journal:?}");
    }

    /// **En parallèle, l'ordre des résultats reste celui des éléments.**
    #[test]
    fn en_parallele_les_resultats_gardent_l_ordre_des_elements() {
        let (journal, rapport) = jouer((1..=5).map(|i| section(&format!("s{i}"), false)).collect(), json!({"parallelism": 2}));
        assert_eq!(rapport.unwrap()["ok"], 5);
        let recus: Vec<&String> = journal.iter().filter(|l| l.starts_with("reçu")).collect();
        assert_eq!(recus, ["reçu rendu s1", "reçu rendu s2", "reçu rendu s3", "reçu rendu s4", "reçu rendu s5"], "{journal:?}");
    }

    /// **Un champ absent de l'élément refuse l'appel en le nommant** :
    /// `$element.title` sur un élément sans `title` ne devient pas une chaîne vide.
    #[test]
    fn un_champ_absent_de_l_element_refuse_l_appel_en_le_nommant() {
        let (journal, rapport) = jouer(vec![json!({"titre": "s1", "echoue": false})], json!({}));
        let e = rapport.unwrap_err();
        assert!(e.contains("element.title"), "{e}");
        assert!(!journal.iter().any(|l| l.starts_with("écrit")), "{journal:?}");
    }

    /// **Le chargeur refuse en nommant le fichier** un graphe cité absent, un
    /// graphe sans le paramètre de l'élément, et un `$element` dans un script.
    #[test]
    fn le_chargeur_refuse_un_sous_graphe_absent_ou_faux_en_nommant_le_fichier() {
        let reg = registre();
        let outil = |config: &str| {
            GraphTool::from_mermaid(&format!(
                "%% tool: archive\n%% description: d\n%% result: chaque.report\n\ngraph LR\n    chaque[\"ForEachNode({config})\"]\n"
            ))
            .unwrap()
        };
        let lire = |chemin: &str| -> Result<String, String> {
            match chemin {
                "graphs/section.mmd" => Ok(SECTION.to_string()),
                "graphs/sans_element.mmd" => Ok(SECTION.replace("%% param: element json! -- la section\n", "%% param: autre json! -- x\n").replace("$element", "$autre")),
                "graphs/script.mmd" => Ok("%% tool: s\n%% description: d\n%% param: element json! -- x\n%% result: r.result\n\ngraph LR\n    r[\"RhaiNode(script='let t = \\\"$element\\\"; t')\"]\n".into()),
                autre => Err(format!("fichier introuvable : {autre}")),
            }
        };
        let mut bon = outil("graph='@graphs/section.mmd'");
        let types = resoudre_les_sous_graphes(&mut bon, &lire, &reg).unwrap();
        assert_eq!(types, ["NoterNode"], "les types du sous-graphe, pour la politique");
        let e = resoudre_les_sous_graphes(&mut outil("graph='@graphs/absent.mmd'"), &lire, &reg).unwrap_err();
        assert!(e.contains("graphs/absent.mmd") && e.contains("chaque"), "{e}");
        let e = resoudre_les_sous_graphes(&mut outil("graph='@graphs/sans_element.mmd'"), &lire, &reg).unwrap_err();
        assert!(e.contains("graphs/sans_element.mmd") && e.contains("« element »"), "{e}");
        let e = resoudre_les_sous_graphes(&mut outil("graph='@graphs/script.mmd'"), &lire, &reg).unwrap_err();
        assert!(e.contains("graphs/script.mmd") && e.contains("script"), "{e}");
    }

    /// **Un manifeste qui cite un sous-graphe absent ou faux se refuse à son
    /// chargement**, en nommant l'outil et le fichier (`PreparedBackend::load`,
    /// avant toute base).
    #[test]
    fn un_manifeste_qui_cite_un_sous_graphe_absent_ou_faux_se_refuse_au_chargement() {
        let charger = |sous: Option<&str>| -> String {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(dir.path().join("graphs")).unwrap();
            if let Some(texte) = sous {
                std::fs::write(dir.path().join("graphs/section.mmd"), texte).unwrap();
            }
            std::fs::write(
                dir.path().join("archive.mmd"),
                "%% tool: archive\n%% description: d\n%% result: chaque.report\n\ngraph LR\n    chaque[\"ForEachNode(graph='@graphs/section.mmd')\"]\n",
            )
            .unwrap();
            let m = json!({
                "version": 1, "name": "foreach", "database": "base.rag3db",
                "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
                "entities": {},
                "tools": {"archive": {"graph": "archive.mmd"}}
            });
            let chemin = dir.path().join("backend.json");
            std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
            match crate::backend::PreparedBackend::load(&chemin) {
                Err(e) => e,
                Ok(_) => "chargé".into(),
            }
        };
        let e = charger(None);
        assert!(e.contains("archive") && e.contains("graphs/section.mmd"), "absent : {e}");
        let e = charger(Some("pas un graphe"));
        assert!(e.contains("archive") && e.contains("graphs/section.mmd"), "faux : {e}");
    }
}
