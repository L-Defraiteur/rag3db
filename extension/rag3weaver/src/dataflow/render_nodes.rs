//! Rendre des résultats **pour un modèle**, pas pour un programme.
//!
//! Le JSON brut d'une recherche coûte cher : `uuid`, `score` en flottant
//! long, `_content_hash`, le `content` entier de chaque résultat, et — pour
//! un voisin de graphe — **toutes** les colonnes de la table, nulles
//! comprises. Mesuré le 26 août : 370 000 jetons pour trois questions, dont
//! l'essentiel en champs que le modèle ne lit jamais
//! ([doc 11](../../docs/25-aout-2026-18h58/11-gemini-fiches-bornees-mesure.md)).
//!
//! `read` et `grep` rendent du markdown compact depuis le début ; ce nœud
//! fait la même chose pour les résultats. Il est **passe-plat** : le port
//! `results` ressort tel quel, pour qu'un graphe continue à composer (c'est
//! ce qui permet à `search_expand` de contenir `search`), et `text` porte la
//! version lisible.

use crate::connection::CypherValue;
use crate::search_strategy::UnifiedResult;

use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue};

/// Longueur d'un extrait, en caractères.
const DEFAULT_MAX_CHARS: usize = 300;
/// Longueur d'une valeur de champ, en caractères.
const FIELD_CHARS: usize = 120;

/// Une valeur de colonne, si elle vaut la peine d'être montrée.
///
/// `Null` disparaît — c'est la moitié du poids d'un voisin de graphe. Les
/// listes et les cartes aussi : personne ne lit un vecteur d'embedding.
fn scalar(v: &CypherValue) -> Option<String> {
    match v {
        CypherValue::String(s) if s.is_empty() => None,
        CypherValue::String(s) => Some(ellipsize(s, FIELD_CHARS)),
        CypherValue::Int(i) => Some(i.to_string()),
        CypherValue::Float(f) => Some(format!("{f:.4}")),
        CypherValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn ellipsize(s: &str, max: usize) -> String {
    let clean = s.replace(['\n', '\r'], " ");
    let clean = clean.trim();
    if clean.chars().count() <= max {
        return clean.to_string();
    }
    let cut: String = clean.chars().take(max).collect();
    format!("{cut}…")
}

/// Les champs internes du moteur — jamais pour le modèle.
fn is_internal(key: &str) -> bool {
    key.starts_with('_')
}

/// Les champs que le rendu consomme lui-même : ils deviennent le lien, le
/// titre hiérarchique ou l'en-tête de groupe, et ne sont donc pas répétés
/// dans la liste des champs.
const CONSUMED: [&str; 6] = ["file_path", "path", "start_line", "end_line", "parent_name", "language"];

type Data = std::collections::BTreeMap<String, CypherValue>;

fn text_field(data: Option<&Data>, key: &str) -> Option<String> {
    match data?.get(key) {
        Some(CypherValue::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

fn int_field(data: Option<&Data>, key: &str) -> Option<i64> {
    match data?.get(key) {
        Some(CypherValue::Int(i)) => Some(*i),
        _ => None,
    }
}

/// `port.rs:101-140` — de quoi lancer `read(path, offset)` sans réfléchir.
/// C'est la forme que tout le monde sait lire, et la seule que le modèle
/// peut réutiliser telle quelle.
fn location(data: Option<&Data>) -> Option<String> {
    let path = text_field(data, "file_path").or_else(|| text_field(data, "path"))?;
    match (int_field(data, "start_line"), int_field(data, "end_line")) {
        (Some(a), Some(b)) if b > a => Some(format!("{path}:{a}-{b}")),
        (Some(a), _) => Some(format!("{path}:{a}")),
        _ => Some(path),
    }
}

/// Le séparateur de portée de la langue : `Classe.methode` en Python et en
/// JavaScript, `Classe::methode` ailleurs. Détail, mais c'est ce qu'un
/// humain écrirait, donc ce qu'un modèle reconnaît.
fn scope_sep(data: Option<&Data>) -> &'static str {
    match text_field(data, "language").as_deref() {
        Some("python" | "javascript" | "typescript" | "ruby" | "java" | "csharp" | "go") => ".",
        _ => "::",
    }
}

/// Le nom d'un résultat : ce qu'un humain citerait.
fn title_of(data: Option<&std::collections::BTreeMap<String, CypherValue>>, uuid: &str) -> String {
    let Some(data) = data else { return uuid.chars().take(8).collect() };
    for key in ["_title", "name", "title", "path", "file_path", "summary", "content"] {
        if let Some(CypherValue::String(s)) = data.get(key) {
            if !s.is_empty() {
                return ellipsize(s, 80);
            }
        }
    }
    uuid.chars().take(8).collect()
}

/// Les champs à montrer, dans l'ordre du schéma, sans le titre ni les
/// internes ni les vides.
fn fields_of(
    data: Option<&std::collections::BTreeMap<String, CypherValue>>,
    title: &str,
) -> Vec<String> {
    let Some(data) = data else { return Vec::new() };
    data.iter()
        .filter(|(k, _)| !is_internal(k) && !CONSUMED.contains(&k.as_str()))
        .filter_map(|(k, v)| scalar(v).map(|s| (k, s)))
        .filter(|(_, s)| s != title)
        .map(|(k, s)| format!("{k}={s}"))
        .collect()
}

/// La clé de regroupement : le parent, dans son fichier. Vide = pas de
/// groupe.
fn group_key(r: &UnifiedResult) -> Option<(String, String)> {
    let parent = text_field(r.data.as_ref(), "parent_name")?;
    let file = text_field(r.data.as_ref(), "file_path")
        .or_else(|| text_field(r.data.as_ref(), "path"))
        .unwrap_or_default();
    Some((file, parent))
}

/// Le rendu markdown d'une liste de résultats — la surface que le modèle lit.
pub fn render_results_markdown(results: &[UnifiedResult], max_chars: usize) -> String {
    render_results_with(results, max_chars, true)
}

/// La même, en choisissant de regrouper ou non les résultats qui partagent
/// une classe (ou une fonction englobante) : trois méthodes d'une même
/// classe deviennent un bloc, au lieu de trois entrées qui répètent le
/// contexte. Le regroupement **réordonne** — les groupes sortent dans
/// l'ordre de leur meilleur score, la numérotation reste globale, et un
/// résultat seul dans son groupe n'a pas d'en-tête.
pub fn render_results_with(results: &[UnifiedResult], max_chars: usize, group: bool) -> String {
    if results.is_empty() {
        return "**No results.**".to_string();
    }
    // Ordre de sortie : soit tel quel, soit par groupes.
    let mut order: Vec<usize> = (0..results.len()).collect();
    let mut header_at: std::collections::HashMap<usize, (String, usize)> = std::collections::HashMap::new();
    if group {
        // Un groupe par (fichier, parent) ; sans parent, chacun le sien.
        let mut groups: Vec<(Option<(String, String)>, Vec<usize>)> = Vec::new();
        for (i, r) in results.iter().enumerate() {
            let key = group_key(r);
            match key.as_ref().and_then(|k| groups.iter_mut().find(|(g, _)| g.as_ref() == Some(k))) {
                Some((_, members)) => members.push(i),
                None => groups.push((key, vec![i])),
            }
        }
        order.clear();
        for (key, members) in groups {
            if let (Some((file, parent)), true) = (key, members.len() > 1) {
                let where_ = if file.is_empty() { String::new() } else { format!(" · {file}") };
                header_at.insert(members[0], (format!("`{parent}`{where_}"), members.len()));
            }
            order.extend(members);
        }
    }

    let mut out = format!("**{} result{}**\n", results.len(), if results.len() == 1 { "" } else { "s" });
    for (rank, &i) in order.iter().enumerate() {
        let r = &results[i];
        if let Some((header, n)) = header_at.get(&i) {
            out.push_str(&format!("\n{header} — {n} matches\n"));
        }
        let name = title_of(r.data.as_ref(), &r.uuid);
        // Hiérarchie : `Classe::methode`, quand le parent est connu.
        let title = match text_field(r.data.as_ref(), "parent_name") {
            Some(parent) if parent != name => format!("{parent}{}{name}", scope_sep(r.data.as_ref())),
            _ => name.clone(),
        };
        let entity = r.entity.as_deref().unwrap_or("?");
        out.push_str(&format!("\n{}. `{title}` — {entity} · {:.2}", rank + 1, r.score));
        if let Some(loc) = location(r.data.as_ref()) {
            out.push_str(&format!(" · {loc}"));
        }
        if let Some(rel) = &r.relation {
            out.push_str(&format!(" · via {rel}"));
        }
        if let Some(sig) = &r.signal {
            out.push_str(&format!(" · {sig}"));
        }
        out.push('\n');

        let fields = fields_of(r.data.as_ref(), &name);
        if !fields.is_empty() {
            out.push_str(&format!("   {}\n", fields.join(" · ")));
        }
        if let Some(chunk) = &r.chunk {
            let text = ellipsize(&chunk.text, max_chars);
            if !text.is_empty() && text != name {
                out.push_str(&format!("   > {text}\n"));
            }
        }
        if let Some(chunks) = &r.chunks {
            for c in chunks.iter().take(3) {
                let text = ellipsize(&c.text, max_chars);
                if !text.is_empty() {
                    out.push_str(&format!("   > {text}\n"));
                }
            }
            if chunks.len() > 3 {
                out.push_str(&format!("   > … {} more chunks\n", chunks.len() - 3));
            }
        }
        for child in r.other_children.iter().flatten() {
            let child_title = title_of(Some(&child.data), &child.uuid);
            let fields = fields_of(Some(&child.data), &child_title);
            let extra = if fields.is_empty() { String::new() } else { format!(" ({})", fields.join(" · ")) };
            // Le voisin porte son lien aussi : un `DEFINED_IN` devient un
            // chemin qu'on peut lire.
            let loc = location(Some(&child.data)).map(|l| format!(" {l}")).unwrap_or_default();
            out.push_str(&format!("   ↳ {} {} `{child_title}`{loc}{extra}\n", child.relation, child.entity));
        }
        for child in r.matched_children.iter().flatten() {
            let child_title = title_of(child.data.as_ref(), &child.uuid);
            out.push_str(&format!("   ↳ {} `{child_title}` · {:.2}\n", child.entity.as_deref().unwrap_or("?"), child.score));
        }
    }
    out
}

// ─── RenderResultsNode ──────────────────────────────────────────────────────

/// Rend `results` en markdown compact sur `text`, et **laisse passer** les
/// résultats sur `results` — un graphe qui contient celui-ci continue à
/// composer.
pub struct RenderResultsNode {
    node_name: String,
    json: bool,
    max_chars: usize,
    group: bool,
}

impl RenderResultsNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string(), json: false, max_chars: DEFAULT_MAX_CHARS, group: true }
    }
    /// `true` : le JSON brut, pour un appelant qui est un programme.
    pub fn with_json(mut self, json: bool) -> Self {
        self.json = json;
        self
    }
    pub fn with_max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars;
        self
    }
    /// Regrouper les résultats d'une même classe (défaut : oui).
    pub fn with_group(mut self, group: bool) -> Self {
        self.group = group;
        self
    }
}

impl Node for RenderResultsNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "RenderResultsNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "format": if self.json { "json" } else { "markdown" },
            "max_chars": self.max_chars,
            "group": self.group,
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "results", port_type: PortType::Results, required: false }]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![
            PortDef { name: "text", port_type: PortType::Text, required: false },
            PortDef { name: "results", port_type: PortType::Results, required: false },
        ]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let results: Vec<UnifiedResult> = ctx
            .take_input("results")
            .and_then(take_or_clone::<Vec<UnifiedResult>>)
            .unwrap_or_default();
        let text = if self.json {
            serde_json::to_string(&results).map_err(|e| format!("RenderResultsNode: {e}"))?
        } else {
            render_results_with(&results, self.max_chars, self.group)
        };
        ctx.set_output("text", PortValue::new(text));
        ctx.set_output("results", PortValue::new(results));
        Ok(())
    }
}

pub struct RenderResultsNodeFactory;

impl NodeFactory for RenderResultsNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let mut node = RenderResultsNode::new(name);
        match config.get("format").and_then(|v| v.as_str()) {
            None | Some("markdown") => {}
            Some("json") => node = node.with_json(true),
            Some(other) => return Err(format!("RenderResultsNode: unknown format '{other}' (markdown | json)")),
        }
        if let Some(n) = config.get("max_chars").and_then(|v| v.as_u64()) {
            node = node.with_max_chars(n as usize);
        }
        if let Some(g) = config.get("group").and_then(|v| v.as_bool()) {
            node = node.with_group(g);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "RenderResultsNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "RenderResultsNode",
            description: "Renders results as compact markdown on 'text' (nulls, internal fields and embeddings dropped) and passes them through on 'results'",
            inputs: vec![PortDef { name: "results", port_type: PortType::Results, required: false }],
            outputs: vec![
                PortDef { name: "text", port_type: PortType::Text, required: false },
                PortDef { name: "results", port_type: PortType::Results, required: false },
            ],
            config_params: vec![
                ConfigParam {
                    name: "format",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("markdown")),
                    description: "markdown (compact, for a model) | json (raw, for a program)",
                    choices: Some(Choices::fixed(["markdown", "json"])),
                    json_schema: None,
                },
                ConfigParam {
                    name: "max_chars",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(DEFAULT_MAX_CHARS)),
                    description: "Snippet length, in characters",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "group",
                    param_type: ConfigParamType::Bool,
                    required: false,
                    default: Some(serde_json::json!(true)),
                    description: "Group results that share a parent scope under one header (reorders by best score)",
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::ChunkInfo;
    use crate::search_strategy::ChildSummary;

    fn data(pairs: &[(&str, CypherValue)]) -> Data {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    fn scope(name: &str, parent: &str, start: i64, end: i64, score: f64) -> UnifiedResult {
        UnifiedResult {
            uuid: format!("uuid-of-{name}"),
            score,
            entity: Some("Scope".into()),
            data: Some(data(&[
                ("name", CypherValue::String(name.into())),
                ("parent_name", CypherValue::String(parent.into())),
                ("file_path", CypherValue::String("port.rs".into())),
                ("language", CypherValue::String("rust".into())),
                ("start_line", CypherValue::Int(start)),
                ("end_line", CypherValue::Int(end)),
                ("_content_hash", CypherValue::String("dead".into())),
                ("docstring", CypherValue::Null),
            ])),
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
            signal: None,
        }
    }

    #[test]
    fn a_result_carries_its_file_link_and_its_hierarchy() {
        let md = render_results_markdown(&[scope("take", "PortValue", 120, 140, 0.813)], 300);
        assert!(md.contains("`PortValue::take` — Scope · 0.81 · port.rs:120-140"), "{md}");
        // Les champs consommés ne sont pas répétés, les internes et les nuls
        // ont disparu.
        for absent in ["file_path=", "start_line=", "parent_name=", "_content_hash", "docstring", "uuid"] {
            assert!(!md.contains(absent), "{absent} devrait avoir disparu :\n{md}");
        }
    }

    #[test]
    fn the_separator_follows_the_language() {
        let mut r = scope("run", "Session", 1, 2, 0.5);
        r.data.as_mut().unwrap().insert("language".into(), CypherValue::String("python".into()));
        assert!(render_results_markdown(&[r], 300).contains("`Session.run`"));
    }

    #[test]
    fn results_of_one_class_are_grouped_once_and_numbered_globally() {
        let results = vec![
            scope("take", "PortValue", 120, 140, 0.81),
            scope("merge_port_values", "", 20, 50, 0.75),
            scope("downcast", "PortValue", 110, 118, 0.60),
        ];
        let md = render_results_with(&results, 300, true);
        assert_eq!(md.matches("`PortValue` · port.rs — 2 matches").count(), 1, "{md}");
        // Regroupés, donc réordonnés : les deux de la classe d'abord (leur
        // meilleur score), la numérotation reste globale et continue.
        let order: Vec<&str> = md.lines().filter(|l| l.starts_with(char::is_numeric)).collect();
        assert_eq!(order.len(), 3, "{md}");
        assert!(order[0].starts_with("1. `PortValue::take`"), "{md}");
        assert!(order[1].starts_with("2. `PortValue::downcast`"), "{md}");
        assert!(order[2].starts_with("3. `merge_port_values`"), "{md}");

        // Sans regroupement : l'ordre des scores, et aucun en-tête.
        let flat = render_results_with(&results, 300, false);
        assert!(!flat.contains("matches"), "{flat}");
        let order: Vec<&str> = flat.lines().filter(|l| l.starts_with(char::is_numeric)).collect();
        assert!(order[1].starts_with("2. `merge_port_values`"), "{flat}");
    }

    #[test]
    fn a_neighbour_carries_its_link_too() {
        let mut r = scope("take", "PortValue", 120, 140, 0.8);
        r.other_children = Some(vec![ChildSummary {
            uuid: "u".into(),
            entity: "File".into(),
            relation: "DEFINED_IN".into(),
            data: data(&[
                ("path", CypherValue::String("src/dataflow/port.rs".into())),
                ("language", CypherValue::String("rust".into())),
                ("lines_of_code", CypherValue::Int(313)),
                ("cursor", CypherValue::Null),
            ]),
        }]);
        let md = render_results_markdown(&[r], 300);
        assert!(md.contains("↳ DEFINED_IN File `src/dataflow/port.rs` src/dataflow/port.rs"), "{md}");
        assert!(md.contains("lines_of_code=313"), "{md}");
        assert!(!md.contains("cursor"), "un champ nul ne se rend pas : {md}");
    }

    #[test]
    fn a_snippet_is_bounded_and_single_line() {
        let mut r = scope("take", "PortValue", 1, 2, 0.5);
        r.chunk = Some(ChunkInfo {
            uuid: "c".into(),
            text: format!("ligne une\nligne deux{}", "x".repeat(500)),
            index: 0,
            score: 0.5,
            start_line: 1,
            end_line: 2,
            start_char: 0,
            end_char: 0,
        });
        let md = render_results_markdown(&[r], 40);
        let quoted = md.lines().find(|l| l.trim_start().starts_with("> ")).expect("un extrait");
        assert!(quoted.chars().count() <= 50, "{quoted}");
        assert!(quoted.ends_with('…'), "{quoted}");
    }

    #[test]
    fn nothing_found_says_so() {
        assert_eq!(render_results_markdown(&[], 300), "**No results.**");
    }
}
